//! Async Canonical ABI implementation for WebAssembly Component Model
//!
//! This module implements async lifting and lowering operations for the
//! Component Model's canonical ABI, enabling asynchronous component calls
//! with streams, futures, and error contexts.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(feature = "std")]
use std::{boxed::Box, collections::BTreeMap, vec::Vec};

// Enable vec! macro for no_std
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{vec, boxed::Box};

#[cfg(not(feature = "std"))]
use wrt_foundation::{
    BoundedVec as Vec, BoundedMap as BTreeMap, 
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

use wrt_foundation::{
    bounded::BoundedVec, prelude::*, WrtResult,
};

#[cfg(feature = "std")]
use wrt_foundation::{component_value::ComponentValue, resource::ResourceHandle};

use crate::{
    async_::async_types::{
        AsyncReadResult, ErrorContext, ErrorContextHandle, Future, FutureHandle, FutureState,
        Stream, StreamHandle, StreamState, Waitable, WaitableSet,
    },
    types::{ValType, Value},
};

use wrt_error::{Error, ErrorCategory, Result};

// Temporary stubs for missing types
#[derive(Debug, Clone, Default)]
pub struct CanonicalAbi;

impl CanonicalAbi {
    pub fn new() -> Self { Self }
}

#[derive(Debug, Clone, Default)]
pub struct CanonicalOptions;

#[derive(Debug, Clone, Default)]
pub struct CanonicalLiftContext {
    pub options: CanonicalOptions,
}

impl Default for CanonicalLiftContext {
    fn default() -> Self { Self { options: CanonicalOptions } }
}

#[derive(Debug, Clone, Default)]
pub struct CanonicalLowerContext {
    pub options: CanonicalOptions,
}

impl Default for CanonicalLowerContext {
    fn default() -> Self { Self { options: CanonicalOptions } }
}

#[derive(Debug, Clone)]
pub struct TaskManager;

impl TaskManager {
    pub fn new() -> Self { Self }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Component,
    Async,
}

// Stub module for missing async_canonical_lifting functions
pub mod async_canonical_lifting {
    use super::*;
    
    pub fn async_canonical_lift(
        _values: &[u8],
        _target_types: &[ValType],
        _options: &CanonicalOptions,
    ) -> Result<Vec<Value>> {
        Ok(vec![])
    }
    
    pub fn async_canonical_lower(
        _values: &[Value],
        _options: &CanonicalOptions,
    ) -> Result<Vec<u8>> {
        Ok(vec![])
    }
}

/// Maximum number of streams/futures in no_std environments
const MAX_ASYNC_RESOURCES: usize = 256;

/// Maximum number of async operations in flight for no_std environments
const MAX_ASYNC_OPS: usize = 256;

/// Maximum size for async call contexts in no_std environments
const MAX_ASYNC_CONTEXT_SIZE: usize = 64;

/// Async operation tracking
#[derive(Debug, Clone)]
pub struct AsyncOperation {
    /// Operation ID
    pub id: u32,
    /// Operation type
    pub op_type: AsyncOperationType,
    /// Current state
    pub state: AsyncOperationState,
    /// Associated context
    #[cfg(feature = "std")]
    pub context: Vec<u8>,
    #[cfg(not(any(feature = "std", )))]
    pub context: BoundedVec<u8>,
    /// Task handle for cancellation
    pub task_handle: Option<u32>,
}

/// Type of async operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncOperationType {
    /// Async call to a component function
    AsyncCall,
    /// Stream read operation
    StreamRead,
    /// Stream write operation
    StreamWrite,
    /// Future get operation
    FutureGet,
    /// Future set operation
    FutureSet,
    /// Waitable poll operation
    WaitablePoll,
}

/// State of an async operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncOperationState {
    /// Operation is starting
    Starting,
    /// Operation is in progress
    InProgress,
    /// Operation is waiting for resources
    Waiting,
    /// Operation has completed successfully
    Completed,
    /// Operation was cancelled
    Cancelled,
    /// Operation failed with error
    Failed,
}

/// Results of async lifting operations
#[derive(Debug, Clone)]
pub enum AsyncLiftResult {
    /// Values are immediately available
    Immediate(Vec<Value>),
    /// Operation needs to wait for async completion
    Pending(AsyncOperation),
    /// Stream for incremental reading
    Stream(StreamHandle),
    /// Future for deferred value
    Future(FutureHandle),
    /// Error occurred during lifting
    Error(ErrorContextHandle),
}

/// Results of async lowering operations
#[derive(Debug, Clone)]
pub enum AsyncLowerResult {
    /// Values were immediately lowered
    Immediate(Vec<u8>),
    /// Operation needs async completion
    Pending(AsyncOperation),
    /// Stream for incremental writing
    Stream(StreamHandle),
    /// Future for deferred lowering
    Future(FutureHandle),
    /// Error occurred during lowering
    Error(ErrorContextHandle),
}

/// Async canonical ABI implementation
pub struct AsyncCanonicalAbi {
    /// Base canonical ABI
    canonical_abi: CanonicalAbi,

    /// Task manager
    task_manager: TaskManager,

    /// Stream registry
    #[cfg(feature = "std")]
    streams: BTreeMap<StreamHandle, Box<dyn StreamValue>>,
    #[cfg(not(any(feature = "std", )))]
    streams: BoundedVec<(StreamHandle, StreamValueEnum)>,

    /// Future registry
    #[cfg(feature = "std")]
    futures: BTreeMap<FutureHandle, Box<dyn FutureValue>>,
    #[cfg(not(any(feature = "std", )))]
    futures: BoundedVec<(FutureHandle, FutureValueEnum)>,

    /// Error context registry
    #[cfg(feature = "std")]
    error_contexts: BTreeMap<ErrorContextHandle, ErrorContext>,
    #[cfg(not(any(feature = "std", )))]
    error_contexts: BoundedVec<(ErrorContextHandle, ErrorContext)>,

    /// Next handle IDs
    next_stream_handle: u32,
    next_future_handle: u32,
    next_error_context_handle: u32,
}

/// Stream value trait for type erasure
#[cfg(feature = "std")]
pub trait StreamValue: fmt::Debug {
    fn read(&mut self) -> WrtResult<AsyncReadResult>;
    fn write(&mut self, values: &[Value]) -> WrtResult<()>;
    fn cancel_read(&mut self) -> WrtResult<()>;
    fn cancel_write(&mut self) -> WrtResult<()>;
    fn close_readable(&mut self) -> WrtResult<()>;
    fn close_writable(&mut self) -> WrtResult<()>;
    fn element_type(&self) -> &ValType;
    fn is_readable(&self) -> bool;
    fn is_writable(&self) -> bool;
}

/// Future value trait for type erasure
#[cfg(feature = "std")]
pub trait FutureValue: fmt::Debug {
    fn read(&mut self) -> WrtResult<AsyncReadResult>;
    fn write(&mut self, value: &Value) -> WrtResult<()>;
    fn cancel_read(&mut self) -> WrtResult<()>;
    fn cancel_write(&mut self) -> WrtResult<()>;
    fn close_readable(&mut self) -> WrtResult<()>;
    fn close_writable(&mut self) -> WrtResult<()>;
    fn value_type(&self) -> &ValType;
    fn is_readable(&self) -> bool;
    fn is_writable(&self) -> bool;
}

/// Enum for stream values in no_std environments
#[cfg(not(any(feature = "std", )))]
#[derive(Debug)]
pub enum StreamValueEnum {
    Values(Stream<Value>),
    // Add more variants as needed for different types
}

/// Enum for future values in no_std environments
#[cfg(not(any(feature = "std", )))]
#[derive(Debug)]
pub enum FutureValueEnum {
    Value(Future<Value>),
    // Add more variants as needed for different types
}

/// Concrete stream implementation
#[derive(Debug)]
pub struct ConcreteStream<T> {
    inner: Stream<T>,
}

/// Concrete future implementation
#[derive(Debug)]
pub struct ConcreteFuture<T> {
    inner: Future<T>,
}

impl AsyncCanonicalAbi {
    /// Create a new async canonical ABI
    pub fn new() -> WrtResult<Self> {
        Ok(Self {
            canonical_abi: CanonicalAbi::new(),
            task_manager: TaskManager::new(),
            #[cfg(feature = "std")]
            streams: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            streams: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(feature = "std")]
            futures: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            futures: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(feature = "std")]
            error_contexts: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            error_contexts: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            next_stream_handle: 0,
            next_future_handle: 0,
            next_error_context_handle: 0,
        })
    }

    /// Create a new stream
    pub fn stream_new(&mut self, element_type: &ValType) -> WrtResult<StreamHandle> {
        let handle = StreamHandle(self.next_stream_handle);
        self.next_stream_handle += 1;

        let stream = Stream::new(handle, element_type.clone());

        #[cfg(feature = "std")]
        {
            let concrete = ConcreteStream { inner: stream };
            self.streams.insert(handle, Box::new(concrete));
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let stream_enum = StreamValueEnum::Values(stream);
            self.streams.push((handle, stream_enum)).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many streams")
            })?;
        }

        Ok(handle)
    }

    /// Read from a stream
    pub fn stream_read(&mut self, stream_handle: StreamHandle) -> WrtResult<AsyncReadResult> {
        #[cfg(feature = "std")]
        {
            if let Some(stream) = self.streams.get_mut(&stream_handle) {
                stream.read()
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid stream handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (handle, stream) in &mut self.streams {
                if *handle == stream_handle {
                    return match stream {
                        StreamValueEnum::Values(ref mut s) => {
                            if s.buffer.is_empty() {
                                if s.writable_closed {
                                    Ok(AsyncReadResult::Closed)
                                } else {
                                    Ok(AsyncReadResult::Blocked)
                                }
                            } else {
                                // Read one value
                                let value = s.buffer.remove(0);
                                #[cfg(feature = "std")]
                                {
                                    Ok(AsyncReadResult::Values(vec![value]))
                                }
                                #[cfg(not(feature = "std"))]
                                {
                                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                                    let mut values = BoundedVec::new(provider)?;
                                    values.push(value).map_err(|_| wrt_error::Error::runtime_execution_error("Failed to push value"))?;
                                    Ok(AsyncReadResult::Values(values))
                                }
                            }
                        }
                    };
                }
            }
            Err(wrt_error::Error::new(wrt_error::ErrorCategory::Validation,
                    wrt_error::errors::codes::INVALID_INPUT,
                    "Invalid stream handle"))
        }
    }

    /// Write to a stream
    pub fn stream_write(&mut self, stream_handle: StreamHandle, values: &[Value]) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            if let Some(stream) = self.streams.get_mut(&stream_handle) {
                stream.write(values)
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (handle, stream) in &mut self.streams {
                if *handle == stream_handle {
                    return match stream {
                        StreamValueEnum::Values(ref mut s) => {
                            if s.writable_closed {
                                return Err(wrt_error::Error::runtime_execution_error("Stream is closed"));
                            }
                            for value in values {
                                s.buffer.push(value.clone()).map_err(|_| {
                                    wrt_error::Error::resource_exhausted("Buffer full")
                                })?;
                            }
                            s.state = StreamState::Ready;
                            Ok(())
                        }
                    };
                }
            }
            Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
        }
    }

    /// Cancel read operation on a stream
    pub fn stream_cancel_read(&mut self, stream_handle: StreamHandle) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            if let Some(stream) = self.streams.get_mut(&stream_handle) {
                stream.cancel_read()
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (handle, stream) in &mut self.streams {
                if *handle == stream_handle {
                    return match stream {
                        StreamValueEnum::Values(ref mut s) => {
                            s.close_readable();
                            Ok(())
                        }
                    };
                }
            }
            Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
        }
    }

    /// Cancel write operation on a stream
    pub fn stream_cancel_write(&mut self, stream_handle: StreamHandle) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            if let Some(stream) = self.streams.get_mut(&stream_handle) {
                stream.cancel_write()
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (handle, stream) in &mut self.streams {
                if *handle == stream_handle {
                    return match stream {
                        StreamValueEnum::Values(ref mut s) => {
                            s.close_writable();
                            Ok(())
                        }
                    };
                }
            }
            Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
        }
    }

    /// Close readable end of a stream
    pub fn stream_close_readable(&mut self, stream_handle: StreamHandle) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            if let Some(stream) = self.streams.get_mut(&stream_handle) {
                stream.close_readable()
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (handle, stream) in &mut self.streams {
                if *handle == stream_handle {
                    return match stream {
                        StreamValueEnum::Values(ref mut s) => {
                            s.close_readable();
                            Ok(())
                        }
                    };
                }
            }
            Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
        }
    }

    /// Close writable end of a stream
    pub fn stream_close_writable(&mut self, stream_handle: StreamHandle) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            if let Some(stream) = self.streams.get_mut(&stream_handle) {
                stream.close_writable()
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (handle, stream) in &mut self.streams {
                if *handle == stream_handle {
                    return match stream {
                        StreamValueEnum::Values(ref mut s) => {
                            s.close_writable();
                            Ok(())
                        }
                    };
                }
            }
            Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
        }
    }

    /// Create a new future
    pub fn future_new(&mut self, value_type: &ValType) -> WrtResult<FutureHandle> {
        let handle = FutureHandle(self.next_future_handle);
        self.next_future_handle += 1;

        let future = Future::new(handle, value_type.clone());

        #[cfg(feature = "std")]
        {
            let concrete = ConcreteFuture { inner: future };
            self.futures.insert(handle, Box::new(concrete));
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let future_enum = FutureValueEnum::Value(future);
            self.futures.push((handle, future_enum)).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many futures")
            })?;
        }

        Ok(handle)
    }

    /// Read from a future
    pub fn future_read(&mut self, future_handle: FutureHandle) -> WrtResult<AsyncReadResult> {
        #[cfg(feature = "std")]
        {
            if let Some(future) = self.futures.get_mut(&future_handle) {
                future.read()
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (handle, future) in &mut self.futures {
                if *handle == future_handle {
                    return match future {
                        FutureValueEnum::Value(ref mut f) => match f.state {
                            FutureState::Ready => {
                                if let Some(value) = f.value.take() {
                                    #[cfg(feature = "std")]
                                    {
                                        Ok(AsyncReadResult::Values(vec![value]))
                                    }
                                    #[cfg(not(feature = "std"))]
                                    {
                                        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                                        let mut values = BoundedVec::new(provider)?;
                                        values.push(value).map_err(|_| wrt_error::Error::runtime_execution_error("Failed to push value"))?;
                                        Ok(AsyncReadResult::Values(values))
                                    }
                                } else {
                                    Ok(AsyncReadResult::Closed)
                                }
                            }
                            FutureState::Cancelled => Ok(AsyncReadResult::Closed),
                            FutureState::Error => Ok(AsyncReadResult::Closed),
                            FutureState::Pending => Ok(AsyncReadResult::Blocked),
                        }
                    }
                }
            }
            Err(wrt_error::Error::new(wrt_error::ErrorCategory::Validation,
                    wrt_error::errors::codes::INVALID_INPUT,
                    "Invalid stream handle"))
        }
    }

    /// Write to a future
    pub fn future_write(&mut self, future_handle: FutureHandle, value: &Value) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            if let Some(future) = self.futures.get_mut(&future_handle) {
                future.write(value)
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (handle, future) in &mut self.futures {
                if *handle == future_handle {
                    return match future {
                        FutureValueEnum::Value(ref mut f) => f.set_value(value.clone()),
                    };
                }
            }
            Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
        }
    }

    /// Create a new error context
    pub fn error_context_new(&mut self, message: &str) -> WrtResult<ErrorContextHandle> {
        let handle = ErrorContextHandle(self.next_error_context_handle);
        self.next_error_context_handle += 1;

        #[cfg(feature = "std")]
        let error_context = ErrorContext::new(handle, message.to_string());
        #[cfg(not(any(feature = "std", )))]
        let error_context =
            ErrorContext::new(handle, BoundedString::from_str(message).unwrap_or_default());

        #[cfg(feature = "std")]
        {
            self.error_contexts.insert(handle, error_context);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.error_contexts.push((handle, error_context)).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many error contexts")
            })?;
        }

        Ok(handle)
    }

    /// Get debug string from error context
    pub fn error_context_debug_string(
        &self,
        handle: ErrorContextHandle,
    ) -> WrtResult<BoundedString<2048>> {
        #[cfg(feature = "std")]
        {
            if let Some(error_context) = self.error_contexts.get(&handle) {
                Ok(error_context.debug_string())
            } else {
                Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (ctx_handle, error_context) in &self.error_contexts {
                if *ctx_handle == handle {
                    return Ok(error_context.debug_string());
                }
            }
            Err(wrt_error::Error::runtime_execution_error("Invalid handle"))
        }
    }

    /// Drop an error context
    pub fn error_context_drop(&mut self, handle: ErrorContextHandle) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.error_contexts.remove(&handle);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.error_contexts.retain(|(h, _)| *h != handle);
        }
        Ok(())
    }

    /// Task return operation
    pub fn task_return(&mut self, values: Vec<Value>) -> WrtResult<()> {
        self.task_manager.task_return(values)
    }

    /// Task wait operation
    pub fn task_wait(&mut self, waitables: WaitableSet) -> WrtResult<u32> {
        self.task_manager.task_wait(waitables)
    }

    /// Task poll operation
    pub fn task_poll(&self, waitables: &WaitableSet) -> WrtResult<Option<u32>> {
        self.task_manager.task_poll(waitables)
    }

    /// Task yield operation
    pub fn task_yield(&mut self) -> WrtResult<()> {
        self.task_manager.task_yield()
    }

    /// Task cancel operation
    pub fn task_cancel(&mut self, task_id: TaskId) -> WrtResult<()> {
        self.task_manager.task_cancel(task_id)
    }

    /// Task backpressure operation
    pub fn task_backpressure(&mut self) -> WrtResult<()> {
        self.task_manager.task_backpressure()
    }

    /// Get the underlying task manager
    pub fn task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    /// Get mutable task manager
    pub fn task_manager_mut(&mut self) -> &mut TaskManager {
        &mut self.task_manager
    }

    /// Get the underlying canonical ABI
    pub fn canonical_abi(&self) -> &CanonicalAbi {
        &self.canonical_abi
    }

    /// Get mutable canonical ABI
    pub fn canonical_abi_mut(&mut self) -> &mut CanonicalAbi {
        &mut self.canonical_abi
    }

    /// Perform async lifting of values from core representation
    pub fn async_lift(
        &mut self,
        values: &[u8],
        target_types: &[ValType],
        context: &CanonicalLiftContext,
    ) -> WrtResult<AsyncLiftResult> {
        // Check for immediate values first
        if self.can_lift_immediately(values, target_types)? {
            let lifted_values = self.lift_immediate(values, target_types, &context.options)?;
            return Ok(AsyncLiftResult::Immediate(lifted_values));
        }

        // Check for stream types
        if target_types.len() == 1 {
            if let ValType::Stream(_) = &target_types[0] {
                let stream_handle = self.stream_new(&target_types[0])?;
                return Ok(AsyncLiftResult::Stream(stream_handle));
            }
            if let ValType::Future(_) = &target_types[0] {
                let future_handle = self.future_new(&target_types[0])?;
                return Ok(AsyncLiftResult::Future(future_handle));
            }
        }

        // Create pending async operation for complex lifting
        let operation = AsyncOperation {
            id: self.next_error_context_handle, // Reuse counter
            op_type: AsyncOperationType::AsyncCall,
            state: AsyncOperationState::Starting,
            #[cfg(feature = "std")]
            context: values.to_vec(),
            #[cfg(not(any(feature = "std", )))]
            context: BoundedVec::from_slice(values).map_err(|_| {
                Error::runtime_execution_error("Context too large")
            })?,
            task_handle: None,
        };

        self.next_error_context_handle += 1;
        Ok(AsyncLiftResult::Pending(operation))
    }

    /// Perform async lowering of values to core representation
    pub fn async_lower(
        &mut self,
        values: &[Value],
        context: &CanonicalLowerContext,
    ) -> WrtResult<AsyncLowerResult> {
        // Check for immediate lowering
        if self.can_lower_immediately(values)? {
            let lowered_bytes = self.lower_immediate(values, &context.options)?;
            return Ok(AsyncLowerResult::Immediate(lowered_bytes));
        }

        // Check for stream/future values
        if values.len() == 1 {
            match &values[0] {
                Value::Stream(handle) => {
                    return Ok(AsyncLowerResult::Stream(*handle));
                }
                Value::Future(handle) => {
                    return Ok(AsyncLowerResult::Future(*handle));
                }
                _ => {}
            }
        }

        // Create pending async operation for complex lowering
        let operation = AsyncOperation {
            id: self.next_error_context_handle,
            op_type: AsyncOperationType::AsyncCall,
            state: AsyncOperationState::Starting,
            #[cfg(feature = "std")]
            context: Vec::new(), // Values will be serialized separately
            #[cfg(not(any(feature = "std", )))]
            context: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            task_handle: None,
        };

        self.next_error_context_handle += 1;
        Ok(AsyncLowerResult::Pending(operation))
    }

    // Private helper methods for async operations
    fn can_lift_immediately(&self, _values: &[u8], target_types: &[ValType]) -> WrtResult<bool> {
        // Check if all target types are immediately liftable (not async types)
        for ty in target_types {
            match ty {
                ValType::Stream(_) | ValType::Future(_) => return Ok(false),
                _ => {}
            }
        }
        Ok(true)
    }

    fn can_lower_immediately(&self, values: &[Value]) -> WrtResult<bool> {
        // Check if all values are immediately lowerable (not async values)
        for value in values {
            match value {
                Value::Stream(_) | Value::Future(_) => return Ok(false),
                _ => {}
            }
        }
        Ok(true)
    }

    fn lift_immediate(&self, values: &[u8], target_types: &[ValType], options: &CanonicalOptions) -> WrtResult<Vec<Value>> {
        // Use the stub canonical ABI lifting
        async_canonical_lifting::async_canonical_lift(values, target_types, options)
    }

    fn lower_immediate(&self, values: &[Value], options: &CanonicalOptions) -> WrtResult<Vec<u8>> {
        // Use the stub canonical ABI lowering
        async_canonical_lifting::async_canonical_lower(values, options)
    }
}

// Trait implementations for std environment
#[cfg(feature = "std")]
impl<T: Clone + fmt::Debug> StreamValue for ConcreteStream<T>
where
    Value: From<T>,
    T: TryFrom<Value>,
{
    fn read(&mut self) -> WrtResult<AsyncReadResult> {
        if self.inner.buffer.is_empty() {
            if self.inner.writable_closed {
                Ok(AsyncReadResult::Closed)
            } else {
                Ok(AsyncReadResult::Blocked)
            }
        } else {
            let value = self.inner.buffer.remove(0);
            Ok(AsyncReadResult::Values(vec![Value::from(value)]))
        }
    }

    fn write(&mut self, values: &[Value]) -> WrtResult<()> {
        if self.inner.writable_closed {
            return Err(wrt_error::Error::runtime_execution_error("Stream is closed"));
        }

        for value in values {
            if let Ok(typed_value) = T::try_from(value.clone()) {
                self.inner.buffer.push(typed_value);
            } else {
                return Err(wrt_error::Error::type_mismatch_error("Value type mismatch"));
            }
        }
        self.inner.state = StreamState::Ready;
        Ok(())
    }

    fn cancel_read(&mut self) -> WrtResult<()> {
        self.inner.close_readable();
        Ok(())
    }

    fn cancel_write(&mut self) -> WrtResult<()> {
        self.inner.close_writable();
        Ok(())
    }

    fn close_readable(&mut self) -> WrtResult<()> {
        self.inner.close_readable();
        Ok(())
    }

    fn close_writable(&mut self) -> WrtResult<()> {
        self.inner.close_writable();
        Ok(())
    }

    fn element_type(&self) -> &ValType {
        &self.inner.element_type
    }

    fn is_readable(&self) -> bool {
        self.inner.is_readable()
    }

    fn is_writable(&self) -> bool {
        self.inner.is_writable()
    }
}

#[cfg(feature = "std")]
impl<T: Clone + fmt::Debug> FutureValue for ConcreteFuture<T>
where
    Value: From<T>,
    T: TryFrom<Value>,
{
    fn read(&mut self) -> WrtResult<AsyncReadResult> {
        match self.inner.state {
            FutureState::Ready => {
                if let Some(value) = self.inner.value.take() {
                    Ok(AsyncReadResult::Values(vec![Value::from(value)]))
                } else {
                    Ok(AsyncReadResult::Closed)
                }
            }
            FutureState::Cancelled | FutureState::Error => Ok(AsyncReadResult::Closed),
            FutureState::Pending => Ok(AsyncReadResult::Blocked),
        }
    }

    fn write(&mut self, value: &Value) -> WrtResult<()> {
        if let Ok(typed_value) = T::try_from(value.clone()) {
            self.inner.set_value(typed_value)
        } else {
            Err(wrt_error::Error::type_mismatch_error("Value type mismatch for future"))
        }
    }

    fn cancel_read(&mut self) -> WrtResult<()> {
        self.inner.cancel();
        Ok(())
    }

    fn cancel_write(&mut self) -> WrtResult<()> {
        self.inner.cancel();
        Ok(())
    }

    fn close_readable(&mut self) -> WrtResult<()> {
        self.inner.readable_closed = true;
        Ok(())
    }

    fn close_writable(&mut self) -> WrtResult<()> {
        self.inner.writable_closed = true;
        Ok(())
    }

    fn value_type(&self) -> &ValType {
        &self.inner.value_type
    }

    fn is_readable(&self) -> bool {
        self.inner.is_readable()
    }

    fn is_writable(&self) -> bool {
        self.inner.is_writable()
    }
}

impl Default for AsyncCanonicalAbi {
    fn default() -> Self {
        Self::new().expect("Failed to create AsyncCanonicalAbi")
    }
}

impl fmt::Display for AsyncOperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsyncOperationType::AsyncCall => write!(f, "async-call"),
            AsyncOperationType::StreamRead => write!(f, "stream-read"),
            AsyncOperationType::StreamWrite => write!(f, "stream-write"),
            AsyncOperationType::FutureGet => write!(f, "future-get"),
            AsyncOperationType::FutureSet => write!(f, "future-set"),
            AsyncOperationType::WaitablePoll => write!(f, "waitable-poll"),
        }
    }
}

impl fmt::Display for AsyncOperationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsyncOperationState::Starting => write!(f, "starting"),
            AsyncOperationState::InProgress => write!(f, "in-progress"),
            AsyncOperationState::Waiting => write!(f, "waiting"),
            AsyncOperationState::Completed => write!(f, "completed"),
            AsyncOperationState::Cancelled => write!(f, "cancelled"),
            AsyncOperationState::Failed => write!(f, "failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_canonical_abi_creation() {
        let abi = AsyncCanonicalAbi::new().unwrap();
        assert_eq!(abi.streams.len(), 0);
        assert_eq!(abi.futures.len(), 0);
        assert_eq!(abi.error_contexts.len(), 0);
    }

    #[test]
    fn test_stream_lifecycle() {
        let mut abi = AsyncCanonicalAbi::new().unwrap();

        // Create stream
        let stream_handle = abi.stream_new(&ValType::U32).unwrap();

        // Write to stream
        let values = vec![Value::U32(42), Value::U32(24)];
        abi.stream_write(stream_handle, &values).unwrap();

        // Read from stream
        let result = abi.stream_read(stream_handle).unwrap();
        match result {
            AsyncReadResult::Values(read_values) => {
                assert_eq!(read_values.len(), 1);
                assert_eq!(read_values[0], Value::U32(42));
            }
            _ => panic!("Expected values"),
        }

        // Close stream
        abi.stream_close_writable(stream_handle).unwrap();
        abi.stream_close_readable(stream_handle).unwrap();
    }

    #[test]
    fn test_future_lifecycle() {
        let mut abi = AsyncCanonicalAbi::new().unwrap();

        // Create future
        let future_handle = abi.future_new(&ValType::String).unwrap();

        // Initially should block
        let result = abi.future_read(future_handle).unwrap();
        assert!(matches!(result, AsyncReadResult::Blocked));

        // Write value
        let value = Value::String(BoundedString::from_str("hello").unwrap());
        abi.future_write(future_handle, &value).unwrap();

        // Should be ready now
        let result = abi.future_read(future_handle).unwrap();
        match result {
            AsyncReadResult::Values(values) => {
                assert_eq!(values.len(), 1);
                assert_eq!(values[0], value);
            }
            _ => panic!("Expected values"),
        }
    }

    #[test]
    fn test_error_context() {
        let mut abi = AsyncCanonicalAbi::new().unwrap();

        let handle = abi.error_context_new("Test error").unwrap();
        let debug_string = abi.error_context_debug_string(handle).unwrap();
        assert!(debug_string.as_str().contains("Test error"));

        abi.error_context_drop(handle).unwrap();

        // Should be gone
        assert!(abi.error_context_debug_string(handle).is_err());
    }

    #[test]
    fn test_task_operations() {
        let mut abi = AsyncCanonicalAbi::new().unwrap();

        // Test yield
        assert!(abi.task_yield().is_err()); // No current task

        // Test backpressure
        assert!(abi.task_backpressure().is_err()); // No current task
    }

    #[test]
    fn test_async_lift_immediate() {
        let mut abi = AsyncCanonicalAbi::new().unwrap();
        let context = CanonicalLiftContext::default();
        let values = vec![42u8, 0, 0, 0];
        let types = vec![ValType::U32];

        match abi.async_lift(&values, &types, &context).unwrap() {
            AsyncLiftResult::Immediate(vals) => {
                assert_eq!(vals.len(), 1);
                assert_eq!(vals[0], Value::U32(42));
            }
            _ => panic!("Expected immediate result"),
        }
    }

    #[test]
    fn test_async_lift_stream() {
        let mut abi = AsyncCanonicalAbi::new().unwrap();
        let context = CanonicalLiftContext::default();
        let values = vec![];
        let types = vec![ValType::Stream(Box::new(ValType::U32))];

        match abi.async_lift(&values, &types, &context).unwrap() {
            AsyncLiftResult::Stream(handle) => {
                assert_eq!(handle.0, 0);
            }
            _ => panic!("Expected stream result"),
        }
    }

    #[test]
    fn test_async_lower_immediate() {
        let mut abi = AsyncCanonicalAbi::new().unwrap();
        let context = CanonicalLowerContext::default();
        let values = vec![Value::U32(42)];

        match abi.async_lower(&values, &context).unwrap() {
            AsyncLowerResult::Immediate(bytes) => {
                assert_eq!(bytes, vec![42, 0, 0, 0]);
            }
            _ => panic!("Expected immediate result"),
        }
    }

    #[test]
    fn test_async_lower_stream() {
        let mut abi = AsyncCanonicalAbi::new().unwrap();
        let context = CanonicalLowerContext::default();
        let stream_handle = StreamHandle(42);
        let values = vec![Value::Stream(stream_handle)];

        match abi.async_lower(&values, &context).unwrap() {
            AsyncLowerResult::Stream(handle) => {
                assert_eq!(handle, stream_handle);
            }
            _ => panic!("Expected stream result"),
        }
    }

    #[test]
    fn test_operation_state_display() {
        assert_eq!(AsyncOperationState::Starting.to_string(), "starting");
        assert_eq!(AsyncOperationType::AsyncCall.to_string(), "async-call");
        assert_eq!(AsyncOperationState::Completed.to_string(), "completed");
        assert_eq!(AsyncOperationType::StreamRead.to_string(), "stream-read");
    }
}
