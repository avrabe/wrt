//! Async types for WebAssembly Component Model
//!
//! This module implements the async types (stream, future, error-context)
//! required by the Component Model MVP specification for concurrent operations.

#[cfg(not(feature = "std"))]
use core::{
    fmt,
    mem,
};
#[cfg(feature = "std")]
use std::{
    boxed::Box,
    string::String,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    fmt,
    mem,
};

use wrt_error::Result as WrtResult;
#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;
#[cfg(feature = "std")]
use wrt_foundation::{
    bounded::BoundedVec,
    component_value::ComponentValue,
    prelude::*,
};
#[cfg(not(feature = "std"))]
use wrt_foundation::{
    bounded::{
        BoundedString,
        BoundedVec,
    },
    budget_aware_provider::CrateId,
    safe_managed_alloc,
    safe_memory::NoStdProvider as _,
    NoStdProvider,
};

#[cfg(not(feature = "std"))]
// For no_std, use a simpler ComponentValue representation
use crate::types::Value as ComponentValue;
use crate::types::{
    ValType,
    Value,
};

/// Maximum number of pending values in a stream for no_std environments
const MAX_STREAM_BUFFER: usize = 1024;

/// Maximum number of waitables in a set for no_std environments  
const MAX_WAITABLES: usize = 64;

/// Handle to a stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamHandle(pub u32);

/// Handle to a future
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FutureHandle(pub u32);

/// Handle to an error context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorContextHandle(pub u32);

/// Stream type for incremental value passing
#[derive(Debug, Clone)]
pub struct Stream<T> {
    /// Stream handle
    pub handle:          StreamHandle,
    /// Element type
    pub element_type:    ValType,
    /// Stream state
    pub state:           StreamState,
    /// Buffered values
    #[cfg(feature = "std")]
    pub buffer:          Vec<T>,
    #[cfg(not(any(feature = "std",)))]
    pub buffer:          BoundedVec<T, MAX_STREAM_BUFFER, NoStdProvider<65536>>,
    /// Readable end closed
    pub readable_closed: bool,
    /// Writable end closed  
    pub writable_closed: bool,
}

/// Future type for deferred values
#[derive(Debug, Clone)]
pub struct Future<T> {
    /// Future handle
    pub handle:          FutureHandle,
    /// Value type
    pub value_type:      ValType,
    /// Future state
    pub state:           FutureState,
    /// Stored value (once available)
    pub value:           Option<T>,
    /// Readable end closed
    pub readable_closed: bool,
    /// Writable end closed
    pub writable_closed: bool,
}

/// Error context for detailed error information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Error context handle
    pub handle:      ErrorContextHandle,
    /// Error message
    #[cfg(feature = "std")]
    pub message:     String,
    #[cfg(not(any(feature = "std",)))]
    pub message:     BoundedString<1024, NoStdProvider<65536>>,
    /// Stack trace if available
    #[cfg(feature = "std")]
    pub stack_trace: Option<Vec<StackFrame>>,
    #[cfg(not(any(feature = "std",)))]
    pub stack_trace: Option<BoundedVec<StackFrame, 32, NoStdProvider<65536>>>,
    /// Additional debug information
    pub debug_info:  DebugInfo,
}

/// Stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is open and can transfer values
    Open,
    /// Stream has pending values to read
    Ready,
    /// Stream is closed (no more values)
    Closed,
    /// Stream encountered an error
    Error,
}

/// Future state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutureState {
    /// Future is pending (no value yet)
    Pending,
    /// Future has a value ready
    Ready,
    /// Future was cancelled
    Cancelled,
    /// Future encountered an error
    Error,
}

/// Stack frame information
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function name
    #[cfg(feature = "std")]
    pub function:           String,
    #[cfg(not(any(feature = "std",)))]
    pub function:           BoundedString<128, NoStdProvider<65536>>,
    /// Component instance
    pub component_instance: Option<u32>,
    /// Instruction offset
    pub offset:             Option<u32>,
}

/// Debug information for error contexts
#[derive(Debug, Clone)]
pub struct DebugInfo {
    /// Component that created the error
    pub source_component: Option<u32>,
    /// Error code if available
    pub error_code:       Option<u32>,
    /// Additional properties
    #[cfg(feature = "std")]
    pub properties:       Vec<(String, ComponentValue)>,
    #[cfg(not(any(feature = "std",)))]
    pub properties: BoundedVec<
        (BoundedString<64, NoStdProvider<65536>>, ComponentValue),
        16,
        NoStdProvider<65536>,
    >,
}

/// Async read result
#[derive(Debug, Clone)]
pub enum AsyncReadResult {
    /// Value(s) available
    Values(Vec<Value>),
    /// Stream/future closed
    Closed,
    /// Would block (no data available)
    Blocked,
    /// Error occurred
    Error(ErrorContextHandle),
}

/// Waitable resource for task synchronization
#[derive(Debug, Clone, Copy)]
pub enum Waitable {
    /// Stream readable
    StreamReadable(StreamHandle),
    /// Stream writable
    StreamWritable(StreamHandle),
    /// Future readable
    FutureReadable(FutureHandle),
    /// Future writable
    FutureWritable(FutureHandle),
}

/// Set of waitables for task synchronization
#[derive(Debug, Clone)]
pub struct WaitableSet {
    /// Waitables in the set
    #[cfg(feature = "std")]
    pub waitables:  Vec<Waitable>,
    #[cfg(not(any(feature = "std",)))]
    pub waitables:  BoundedVec<Waitable, MAX_WAITABLES, NoStdProvider<65536>>,
    /// Ready mask (bit per waitable)
    pub ready_mask: u64,
}

impl<T> Stream<T> {
    /// Create a new stream
    pub fn new(handle: StreamHandle, element_type: ValType) -> WrtResult<Self> {
        Ok(Self {
            handle,
            element_type,
            state: StreamState::Open,
            #[cfg(feature = "std")]
            buffer: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            buffer: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            readable_closed: false,
            writable_closed: false,
        })
    }

    /// Check if stream is readable
    pub fn is_readable(&self) -> bool {
        !self.readable_closed && (self.state == StreamState::Ready || !self.buffer.is_empty())
    }

    /// Check if stream is writable
    pub fn is_writable(&self) -> bool {
        !self.writable_closed && self.state != StreamState::Closed
    }

    /// Close readable end
    pub fn close_readable(&mut self) {
        self.readable_closed = true;
        if self.writable_closed {
            self.state = StreamState::Closed;
        }
    }

    /// Close writable end
    pub fn close_writable(&mut self) {
        self.writable_closed = true;
        if self.readable_closed {
            self.state = StreamState::Closed;
        }
    }
}

impl<T> Future<T> {
    /// Create a new future
    pub fn new(handle: FutureHandle, value_type: ValType) -> Self {
        Self {
            handle,
            value_type,
            state: FutureState::Pending,
            value: None,
            readable_closed: false,
            writable_closed: false,
        }
    }

    /// Check if future is readable
    pub fn is_readable(&self) -> bool {
        !self.readable_closed && self.state == FutureState::Ready
    }

    /// Check if future is writable
    pub fn is_writable(&self) -> bool {
        !self.writable_closed && self.state == FutureState::Pending
    }

    /// Set the future value
    pub fn set_value(&mut self, value: T) -> WrtResult<()> {
        if self.state != FutureState::Pending {
            return Err(wrt_error::Error::runtime_execution_error(
                "Future already completed",
            ));
        }
        self.value = Some(value);
        self.state = FutureState::Ready;
        Ok(())
    }

    /// Cancel the future
    pub fn cancel(&mut self) {
        if self.state == FutureState::Pending {
            self.state = FutureState::Cancelled;
        }
    }
}

impl ErrorContext {
    /// Create a new error context
    #[cfg(feature = "std")]
    pub fn new(handle: ErrorContextHandle, message: String) -> Self {
        Self {
            handle,
            message,
            stack_trace: None,
            debug_info: DebugInfo::new().unwrap_or_default(),
        }
    }

    /// Create a new error context (no_std)
    #[cfg(not(any(feature = "std",)))]
    pub fn new(
        handle: ErrorContextHandle,
        message: BoundedString<1024, NoStdProvider<65536>>,
    ) -> WrtResult<Self> {
        Ok(Self {
            handle,
            message,
            stack_trace: None,
            debug_info: DebugInfo::new()?,
        })
    }

    /// Get debug string representation
    pub fn debug_string(&self) -> BoundedString<2048, NoStdProvider<65536>> {
        #[cfg(feature = "std")]
        {
            let mut result = self.message.clone();
            if let Some(trace) = &self.stack_trace {
                result.push_str("\nStack trace:\n");
                for frame in trace {
                    result.push_str(&format!("  {}\n", frame));
                }
            }
            BoundedString::from_str(&result).unwrap_or_default()
        }
        #[cfg(not(any(feature = "std",)))]
        {
            // In no_std, just return the message
            self.message.clone()
        }
    }
}

impl DebugInfo {
    /// Create new debug info
    pub fn new() -> WrtResult<Self> {
        Ok(Self {
            source_component: None,
            error_code: None,
            #[cfg(feature = "std")]
            properties: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            properties: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
        })
    }

    /// Add a property
    #[cfg(feature = "std")]
    pub fn add_property(&mut self, key: String, value: ComponentValue) {
        self.properties.push((key, value));
    }

    /// Add a property (no_std)
    #[cfg(not(any(feature = "std",)))]
    pub fn add_property(
        &mut self,
        key: BoundedString<64, NoStdProvider<65536>>,
        value: ComponentValue,
    ) -> WrtResult<()> {
        self.properties
            .push((key, value))
            .map_err(|_| wrt_error::Error::runtime_execution_error("Failed to add property"))
    }
}

impl WaitableSet {
    /// Create a new waitable set
    pub fn new() -> WrtResult<Self> {
        Ok(Self {
            #[cfg(feature = "std")]
            waitables: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            waitables: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            ready_mask: 0,
        })
    }

    /// Add a waitable to the set
    pub fn add(&mut self, waitable: Waitable) -> WrtResult<u32> {
        let index = self.waitables.len();
        if index >= 64 {
            return Err(wrt_error::Error::runtime_execution_error(
                "Too many waitables",
            ));
        }

        #[cfg(feature = "std")]
        {
            self.waitables.push(waitable);
        }
        #[cfg(not(any(feature = "std",)))]
        {
            self.waitables
                .push(waitable)
                .map_err(|_| wrt_error::Error::runtime_execution_error("Error occurred"))?;
        }

        Ok(index as u32)
    }

    /// Mark a waitable as ready
    pub fn mark_ready(&mut self, index: u32) {
        if index < 64 {
            self.ready_mask |= 1u64 << index;
        }
    }

    /// Check if any waitables are ready
    pub fn has_ready(&self) -> bool {
        self.ready_mask != 0
    }

    /// Get the first ready waitable index
    pub fn first_ready(&self) -> Option<u32> {
        if self.ready_mask == 0 {
            None
        } else {
            Some(self.ready_mask.trailing_zeros())
        }
    }

    /// Clear ready state for a waitable
    pub fn clear_ready(&mut self, index: u32) {
        if index < 64 {
            self.ready_mask &= !(1u64 << index);
        }
    }
}

impl Default for StreamState {
    fn default() -> Self {
        Self::Open
    }
}

impl Default for FutureState {
    fn default() -> Self {
        Self::Pending
    }
}

impl Default for DebugInfo {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            source_component: None,
            error_code: None,
            #[cfg(feature = "std")]
            properties: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            properties: BoundedVec::new_with_default_provider().unwrap(),
        })
    }
}

impl Default for WaitableSet {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            #[cfg(feature = "std")]
            waitables: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            waitables: BoundedVec::new_with_default_provider().unwrap(),
            ready_mask: 0,
        })
    }
}

impl fmt::Display for StreamState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamState::Open => write!(f, "open"),
            StreamState::Ready => write!(f, "ready"),
            StreamState::Closed => write!(f, "closed"),
            StreamState::Error => write!(f, "error"),
        }
    }
}

impl fmt::Display for FutureState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FutureState::Pending => write!(f, "pending"),
            FutureState::Ready => write!(f, "ready"),
            FutureState::Cancelled => write!(f, "cancelled"),
            FutureState::Error => write!(f, "error"),
        }
    }
}

impl fmt::Display for Waitable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Waitable::StreamReadable(h) => write!(f, "stream-readable({})", h.0),
            Waitable::StreamWritable(h) => write!(f, "stream-writable({})", h.0),
            Waitable::FutureReadable(h) => write!(f, "future-readable({})", h.0),
            Waitable::FutureWritable(h) => write!(f, "future-writable({})", h.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_lifecycle() {
        let mut stream: Stream<Value> = Stream::new(StreamHandle(1), ValType::U32).unwrap();

        assert!(stream.is_writable());
        assert!(!stream.is_readable()); // Empty buffer

        stream.buffer.push(Value::U32(42));
        assert!(stream.is_readable());

        stream.close_writable();
        assert!(!stream.is_writable());

        stream.close_readable();
        assert_eq!(stream.state, StreamState::Closed);
    }

    #[test]
    fn test_future_lifecycle() {
        let mut future: Future<Value> = Future::new(FutureHandle(1), ValType::String);

        assert!(future.is_writable());
        assert!(!future.is_readable());

        future
            .set_value(Value::String(BoundedString::from_str("hello").unwrap()))
            .unwrap();
        assert!(future.is_readable());
        assert!(!future.is_writable());
        assert_eq!(future.state, FutureState::Ready);
    }

    #[test]
    fn test_future_cancel() {
        let mut future: Future<Value> = Future::new(FutureHandle(2), ValType::Bool);

        future.cancel();
        assert_eq!(future.state, FutureState::Cancelled);
        assert!(future.set_value(Value::Bool(true)).is_err());
    }

    #[test]
    fn test_error_context() {
        #[cfg(feature = "std")]
        let error = ErrorContext::new(ErrorContextHandle(1), "Test error".to_string());
        #[cfg(not(any(feature = "std",)))]
        let error = ErrorContext::new(
            ErrorContextHandle(1),
            BoundedString::from_str("Test error").unwrap(),
        )
        .unwrap();

        let debug_str = error.debug_string();
        assert!(debug_str.as_str().contains("Test error"));
    }

    #[test]
    fn test_waitable_set() {
        let mut set = WaitableSet::new().unwrap();

        let idx1 = set.add(Waitable::StreamReadable(StreamHandle(1))).unwrap();
        let idx2 = set.add(Waitable::FutureReadable(FutureHandle(1))).unwrap();

        assert!(!set.has_ready());

        set.mark_ready(idx1);
        assert!(set.has_ready());
        assert_eq!(set.first_ready(), Some(idx1));

        set.mark_ready(idx2);
        assert_eq!(set.first_ready(), Some(idx1)); // First ready

        set.clear_ready(idx1);
        assert_eq!(set.first_ready(), Some(idx2));
    }

    #[test]
    fn test_state_display() {
        assert_eq!(StreamState::Open.to_string(), "open");
        assert_eq!(FutureState::Pending.to_string(), "pending");
        assert_eq!(
            Waitable::StreamReadable(StreamHandle(42)).to_string(),
            "stream-readable(42)"
        );
    }
}
