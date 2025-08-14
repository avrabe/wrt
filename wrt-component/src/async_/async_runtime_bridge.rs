//! Bridge between Component Model async and Rust async (if needed)
//!
//! The WebAssembly Component Model defines its own async primitives (stream,
//! future, error-context) that are different from Rust's async/await. This
//! module provides optional bridges between them.

use core::{
    pin::Pin,
    task::{
        Context,
        Poll,
        Waker,
    },
};

#[cfg(feature = "std")]
use wrt_foundation::{
    bounded_collections::BoundedVec,
    component_value::ComponentValue,
};

use super::async_types::{
    Future as WasmFuture,
    FutureHandle,
    FutureState,
    Stream as WasmStream,
    StreamHandle,
};
#[cfg(not(feature = "std"))]
// For no_std, use a simpler ComponentValue representation
use crate::types::Value as ComponentValue;
use crate::{
    threading::task_manager::{
        TaskId,
        TaskManager,
        TaskState,
    },
    ComponentInstanceId,
    ValType,
};

/// The Component Model async primitives DO NOT require Rust's Future trait.
/// They work through their own polling/waiting mechanisms via the task manager.
///
/// However, if you want to integrate with Rust async runtimes (tokio,
/// async-std), this module provides adapters.

#[cfg(feature = "std")]
pub mod rust_async_bridge {
    use std::{
        future::Future as RustFuture,
        sync::{
            Arc,
            Mutex,
        },
        task::Wake,
    };

    use super::*;

    /// Adapter to use a Component Model Future in Rust async code
    pub struct ComponentFutureAdapter<T> {
        wasm_future:  Arc<Mutex<WasmFuture<T>>>,
        task_manager: Arc<Mutex<TaskManager>>,
        task_id:      TaskId,
    }

    impl<T: Clone + Send + 'static> RustFuture for ComponentFutureAdapter<T> {
        type Output = core::result::Result<T, String>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let future = self.wasm_future.lock().unwrap();

            match future.state {
                FutureState::Ready => {
                    if let Some(ref value) = future.value {
                        Poll::Ready(Ok(value.clone()))
                    } else {
                        Poll::Ready(Err("Future ready but no value".to_string()))
                    }
                },
                FutureState::Failed => Poll::Ready(Err("Future failed".to_string())),
                FutureState::Cancelled => Poll::Ready(Err("Future cancelled".to_string())),
                FutureState::Pending => {
                    // Register waker with task manager
                    // In a real implementation, this would notify the task manager
                    // to wake this future when the Component Model future completes
                    cx.waker().wake_by_ref();
                    Poll::Pending
                },
            }
        }
    }

    /// Adapter to use Rust futures in Component Model
    pub fn rust_future_to_component<F, T>(
        future: F,
        task_manager: &mut TaskManager,
        component_id: ComponentInstanceId,
    ) -> core::result::Result<FutureHandle, String>
    where
        F: RustFuture<Output = T> + Send + 'static,
        T: Into<ComponentValue>,
    {
        // This would spawn the Rust future and create a Component Model future
        // that completes when the Rust future completes
        // For now, this is a placeholder
        Err("Not implemented".to_string())
    }
}

/// Pure Component Model async - no Rust Future trait needed
pub mod component_async {
    use super::*;

    /// Execute an async Component Model operation without Rust's async runtime
    pub fn execute_async_operation(
        task_manager: &mut TaskManager,
        operation: AsyncOperation,
    ) -> core::result::Result<TaskId, String> {
        // Create a task for the async operation
        let task_id = task_manager
            .create_task(operation.component_id, &operation.name)
            .map_err(|e| Error::runtime_execution_error("Component not found"))?;

        // Start the task
        task_manager
            .start_task(task_id)
            .map_err(|e| Error::runtime_execution_error("Failed to start task"))?;

        Ok(task_id)
    }

    /// Poll a Component Model future manually
    pub fn poll_future<T>(
        future: &mut WasmFuture<T>,
        task_manager: &mut TaskManager,
    ) -> PollResult<T> {
        match future.state {
            FutureState::Ready => {
                if let Some(ref value) = future.value {
                    PollResult::Ready(value.clone())
                } else {
                    PollResult::Error("Future ready but no value".to_string())
                }
            },
            FutureState::Pending => PollResult::Pending,
            FutureState::Failed => PollResult::Error("Future failed".to_string()),
            FutureState::Cancelled => PollResult::Error("Future cancelled".to_string()),
        }
    }

    /// Poll a Component Model stream manually  
    pub fn poll_stream<T>(
        stream: &mut WasmStream<T>,
        task_manager: &mut TaskManager,
    ) -> StreamPollResult<T> {
        if !stream.buffer.is_empty() {
            // Return first item from buffer
            #[cfg(feature = "std")]
            {
                StreamPollResult::Item(stream.buffer.remove(0))
            }
            #[cfg(not(any(feature = "std",)))]
            {
                if let Some(item) = stream.buffer.pop_front() {
                    StreamPollResult::Item(item)
                } else {
                    StreamPollResult::Pending
                }
            }
        } else if stream.readable_closed {
            StreamPollResult::Closed
        } else {
            StreamPollResult::Pending
        }
    }

    #[derive(Debug, Clone)]
    pub struct AsyncOperation {
        pub component_id:   ComponentInstanceId,
        pub name:           String,
        pub operation_type: AsyncOperationType,
    }

    #[derive(Debug, Clone)]
    pub enum AsyncOperationType {
        StreamRead(StreamHandle),
        StreamWrite(StreamHandle, ComponentValue),
        FutureWait(FutureHandle),
        FutureComplete(FutureHandle, ComponentValue),
    }

    #[derive(Debug, Clone)]
    pub enum PollResult<T> {
        Ready(T),
        Pending,
        Error(String),
    }

    #[derive(Debug, Clone)]
    pub enum StreamPollResult<T> {
        Item(T),
        Pending,
        Closed,
    }
}

/// Example of using Component Model async WITHOUT Rust futures
#[cfg(test)]
mod tests {
    use super::{
        component_async::*,
        *,
    };

    #[test]
    fn test_component_model_async_without_rust_futures() {
        let mut task_manager = TaskManager::new();
        let component_id = ComponentInstanceId::new(1);

        // Create a Component Model future - no Rust Future trait needed!
        let future_handle = FutureHandle(1);
        let mut wasm_future = WasmFuture::<i32>::new(future_handle, ValType::I32);

        // Poll it manually
        let result = poll_future(&mut wasm_future, &mut task_manager);
        assert!(matches!(result, PollResult::Pending));

        // Complete the future
        wasm_future.set_value(42).unwrap();

        // Poll again
        let result = poll_future(&mut wasm_future, &mut task_manager);
        assert!(matches!(result, PollResult::Ready(42)));
    }

    #[test]
    fn test_component_model_stream_without_rust_futures() {
        let mut task_manager = TaskManager::new();

        // Create a Component Model stream - no Rust Stream trait needed!
        let stream_handle = StreamHandle(1);
        let mut wasm_stream = WasmStream::<String>::new(stream_handle, ValType::String);

        // Add some values
        #[cfg(feature = "std")]
        {
            wasm_stream.buffer.push("Hello".to_string());
            wasm_stream.buffer.push("World".to_string());
        }

        // Poll values manually
        let result1 = poll_stream(&mut wasm_stream, &mut task_manager);
        assert!(matches!(result1, StreamPollResult::Item(ref s) if s == "Hello"));

        let result2 = poll_stream(&mut wasm_stream, &mut task_manager);
        assert!(matches!(result2, StreamPollResult::Item(ref s) if s == "World"));

        // Now empty
        let result3 = poll_stream(&mut wasm_stream, &mut task_manager);
        assert!(matches!(result3, StreamPollResult::Pending));
    }
}

/// Summary: The WebAssembly Component Model async does NOT require the futures
/// crate or Rust's async/await. It has its own async primitives:
///
/// 1. `stream<T>` - for incremental value passing
/// 2. `future<T>` - for deferred single values
/// 3. `error-context` - for detailed error information
///
/// These are polled/waited on through the task manager and canonical built-ins
/// like:
/// - `task.wait` - wait for async operations
/// - `stream.read` / `stream.write` - stream operations
/// - `future.read` / `future.write` - future operations
///
/// The Rust Future trait is only needed if you want to integrate with Rust
/// async runtimes like tokio or async-std, which is optional.
pub struct ComponentModelAsyncSummary;
