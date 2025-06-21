//! Bounded async channels with fuel-based flow control
//!
//! This module provides async channels that integrate with the fuel system
//! for deterministic inter-task communication in safety-critical environments.

use crate::{
    async_::fuel_priority_inheritance::{FuelPriorityInheritanceProtocol, ResourceId},
    task_manager::TaskId,
    ComponentInstanceId,
    prelude::*,
};
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
    time::Duration,
};
use wrt_foundation::{
    bounded_collections::{BoundedVec, BoundedHashMap},
    operations::{record_global_operation, Type as OperationType},
    verification::VerificationLevel,
    CrateId, safe_managed_alloc,
};
use wrt_platform::advanced_sync::Priority;

#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};
#[cfg(not(feature = "std"))]
use wrt_sync::{Arc, Mutex};

/// Maximum number of async channels
const MAX_ASYNC_CHANNELS: usize = 64;

/// Maximum capacity for individual channels
const MAX_CHANNEL_CAPACITY: usize = 256;

/// Maximum number of waiters per channel
const MAX_WAITERS_PER_CHANNEL: usize = 32;

/// Fuel costs for channel operations
const CHANNEL_SEND_FUEL: u64 = 8;
const CHANNEL_RECEIVE_FUEL: u64 = 6;
const CHANNEL_CLOSE_FUEL: u64 = 5;
const CHANNEL_WAKER_FUEL: u64 = 3;

/// Unique identifier for async channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelId(pub u64);

impl ChannelId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Bounded async channel with fuel tracking
pub struct FuelAsyncChannel<T> {
    /// Unique channel identifier
    id: ChannelId,
    /// Channel capacity
    capacity: usize,
    /// Message buffer
    buffer: BoundedVec<T, MAX_CHANNEL_CAPACITY>,
    /// Senders waiting to send (when buffer is full)
    waiting_senders: BoundedVec<ChannelWaiter, MAX_WAITERS_PER_CHANNEL>,
    /// Receivers waiting to receive (when buffer is empty)
    waiting_receivers: BoundedVec<ChannelWaiter, MAX_WAITERS_PER_CHANNEL>,
    /// Whether the channel is closed
    closed: AtomicBool,
    /// Total messages sent through this channel
    messages_sent: AtomicU64,
    /// Total messages received through this channel
    messages_received: AtomicU64,
    /// Total fuel consumed by this channel
    fuel_consumed: AtomicU64,
    /// Channel verification level for fuel tracking
    verification_level: VerificationLevel,
    /// Priority inheritance protocol for blocking operations
    priority_protocol: Option<FuelPriorityInheritanceProtocol>,
}

/// Channel waiter information
#[derive(Debug, Clone)]
pub struct ChannelWaiter {
    /// Task waiting on the channel
    pub task_id: TaskId,
    /// Component owning the task
    pub component_id: ComponentInstanceId,
    /// Priority of the waiting task
    pub priority: Priority,
    /// Waker to notify when ready
    pub waker: Option<Waker>,
    /// When the wait started (fuel time)
    pub wait_start_time: u64,
    /// Maximum wait time allowed
    pub max_wait_time: Option<Duration>,
}

/// Sender half of an async channel
pub struct FuelAsyncSender<T> {
    /// Channel ID
    channel_id: ChannelId,
    /// Shared reference to the channel manager (ASIL-D safe)
    channel_manager: Arc<Mutex<FuelAsyncChannelManager<T>>>,
    /// Task ID of the sender
    sender_task: TaskId,
    /// Component ID of the sender
    sender_component: ComponentInstanceId,
    /// Priority of the sender
    sender_priority: Priority,
}

/// Receiver half of an async channel
pub struct FuelAsyncReceiver<T> {
    /// Channel ID
    channel_id: ChannelId,
    /// Shared reference to the channel manager (ASIL-D safe)
    channel_manager: Arc<Mutex<FuelAsyncChannelManager<T>>>,
    /// Task ID of the receiver
    receiver_task: TaskId,
    /// Component ID of the receiver
    receiver_component: ComponentInstanceId,
    /// Priority of the receiver
    receiver_priority: Priority,
}

/// Future for sending a message
pub struct SendFuture<T> {
    /// Message to send
    message: Option<T>,
    /// Sender information
    sender: FuelAsyncSender<T>,
    /// Whether the send is registered with the channel
    registered: bool,
}

/// Future for receiving a message
pub struct ReceiveFuture<T> {
    /// Receiver information
    receiver: FuelAsyncReceiver<T>,
    /// Whether the receive is registered with the channel
    registered: bool,
}

/// Channel manager for organizing multiple async channels
pub struct FuelAsyncChannelManager<T> {
    /// Active channels indexed by ID
    channels: BoundedHashMap<ChannelId, FuelAsyncChannel<T>, MAX_ASYNC_CHANNELS>,
    /// Global channel statistics
    global_stats: ChannelManagerStats,
    /// Next channel ID counter
    next_channel_id: AtomicU64,
    /// Global verification level
    verification_level: VerificationLevel,
}

/// Channel manager statistics
#[derive(Debug, Clone)]
pub struct ChannelManagerStats {
    /// Total channels created
    pub total_channels_created: AtomicUsize,
    /// Currently active channels
    pub active_channels: AtomicUsize,
    /// Total messages sent across all channels
    pub total_messages_sent: AtomicU64,
    /// Total messages received across all channels
    pub total_messages_received: AtomicU64,
    /// Total fuel consumed by all channels
    pub total_fuel_consumed: AtomicU64,
    /// Total blocked senders across all channels
    pub total_blocked_senders: AtomicUsize,
    /// Total blocked receivers across all channels
    pub total_blocked_receivers: AtomicUsize,
}

impl<T> FuelAsyncChannel<T> {
    /// Create a new bounded async channel
    pub fn new(
        id: ChannelId,
        capacity: usize,
        verification_level: VerificationLevel,
        enable_priority_inheritance: bool,
    ) -> Result<Self, Error> {
        if capacity > MAX_CHANNEL_CAPACITY {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                format!("Channel capacity {} exceeds maximum {}", capacity, MAX_CHANNEL_CAPACITY),
            ));
        }

        let provider = safe_managed_alloc!(4096, CrateId::Component)?;
        
        let priority_protocol = if enable_priority_inheritance {
            Some(FuelPriorityInheritanceProtocol::new(verification_level)?)
        } else {
            None
        };

        Ok(Self {
            id,
            capacity,
            buffer: BoundedVec::new(provider.clone())?,
            waiting_senders: BoundedVec::new(provider.clone())?,
            waiting_receivers: BoundedVec::new(provider)?,
            closed: AtomicBool::new(false),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            fuel_consumed: AtomicU64::new(0),
            verification_level,
            priority_protocol,
        })
    }

    /// Attempt to send a message (non-blocking)
    pub fn try_send(
        &mut self,
        message: T,
        sender_task: TaskId,
        sender_component: ComponentInstanceId,
        sender_priority: Priority,
    ) -> Result<(), ChannelError<T>> {
        if self.closed.load(Ordering::Acquire) {
            return Err(ChannelError::Closed(message));
        }

        record_global_operation(OperationType::CollectionInsert, self.verification_level);
        self.consume_fuel(CHANNEL_SEND_FUEL);

        // Try to deliver to waiting receiver first
        if let Some(waiter) = self.waiting_receivers.pop() {
            // Wake up the receiver
            if let Some(waker) = waiter.waker {
                waker.wake();
                self.consume_fuel(CHANNEL_WAKER_FUEL);
            }
            
            // Message is immediately consumed by waiting receiver
            self.messages_sent.fetch_add(1, Ordering::AcqRel);
            self.messages_received.fetch_add(1, Ordering::AcqRel);
            return Ok(());
        }

        // Try to add to buffer
        if self.buffer.len() < self.capacity {
            self.buffer.push(message).map_err(|msg| ChannelError::BufferFull(msg))?;
            self.messages_sent.fetch_add(1, Ordering::AcqRel);
            Ok(())
        } else {
            // Buffer is full, would need to wait
            Err(ChannelError::WouldBlock(message))
        }
    }

    /// Attempt to receive a message (non-blocking)
    pub fn try_receive(
        &mut self,
        receiver_task: TaskId,
        receiver_component: ComponentInstanceId,
        receiver_priority: Priority,
    ) -> Result<T, ChannelError<()>> {
        record_global_operation(OperationType::CollectionRemove, self.verification_level);
        self.consume_fuel(CHANNEL_RECEIVE_FUEL);

        // Try to get message from buffer
        if let Some(message) = self.buffer.pop() {
            self.messages_received.fetch_add(1, Ordering::AcqRel);
            
            // Wake up waiting sender if any
            if let Some(waiter) = self.waiting_senders.pop() {
                if let Some(waker) = waiter.waker {
                    waker.wake();
                    self.consume_fuel(CHANNEL_WAKER_FUEL);
                }
            }
            
            return Ok(message);
        }

        // No message available
        if self.closed.load(Ordering::Acquire) {
            Err(ChannelError::Closed(()))
        } else {
            Err(ChannelError::WouldBlock(()))
        }
    }

    /// Register a sender to wait for space
    pub fn register_sender_waiter(
        &mut self,
        task_id: TaskId,
        component_id: ComponentInstanceId,
        priority: Priority,
        waker: Option<Waker>,
        max_wait_time: Option<Duration>,
    ) -> Result<(), Error> {
        let waiter = ChannelWaiter {
            task_id,
            component_id,
            priority,
            waker,
            wait_start_time: self.get_current_fuel_time(),
            max_wait_time,
        };

        self.waiting_senders.push(waiter).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many waiting senders".to_string(),
            )
        })?;

        // Register priority inheritance if enabled
        if let Some(priority_protocol) = &mut self.priority_protocol {
            let resource_id = ResourceId::new(self.id.0);
            priority_protocol.register_blocking(
                task_id,
                priority,
                resource_id,
                None, // No specific holder for channel buffer
                max_wait_time,
            )?;
        }

        Ok(())
    }

    /// Register a receiver to wait for messages
    pub fn register_receiver_waiter(
        &mut self,
        task_id: TaskId,
        component_id: ComponentInstanceId,
        priority: Priority,
        waker: Option<Waker>,
        max_wait_time: Option<Duration>,
    ) -> Result<(), Error> {
        let waiter = ChannelWaiter {
            task_id,
            component_id,
            priority,
            waker,
            wait_start_time: self.get_current_fuel_time(),
            max_wait_time,
        };

        self.waiting_receivers.push(waiter).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many waiting receivers".to_string(),
            )
        })?;

        // Register priority inheritance if enabled
        if let Some(priority_protocol) = &mut self.priority_protocol {
            let resource_id = ResourceId::new(self.id.0 + 1000000); // Offset for receiver resources
            priority_protocol.register_blocking(
                task_id,
                priority,
                resource_id,
                None,
                max_wait_time,
            )?;
        }

        Ok(())
    }

    /// Close the channel
    pub fn close(&mut self) {
        record_global_operation(OperationType::CollectionMutate, self.verification_level);
        self.consume_fuel(CHANNEL_CLOSE_FUEL);
        
        self.closed.store(true, Ordering::Release);
        
        // Wake up all waiting senders and receivers
        while let Some(waiter) = self.waiting_senders.pop() {
            if let Some(waker) = waiter.waker {
                waker.wake();
                self.consume_fuel(CHANNEL_WAKER_FUEL);
            }
        }
        
        while let Some(waiter) = self.waiting_receivers.pop() {
            if let Some(waker) = waiter.waker {
                waker.wake();
                self.consume_fuel(CHANNEL_WAKER_FUEL);
            }
        }
    }

    /// Get channel statistics
    pub fn get_stats(&self) -> ChannelStats {
        ChannelStats {
            id: self.id,
            capacity: self.capacity,
            current_size: self.buffer.len(),
            messages_sent: self.messages_sent.load(Ordering::Acquire),
            messages_received: self.messages_received.load(Ordering::Acquire),
            fuel_consumed: self.fuel_consumed.load(Ordering::Acquire),
            waiting_senders: self.waiting_senders.len(),
            waiting_receivers: self.waiting_receivers.len(),
            closed: self.closed.load(Ordering::Acquire),
        }
    }

    // Private helper methods

    fn consume_fuel(&self, amount: u64) {
        self.fuel_consumed.fetch_add(amount, Ordering::AcqRel);
    }

    fn get_current_fuel_time(&self) -> u64 {
        // In real implementation, this would get current fuel time from system
        self.fuel_consumed.load(Ordering::Acquire)
    }
}

/// Channel statistics
#[derive(Debug, Clone)]
pub struct ChannelStats {
    pub id: ChannelId,
    pub capacity: usize,
    pub current_size: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub fuel_consumed: u64,
    pub waiting_senders: usize,
    pub waiting_receivers: usize,
    pub closed: bool,
}

/// Channel operation errors
#[derive(Debug)]
pub enum ChannelError<T> {
    /// Channel is closed
    Closed(T),
    /// Operation would block
    WouldBlock(T),
    /// Buffer is full
    BufferFull(T),
    /// No message available
    Empty,
    /// Timeout occurred
    Timeout,
    /// Internal error (for ASIL-D safety)
    InternalError,
}

impl<T> FuelAsyncChannelManager<T> {
    /// Create a new channel manager
    pub fn new(verification_level: VerificationLevel) -> Self {
        Self {
            channels: BoundedHashMap::new(),
            global_stats: ChannelManagerStats {
                total_channels_created: AtomicUsize::new(0),
                active_channels: AtomicUsize::new(0),
                total_messages_sent: AtomicU64::new(0),
                total_messages_received: AtomicU64::new(0),
                total_fuel_consumed: AtomicU64::new(0),
                total_blocked_senders: AtomicUsize::new(0),
                total_blocked_receivers: AtomicUsize::new(0),
            },
            next_channel_id: AtomicU64::new(1),
            verification_level,
        }
    }

    /// Create a new bounded async channel pair
    pub fn create_channel(
        &mut self,
        capacity: usize,
        enable_priority_inheritance: bool,
        sender_task: TaskId,
        sender_component: ComponentInstanceId,
        sender_priority: Priority,
        receiver_task: TaskId,
        receiver_component: ComponentInstanceId,
        receiver_priority: Priority,
    ) -> Result<(FuelAsyncSender<T>, FuelAsyncReceiver<T>), Error> {
        let channel_id = ChannelId::new(self.next_channel_id.fetch_add(1, Ordering::AcqRel));
        
        let channel = FuelAsyncChannel::new(
            channel_id,
            capacity,
            self.verification_level,
            enable_priority_inheritance,
        )?;

        self.channels.insert(channel_id, channel).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many active channels".to_string(),
            )
        })?;

        self.global_stats.total_channels_created.fetch_add(1, Ordering::AcqRel);
        self.global_stats.active_channels.fetch_add(1, Ordering::AcqRel);

        let sender = FuelAsyncSender {
            channel_id,
            channel_manager: self as *mut Self,
            sender_task,
            sender_component,
            sender_priority,
        };

        let receiver = FuelAsyncReceiver {
            channel_id,
            channel_manager: self as *mut Self,
            receiver_task,
            receiver_component,
            receiver_priority,
        };

        Ok((sender, receiver))
    }

    /// Close a channel
    pub fn close_channel(&mut self, channel_id: ChannelId) -> Result<(), Error> {
        if let Some(channel) = self.channels.get_mut(&channel_id) {
            channel.close();
            Ok(())
        } else {
            Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Channel not found".to_string(),
            ))
        }
    }

    /// Get global channel manager statistics
    pub fn get_global_stats(&self) -> ChannelManagerStats {
        // Update statistics from individual channels
        let mut total_sent = 0;
        let mut total_received = 0;
        let mut total_fuel = 0;
        let mut total_blocked_senders = 0;
        let mut total_blocked_receivers = 0;

        for channel in self.channels.values() {
            total_sent += channel.messages_sent.load(Ordering::Acquire);
            total_received += channel.messages_received.load(Ordering::Acquire);
            total_fuel += channel.fuel_consumed.load(Ordering::Acquire);
            total_blocked_senders += channel.waiting_senders.len();
            total_blocked_receivers += channel.waiting_receivers.len();
        }

        let mut stats = self.global_stats.clone();
        stats.total_messages_sent.store(total_sent, Ordering::Release);
        stats.total_messages_received.store(total_received, Ordering::Release);
        stats.total_fuel_consumed.store(total_fuel, Ordering::Release);
        stats.total_blocked_senders.store(total_blocked_senders, Ordering::Release);
        stats.total_blocked_receivers.store(total_blocked_receivers, Ordering::Release);

        stats
    }
}

impl<T> Future for SendFuture<T> {
    type Output = Result<(), ChannelError<T>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // ASIL-D safe: Use mutex lock instead of unsafe pointer dereferencing
        let mut manager = match self.sender.channel_manager.lock() {
            Ok(guard) => guard,
            Err(_) => {
                // ASIL-D safe: Proper error handling without unwrap
                if let Some(msg) = self.message.take() {
                    return Poll::Ready(Err(ChannelError::Closed(msg)));
                } else {
                    return Poll::Ready(Err(ChannelError::InternalError));
                }
            },
        };
        let channel = match manager.channels.get_mut(&self.sender.channel_id) {
            Some(ch) => ch,
            None => {
                // ASIL-D safe: Proper error handling without unwrap
                if let Some(msg) = self.message.take() {
                    return Poll::Ready(Err(ChannelError::Closed(msg)));
                } else {
                    return Poll::Ready(Err(ChannelError::InternalError));
                }
            },
        };

        if let Some(message) = self.message.take() {
            match channel.try_send(
                message,
                self.sender.sender_task,
                self.sender.sender_component,
                self.sender.sender_priority,
            ) {
                Ok(()) => Poll::Ready(Ok(())),
                Err(ChannelError::WouldBlock(msg)) => {
                    // Register to wait for space
                    if !self.registered {
                        if let Ok(()) = channel.register_sender_waiter(
                            self.sender.sender_task,
                            self.sender.sender_component,
                            self.sender.sender_priority,
                            Some(cx.waker().clone()),
                            None, // No timeout for now
                        ) {
                            self.registered = true;
                        }
                    }
                    self.message = Some(msg);
                    Poll::Pending
                }
                Err(other_error) => Poll::Ready(Err(other_error)),
            }
        } else {
            Poll::Ready(Err(ChannelError::Empty))
        }
    }
}

impl<T> Future for ReceiveFuture<T> {
    type Output = Result<T, ChannelError<()>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // ASIL-D safe: Use mutex lock instead of unsafe pointer dereferencing
        let mut manager = match self.receiver.channel_manager.lock() {
            Ok(guard) => guard,
            Err(_) => return Poll::Ready(Err(ChannelError::Closed(()))),
        };
        let channel = match manager.channels.get_mut(&self.receiver.channel_id) {
            Some(ch) => ch,
            None => return Poll::Ready(Err(ChannelError::Closed(()))),
        };

        match channel.try_receive(
            self.receiver.receiver_task,
            self.receiver.receiver_component,
            self.receiver.receiver_priority,
        ) {
            Ok(message) => Poll::Ready(Ok(message)),
            Err(ChannelError::WouldBlock(())) => {
                // Register to wait for message
                if !self.registered {
                    if let Ok(()) = channel.register_receiver_waiter(
                        self.receiver.receiver_task,
                        self.receiver.receiver_component,
                        self.receiver.receiver_priority,
                        Some(cx.waker().clone()),
                        None, // No timeout for now
                    ) {
                        self.registered = true;
                    }
                }
                Poll::Pending
            }
            Err(other_error) => Poll::Ready(Err(other_error)),
        }
    }
}

impl<T> FuelAsyncSender<T> {
    /// Send a message asynchronously
    pub fn send(&self, message: T) -> SendFuture<T> {
        SendFuture {
            message: Some(message),
            sender: FuelAsyncSender {
                channel_id: self.channel_id,
                channel_manager: self.channel_manager,
                sender_task: self.sender_task,
                sender_component: self.sender_component,
                sender_priority: self.sender_priority,
            },
            registered: false,
        }
    }

    /// Try to send a message without blocking
    pub fn try_send(&self, message: T) -> Result<(), ChannelError<T>> {
        // ASIL-D safe: Use mutex lock instead of unsafe pointer dereferencing
        let mut manager = match self.channel_manager.lock() {
            Ok(guard) => guard,
            Err(_) => return Err(ChannelError::Closed(message)),
        };
        if let Some(channel) = manager.channels.get_mut(&self.channel_id) {
            channel.try_send(message, self.sender_task, self.sender_component, self.sender_priority)
        } else {
            Err(ChannelError::Closed(message))
        }
    }
}

impl<T> FuelAsyncReceiver<T> {
    /// Receive a message asynchronously
    pub fn receive(&self) -> ReceiveFuture<T> {
        ReceiveFuture {
            receiver: FuelAsyncReceiver {
                channel_id: self.channel_id,
                channel_manager: self.channel_manager,
                receiver_task: self.receiver_task,
                receiver_component: self.receiver_component,
                receiver_priority: self.receiver_priority,
            },
            registered: false,
        }
    }

    /// Try to receive a message without blocking
    pub fn try_receive(&self) -> Result<T, ChannelError<()>> {
        // ASIL-D safe: Use mutex lock instead of unsafe pointer dereferencing
        let mut manager = match self.channel_manager.lock() {
            Ok(guard) => guard,
            Err(_) => return Err(ChannelError::Closed(())),
        };
        if let Some(channel) = manager.channels.get_mut(&self.channel_id) {
            channel.try_receive(self.receiver_task, self.receiver_component, self.receiver_priority)
        } else {
            Err(ChannelError::Closed(()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let mut manager = FuelAsyncChannelManager::<u32>::new(VerificationLevel::Standard);
        
        let result = manager.create_channel(
            10, // capacity
            false, // no priority inheritance
            TaskId::new(1), ComponentInstanceId::new(1), Priority::Normal,
            TaskId::new(2), ComponentInstanceId::new(1), Priority::Normal,
        );
        
        assert!(result.is_ok());
        
        let stats = manager.get_global_stats();
        assert_eq!(stats.total_channels_created.load(Ordering::Acquire), 1);
        assert_eq!(stats.active_channels.load(Ordering::Acquire), 1);
    }

    #[test]
    fn test_try_send_receive() {
        let mut manager = FuelAsyncChannelManager::<u32>::new(VerificationLevel::Standard);
        
        let (sender, receiver) = manager.create_channel(
            5, // capacity
            false, // no priority inheritance
            TaskId::new(1), ComponentInstanceId::new(1), Priority::Normal,
            TaskId::new(2), ComponentInstanceId::new(1), Priority::Normal,
        ).unwrap();
        
        // Send a message
        let send_result = sender.try_send(42);
        assert!(send_result.is_ok());
        
        // Receive the message
        let receive_result = receiver.try_receive();
        assert!(receive_result.is_ok());
        assert_eq!(receive_result.unwrap(), 42);
    }

    #[test]
    fn test_channel_capacity() {
        let mut manager = FuelAsyncChannelManager::<u32>::new(VerificationLevel::Standard);
        
        let (sender, _receiver) = manager.create_channel(
            2, // small capacity
            false,
            TaskId::new(1), ComponentInstanceId::new(1), Priority::Normal,
            TaskId::new(2), ComponentInstanceId::new(1), Priority::Normal,
        ).unwrap();
        
        // Fill the channel
        assert!(sender.try_send(1).is_ok());
        assert!(sender.try_send(2).is_ok());
        
        // Third send should fail with WouldBlock
        let result = sender.try_send(3);
        assert!(matches!(result, Err(ChannelError::WouldBlock(_))));
    }

    #[test]
    fn test_channel_close() {
        let mut manager = FuelAsyncChannelManager::<u32>::new(VerificationLevel::Standard);
        
        let (sender, receiver) = manager.create_channel(
            5,
            false,
            TaskId::new(1), ComponentInstanceId::new(1), Priority::Normal,
            TaskId::new(2), ComponentInstanceId::new(1), Priority::Normal,
        ).unwrap();
        
        // Close the channel
        manager.close_channel(sender.channel_id).unwrap();
        
        // Operations should now fail with Closed
        let send_result = sender.try_send(42);
        assert!(matches!(send_result, Err(ChannelError::Closed(_))));
        
        let receive_result = receiver.try_receive();
        assert!(matches!(receive_result, Err(ChannelError::Closed(_))));
    }
}