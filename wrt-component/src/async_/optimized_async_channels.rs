//! Optimized async channels for Component Model communication
//!
//! This module provides high-performance async channels with fuel tracking,
//! backpressure handling, and Component Model integration.

use core::{
    future::Future as CoreFuture,
    pin::Pin,
    sync::atomic::{
        AtomicBool,
        AtomicU32,
        AtomicU64,
        AtomicUsize,
        Ordering,
    },
    task::{
        Context,
        Poll,
        Waker,
    },
};

use wrt_foundation::{
    bounded_collections::{
        BoundedMap,
        BoundedVec,
    },
    component_value::ComponentValue,
    safe_managed_alloc,
    sync::Mutex,
    Arc,
    CrateId,
    Weak,
};
use wrt_platform::advanced_sync::Priority;

use crate::{
    async_::{
        fuel_async_executor::AsyncTaskState,
        fuel_aware_waker::{
            create_fuel_aware_waker,
            WakeCoalescer,
        },
        task_manager_async_bridge::{
            ComponentAsyncTaskType,
            TaskManagerAsyncBridge,
        },
    },
    prelude::*,
    task_manager::TaskId,
    ComponentInstanceId,
};

/// Maximum channel capacity
const MAX_CHANNEL_CAPACITY: usize = 1024;

/// Maximum channels per component
const MAX_CHANNELS_PER_COMPONENT: usize = 64;

/// Fuel costs for channel operations
const CHANNEL_SEND_FUEL: u64 = 15;
const CHANNEL_RECV_FUEL: u64 = 10;
const CHANNEL_CREATE_FUEL: u64 = 50;
const CHANNEL_CLOSE_FUEL: u64 = 20;

/// Optimized async channels manager
pub struct OptimizedAsyncChannels {
    /// Bridge for task management
    bridge:             Arc<Mutex<TaskManagerAsyncBridge>>,
    /// Active channels
    channels:           BoundedMap<ChannelId, AsyncChannel, 512>,
    /// Component channel contexts
    component_contexts: BoundedMap<ComponentInstanceId, ComponentChannelContext, 128>,
    /// Next channel ID
    next_channel_id:    AtomicU64,
    /// Channel statistics
    channel_stats:      ChannelStatistics,
    /// Global channel configuration
    global_config:      ChannelConfiguration,
}

/// Channel identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelId(u64);

/// Sender half of a channel
#[derive(Debug, Clone)]
pub struct ChannelSender {
    channel_id:   ChannelId,
    component_id: ComponentInstanceId,
    channels_ref: Weak<Mutex<OptimizedAsyncChannels>>,
}

/// Receiver half of a channel
#[derive(Debug, Clone)]
pub struct ChannelReceiver {
    channel_id:   ChannelId,
    component_id: ComponentInstanceId,
    channels_ref: Weak<Mutex<OptimizedAsyncChannels>>,
}

/// Async channel implementation
#[derive(Debug)]
struct AsyncChannel {
    id:              ChannelId,
    channel_type:    ChannelType,
    capacity:        usize,
    buffer:          ChannelBuffer,
    sender_wakers:   BoundedVec<Waker, 32>,
    receiver_wakers: BoundedVec<Waker, 32>,
    closed:          AtomicBool,
    sender_count:    AtomicU32,
    receiver_count:  AtomicU32,
    total_sent:      AtomicU64,
    total_received:  AtomicU64,
    fuel_consumed:   AtomicU64,
    created_at:      u64,
}

/// Type of async channel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    /// Unbounded channel (limited by global config)
    Unbounded,
    /// Bounded channel with fixed capacity
    Bounded(usize),
    /// Single-shot channel (oneshot)
    Oneshot,
    /// Broadcast channel (multiple receivers)
    Broadcast(usize),
    /// Priority channel (ordered by priority)
    Priority,
}

/// Channel buffer implementation
#[derive(Debug)]
enum ChannelBuffer {
    /// Ring buffer for bounded channels
    Ring {
        data: BoundedVec<ChannelMessage, MAX_CHANNEL_CAPACITY>,
        head: AtomicUsize,
        tail: AtomicUsize,
        len:  AtomicUsize,
    },
    /// Vector buffer for unbounded channels
    Vector {
        data: BoundedVec<ChannelMessage, MAX_CHANNEL_CAPACITY>,
    },
    /// Single slot for oneshot channels
    Single {
        data:  Option<ChannelMessage>,
        taken: AtomicBool,
    },
    /// Priority queue for priority channels
    Priority {
        data: BoundedVec<PriorityMessage, MAX_CHANNEL_CAPACITY>,
    },
}

/// Message in a channel
#[derive(Debug, Clone)]
struct ChannelMessage {
    value:     ComponentValue,
    sender_id: ComponentInstanceId,
    sent_at:   u64,
    priority:  u8,
}

/// Priority message for priority channels
#[derive(Debug, Clone)]
struct PriorityMessage {
    message:  ChannelMessage,
    priority: u8,
}

impl Ord for PriorityMessage {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for PriorityMessage {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PriorityMessage {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for PriorityMessage {}

/// Component channel context
#[derive(Debug)]
struct ComponentChannelContext {
    component_id:   ComponentInstanceId,
    /// Channels owned by this component
    owned_channels: BoundedVec<ChannelId, MAX_CHANNELS_PER_COMPONENT>,
    /// Senders held by this component
    senders:        BoundedMap<ChannelId, ChannelSender, MAX_CHANNELS_PER_COMPONENT>,
    /// Receivers held by this component
    receivers:      BoundedMap<ChannelId, ChannelReceiver, MAX_CHANNELS_PER_COMPONENT>,
    /// Channel quotas and limits
    channel_limits: ChannelLimits,
}

/// Channel limits per component
#[derive(Debug, Clone)]
struct ChannelLimits {
    max_channels:       usize,
    max_total_capacity: usize,
    max_message_size:   usize,
    fuel_budget:        u64,
}

/// Channel configuration
#[derive(Debug, Clone)]
pub struct ChannelConfiguration {
    pub default_capacity:         usize,
    pub max_unbounded_size:       usize,
    pub enable_backpressure:      bool,
    pub enable_priority_channels: bool,
    pub wake_coalescing:          bool,
    pub fuel_tracking:            bool,
}

impl Default for ChannelConfiguration {
    fn default() -> Self {
        Self {
            default_capacity:         32,
            max_unbounded_size:       1024,
            enable_backpressure:      true,
            enable_priority_channels: true,
            wake_coalescing:          true,
            fuel_tracking:            true,
        }
    }
}

/// Channel statistics
#[derive(Debug, Default)]
struct ChannelStatistics {
    total_channels_created:  AtomicU64,
    total_messages_sent:     AtomicU64,
    total_messages_received: AtomicU64,
    total_channel_closes:    AtomicU64,
    backpressure_events:     AtomicU64,
    wake_coalescings:        AtomicU64,
    total_fuel_consumed:     AtomicU64,
}

impl OptimizedAsyncChannels {
    /// Create new optimized async channels manager
    pub fn new(
        bridge: Arc<Mutex<TaskManagerAsyncBridge>>,
        config: Option<ChannelConfiguration>,
    ) -> Result<Self, Error> {
        let provider = safe_managed_alloc!(4096, CrateId::Component)?;
        Ok(Self {
            bridge,
            channels: BoundedMap::new(provider.clone())?,
            component_contexts: BoundedMap::new(provider.clone())?,
            next_channel_id: AtomicU64::new(1),
            channel_stats: ChannelStatistics::default(),
            global_config: config.unwrap_or_default(),
        })
    }

    /// Initialize component for channel operations
    pub fn initialize_component_channels(
        &mut self,
        component_id: ComponentInstanceId,
        limits: Option<ChannelLimits>,
    ) -> Result<(), Error> {
        let limits = limits.unwrap_or_else(|| ChannelLimits {
            max_channels:       MAX_CHANNELS_PER_COMPONENT,
            max_total_capacity: MAX_CHANNEL_CAPACITY * 4,
            max_message_size:   1024 * 1024, // 1MB
            fuel_budget:        100_000,
        });

        let provider = safe_managed_alloc!(2048, CrateId::Component)?;
        let context = ComponentChannelContext {
            component_id,
            owned_channels: BoundedVec::new(provider.clone())?,
            senders: BoundedMap::new(provider.clone())?,
            receivers: BoundedMap::new(provider.clone())?,
            channel_limits: limits,
        };

        self.component_contexts
            .insert(component_id, context)
            .map_err(|_| Error::resource_limit_exceeded("Too many component channel contexts"))?;

        Ok(())
    }

    /// Create a new channel
    pub fn create_channel(
        &mut self,
        component_id: ComponentInstanceId,
        channel_type: ChannelType,
    ) -> Result<(ChannelSender, ChannelReceiver), Error> {
        let context = self.component_contexts.get_mut(&component_id).ok_or_else(|| {
            Error::validation_invalid_input("Component not initialized for channels")
        })?;

        // Check limits
        if context.owned_channels.len() >= context.channel_limits.max_channels {
            return Err(Error::resource_limit_exceeded(
                "Component channel limit exceeded",
            ));
        }

        let channel_id = ChannelId(self.next_channel_id.fetch_add(1, Ordering::AcqRel));
        let capacity = match channel_type {
            ChannelType::Bounded(cap) => cap,
            ChannelType::Oneshot => 1,
            ChannelType::Broadcast(cap) => cap,
            _ => self.global_config.default_capacity,
        };

        // Create channel buffer
        let buffer = self.create_channel_buffer(channel_type, capacity)?;
        let provider = safe_managed_alloc!(1024, CrateId::Component)?;

        let channel = AsyncChannel {
            id: channel_id,
            channel_type,
            capacity,
            buffer,
            sender_wakers: BoundedVec::new(provider.clone())?,
            receiver_wakers: BoundedVec::new(provider)?,
            closed: AtomicBool::new(false),
            sender_count: AtomicU32::new(1),
            receiver_count: AtomicU32::new(1),
            total_sent: AtomicU64::new(0),
            total_received: AtomicU64::new(0),
            fuel_consumed: AtomicU64::new(CHANNEL_CREATE_FUEL),
            created_at: self.get_timestamp(),
        };

        // Store channel
        self.channels
            .insert(channel_id, channel)
            .map_err(|_| Error::resource_limit_exceeded("Too many active channels"))?;

        // Create sender and receiver
        let channels_weak = Arc::downgrade(&Arc::new(Mutex::new(self)));

        let sender = ChannelSender {
            channel_id,
            component_id,
            channels_ref: channels_weak.clone(),
        };

        let receiver = ChannelReceiver {
            channel_id,
            component_id,
            channels_ref: channels_weak,
        };

        // Add to component context
        context
            .owned_channels
            .push(channel_id)
            .map_err(|_| Error::resource_limit_exceeded("Component channel list full"))?;

        context.senders.insert(channel_id, sender.clone()).ok();
        context.receivers.insert(channel_id, receiver.clone()).ok();

        // Update statistics
        self.channel_stats.total_channels_created.fetch_add(1, Ordering::Relaxed);

        Ok((sender, receiver))
    }

    /// Send message through channel
    pub fn send_message(
        &mut self,
        channel_id: ChannelId,
        sender_id: ComponentInstanceId,
        message: ComponentValue,
        priority: Option<u8>,
    ) -> Result<SendResult, Error> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or_else(|| Error::validation_invalid_input("Channel not found"))?;

        if channel.closed.load(Ordering::Acquire) {
            return Ok(SendResult::Closed);
        }

        let channel_message = ChannelMessage {
            value: message,
            sender_id,
            sent_at: self.get_timestamp(),
            priority: priority.unwrap_or(0),
        };

        // Try to send message
        let send_result = match &mut channel.buffer {
            ChannelBuffer::Ring {
                data,
                head,
                tail,
                len,
            } => {
                let current_len = len.load(Ordering::Acquire);
                if current_len >= channel.capacity {
                    if self.global_config.enable_backpressure {
                        self.channel_stats.backpressure_events.fetch_add(1, Ordering::Relaxed);
                        SendResult::WouldBlock
                    } else {
                        SendResult::Full
                    }
                } else {
                    let tail_idx = tail.load(Ordering::Acquire);
                    data[tail_idx % channel.capacity] = channel_message;
                    tail.store((tail_idx + 1) % channel.capacity, Ordering::Release);
                    len.fetch_add(1, Ordering::AcqRel);
                    SendResult::Sent
                }
            },
            ChannelBuffer::Vector { data } => {
                if data.len() >= self.global_config.max_unbounded_size {
                    SendResult::Full
                } else {
                    data.push(channel_message)
                        .map_err(|_| Error::resource_limit_exceeded("Channel buffer full"))?;
                    SendResult::Sent
                }
            },
            ChannelBuffer::Single { data, taken } => {
                if data.is_some() {
                    SendResult::Full
                } else {
                    *data = Some(channel_message);
                    SendResult::Sent
                }
            },
            ChannelBuffer::Priority { data } => {
                let priority_msg = PriorityMessage {
                    message:  channel_message,
                    priority: priority.unwrap_or(0),
                };
                data.push(priority_msg)
                    .map_err(|_| Error::resource_limit_exceeded("Priority channel full"))?;
                SendResult::Sent
            },
        };

        if send_result == SendResult::Sent {
            // Update statistics
            channel.total_sent.fetch_add(1, Ordering::Relaxed);
            channel.fuel_consumed.fetch_add(CHANNEL_SEND_FUEL, Ordering::Relaxed);
            self.channel_stats.total_messages_sent.fetch_add(1, Ordering::Relaxed);

            // Wake receivers
            self.wake_receivers(channel)?;
        }

        Ok(send_result)
    }

    /// Receive message from channel
    pub fn receive_message(
        &mut self,
        channel_id: ChannelId,
        receiver_id: ComponentInstanceId,
    ) -> Result<ReceiveResult, Error> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or_else(|| Error::validation_invalid_input("Channel not found"))?;

        // Try to receive message
        let receive_result = match &mut channel.buffer {
            ChannelBuffer::Ring {
                data,
                head,
                tail,
                len,
            } => {
                let current_len = len.load(Ordering::Acquire);
                if current_len == 0 {
                    if channel.closed.load(Ordering::Acquire) {
                        ReceiveResult::Closed
                    } else {
                        ReceiveResult::WouldBlock
                    }
                } else {
                    let head_idx = head.load(Ordering::Acquire);
                    let message = data[head_idx % channel.capacity].clone();
                    head.store((head_idx + 1) % channel.capacity, Ordering::Release);
                    len.fetch_sub(1, Ordering::AcqRel);
                    ReceiveResult::Received(message)
                }
            },
            ChannelBuffer::Vector { data } => {
                if data.is_empty() {
                    if channel.closed.load(Ordering::Acquire) {
                        ReceiveResult::Closed
                    } else {
                        ReceiveResult::WouldBlock
                    }
                } else {
                    let message = data.remove(0);
                    ReceiveResult::Received(message)
                }
            },
            ChannelBuffer::Single { data, taken } => {
                if let Some(message) = data.take() {
                    taken.store(true, Ordering::Release);
                    ReceiveResult::Received(message)
                } else if channel.closed.load(Ordering::Acquire) {
                    ReceiveResult::Closed
                } else {
                    ReceiveResult::WouldBlock
                }
            },
            ChannelBuffer::Priority { data } => {
                if data.is_empty() {
                    if channel.closed.load(Ordering::Acquire) {
                        ReceiveResult::Closed
                    } else {
                        ReceiveResult::WouldBlock
                    }
                } else {
                    // Get highest priority message
                    data.sort_by(|a, b| b.priority.cmp(&a.priority));
                    let priority_msg = data.remove(0);
                    ReceiveResult::Received(priority_msg.message)
                }
            },
        };

        if let ReceiveResult::Received(_) = receive_result {
            // Update statistics
            channel.total_received.fetch_add(1, Ordering::Relaxed);
            channel.fuel_consumed.fetch_add(CHANNEL_RECV_FUEL, Ordering::Relaxed);
            self.channel_stats.total_messages_received.fetch_add(1, Ordering::Relaxed);

            // Wake senders if backpressure was active
            if self.global_config.enable_backpressure {
                self.wake_senders(channel)?;
            }
        }

        Ok(receive_result)
    }

    /// Close a channel
    pub fn close_channel(&mut self, channel_id: ChannelId) -> Result<(), Error> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or_else(|| Error::validation_invalid_input("Channel not found"))?;

        channel.closed.store(true, Ordering::Release);
        channel.fuel_consumed.fetch_add(CHANNEL_CLOSE_FUEL, Ordering::Relaxed);

        // Wake all waiters
        self.wake_all_waiters(channel)?;

        // Update statistics
        self.channel_stats.total_channel_closes.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get channel statistics
    pub fn get_channel_statistics(&self) -> ChannelStats {
        ChannelStats {
            total_channels_created:  self
                .channel_stats
                .total_channels_created
                .load(Ordering::Relaxed),
            total_messages_sent:     self.channel_stats.total_messages_sent.load(Ordering::Relaxed),
            total_messages_received: self
                .channel_stats
                .total_messages_received
                .load(Ordering::Relaxed),
            total_channel_closes:    self
                .channel_stats
                .total_channel_closes
                .load(Ordering::Relaxed),
            backpressure_events:     self.channel_stats.backpressure_events.load(Ordering::Relaxed),
            wake_coalescings:        self.channel_stats.wake_coalescings.load(Ordering::Relaxed),
            active_channels:         self.channels.len() as u64,
            total_fuel_consumed:     self.channel_stats.total_fuel_consumed.load(Ordering::Relaxed),
        }
    }

    // Private helper methods

    fn create_channel_buffer(
        &self,
        channel_type: ChannelType,
        capacity: usize,
    ) -> Result<ChannelBuffer, Error> {
        let buffer_size = capacity * 128;
        let provider = safe_managed_alloc!(buffer_size, CrateId::Component)?;

        match channel_type {
            ChannelType::Bounded(_) => Ok(ChannelBuffer::Ring {
                data: BoundedVec::new(provider)?,
                head: AtomicUsize::new(0),
                tail: AtomicUsize::new(0),
                len:  AtomicUsize::new(0),
            }),
            ChannelType::Unbounded => Ok(ChannelBuffer::Vector {
                data: BoundedVec::new(provider)?,
            }),
            ChannelType::Oneshot => Ok(ChannelBuffer::Single {
                data:  None,
                taken: AtomicBool::new(false),
            }),
            ChannelType::Broadcast(_) => Ok(ChannelBuffer::Vector {
                data: BoundedVec::new(provider)?,
            }),
            ChannelType::Priority => Ok(ChannelBuffer::Priority {
                data: BoundedVec::new(provider)?,
            }),
        }
    }

    fn wake_receivers(&mut self, channel: &mut AsyncChannel) -> Result<(), Error> {
        // Wake all receiver wakers
        for waker in channel.receiver_wakers.drain(..) {
            waker.wake();
        }

        if self.global_config.wake_coalescing {
            self.channel_stats.wake_coalescings.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    fn wake_senders(&mut self, channel: &mut AsyncChannel) -> Result<(), Error> {
        // Wake all sender wakers
        for waker in channel.sender_wakers.drain(..) {
            waker.wake();
        }

        Ok(())
    }

    fn wake_all_waiters(&mut self, channel: &mut AsyncChannel) -> Result<(), Error> {
        self.wake_senders(channel)?;
        self.wake_receivers(channel)?;
        Ok(())
    }

    fn get_timestamp(&self) -> u64 {
        // In real implementation, would use proper time source
        0
    }
}

/// Result of send operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendResult {
    /// Message sent successfully
    Sent,
    /// Channel is full, would block
    WouldBlock,
    /// Channel is full and cannot accept more
    Full,
    /// Channel is closed
    Closed,
}

/// Result of receive operation
#[derive(Debug, Clone, PartialEq)]
pub enum ReceiveResult {
    /// Message received successfully
    Received(ChannelMessage),
    /// No message available, would block
    WouldBlock,
    /// Channel is closed
    Closed,
}

/// Channel statistics
#[derive(Debug, Clone)]
pub struct ChannelStats {
    pub total_channels_created:  u64,
    pub total_messages_sent:     u64,
    pub total_messages_received: u64,
    pub total_channel_closes:    u64,
    pub backpressure_events:     u64,
    pub wake_coalescings:        u64,
    pub active_channels:         u64,
    pub total_fuel_consumed:     u64,
}

/// Send future for async channel operations
pub struct SendFuture {
    sender:   ChannelSender,
    message:  Option<ComponentValue>,
    priority: Option<u8>,
}

impl CoreFuture for SendFuture {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(channels) = self.sender.channels_ref.upgrade() {
            if let Ok(mut channels) = channels.lock() {
                if let Some(message) = self.message.take() {
                    match channels.send_message(
                        self.sender.channel_id,
                        self.sender.component_id,
                        message,
                        self.priority,
                    ) {
                        Ok(SendResult::Sent) => Poll::Ready(Ok(())),
                        Ok(SendResult::WouldBlock) => {
                            // Register waker for when space becomes available
                            if let Some(channel) =
                                channels.channels.get_mut(&self.sender.channel_id)
                            {
                                channel.sender_wakers.push(cx.waker().clone()).ok();
                            }
                            Poll::Pending
                        },
                        Ok(SendResult::Closed) => {
                            Poll::Ready(Err(Error::invalid_state_error("Channel closed")))
                        },
                        Ok(SendResult::Full) => {
                            Poll::Ready(Err(Error::resource_limit_exceeded("Channel full")))
                        },
                        Err(e) => Poll::Ready(Err(e)),
                    }
                } else {
                    Poll::Ready(Err(Error::invalid_state_error("Message already sent")))
                }
            } else {
                Poll::Ready(Err(Error::invalid_state_error(
                    "Channel manager unavailable",
                )))
            }
        } else {
            Poll::Ready(Err(Error::invalid_state_error("Channel manager dropped")))
        }
    }
}

/// Receive future for async channel operations
pub struct ReceiveFuture {
    receiver: ChannelReceiver,
}

impl CoreFuture for ReceiveFuture {
    type Output = Result<ComponentValue, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(channels) = self.receiver.channels_ref.upgrade() {
            if let Ok(mut channels) = channels.lock() {
                match channels.receive_message(self.receiver.channel_id, self.receiver.component_id)
                {
                    Ok(ReceiveResult::Received(message)) => Poll::Ready(Ok(message.value)),
                    Ok(ReceiveResult::WouldBlock) => {
                        // Register waker for when message becomes available
                        if let Some(channel) = channels.channels.get_mut(&self.receiver.channel_id)
                        {
                            channel.receiver_wakers.push(cx.waker().clone()).ok();
                        }
                        Poll::Pending
                    },
                    Ok(ReceiveResult::Closed) => {
                        Poll::Ready(Err(Error::invalid_state_error("Channel closed")))
                    },
                    Err(e) => Poll::Ready(Err(e)),
                }
            } else {
                Poll::Ready(Err(Error::invalid_state_error(
                    "Channel manager unavailable",
                )))
            }
        } else {
            Poll::Ready(Err(Error::invalid_state_error("Channel manager dropped")))
        }
    }
}

impl ChannelSender {
    /// Send a message asynchronously
    pub fn send(&self, message: ComponentValue) -> SendFuture {
        SendFuture {
            sender:   self.clone(),
            message:  Some(message),
            priority: None,
        }
    }

    /// Send a message with priority
    pub fn send_with_priority(&self, message: ComponentValue, priority: u8) -> SendFuture {
        SendFuture {
            sender:   self.clone(),
            message:  Some(message),
            priority: Some(priority),
        }
    }
}

impl ChannelReceiver {
    /// Receive a message asynchronously
    pub fn receive(&self) -> ReceiveFuture {
        ReceiveFuture {
            receiver: self.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        task_manager::TaskManager,
        threading::thread_spawn_fuel::FuelTrackedThreadManager,
    };

    fn create_test_bridge() -> Arc<Mutex<TaskManagerAsyncBridge>> {
        let task_manager = Arc::new(Mutex::new(TaskManager::new()));
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
        let config = crate::async_::task_manager_async_bridge::BridgeConfiguration::default();
        let bridge = crate::async_::task_manager_async_bridge::TaskManagerAsyncBridge::new(
            task_manager,
            thread_manager,
            config,
        )
        .unwrap();
        Arc::new(Mutex::new(bridge))
    }

    #[test]
    fn test_channel_creation() {
        let bridge = create_test_bridge();
        let mut channels = OptimizedAsyncChannels::new(bridge, None).unwrap();

        let component_id = ComponentInstanceId::new(1);
        channels.initialize_component_channels(component_id, None).unwrap();

        let (sender, receiver) =
            channels.create_channel(component_id, ChannelType::Bounded(32)).unwrap();

        assert_eq!(sender.component_id, component_id);
        assert_eq!(receiver.component_id, component_id);
    }

    #[test]
    fn test_channel_statistics() {
        let bridge = create_test_bridge();
        let channels = OptimizedAsyncChannels::new(bridge, None).unwrap();

        let stats = channels.get_channel_statistics();
        assert_eq!(stats.total_channels_created, 0);
        assert_eq!(stats.active_channels, 0);
    }

    #[test]
    fn test_channel_types() {
        assert_eq!(ChannelType::Oneshot, ChannelType::Oneshot);
        assert_ne!(ChannelType::Bounded(32), ChannelType::Unbounded);

        match ChannelType::Bounded(64) {
            ChannelType::Bounded(cap) => assert_eq!(cap, 64),
            _ => panic!("Expected bounded channel"),
        }
    }
}
