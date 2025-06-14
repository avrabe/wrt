// Threading built-ins implementation for the WebAssembly Component Model
//
// This module implements the threading-related built-in functions:
// - threading.spawn: Spawn a new thread
// - threading.join: Join a thread (wait for its completion)
// - threading.sync: Create a synchronization primitive

use std::{boxed::Box, collections::HashMap, string::String, sync::Arc, vec::Vec};
#[cfg(feature = "std")]
use std::{
    boxed::Box,
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex, RwLock,
    },
    thread,
    vec::Vec,
};

use wrt_error::{kinds::ThreadingError, Error, Result};
#[cfg(feature = "std")]
use wrt_foundation::{builtin::BuiltinType, component_value::ComponentValue};

use super::BuiltinHandler;

/// Thread handle identifier
type ThreadId = u64;

/// Synchronization primitive identifier
type SyncId = u64;

/// Thread execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadState {
    /// Thread is running
    Running,
    /// Thread has completed successfully with a result
    Completed,
    /// Thread has terminated with an error
    Error,
}

/// Thread handle for tracking thread state and results
#[cfg(feature = "std")]
struct ThreadHandle {
    /// Thread join handle
    handle: Option<thread::JoinHandle<Result<Vec<ComponentValue>>>>,
    /// Thread state
    state: Arc<RwLock<ThreadState>>,
    /// Thread result (available when completed)
    result: Arc<RwLock<Option<Vec<ComponentValue>>>>,
    /// Thread error (available when error)
    error: Arc<RwLock<Option<String>>>,
}

/// Synchronization primitive types
enum SyncPrimitive {
    /// Mutex with optional data
    Mutex {
        /// Inner mutex
        lock: Mutex<Option<Vec<ComponentValue>>>,
    },
    /// Condition variable with a mutex
    CondVar {
        /// Inner mutex
        lock: Mutex<Option<Vec<ComponentValue>>>,
        /// Condition variable
        cvar: Condvar,
    },
    /// Read-write lock
    RwLock {
        /// Inner read-write lock
        lock: RwLock<Option<Vec<ComponentValue>>>,
    },
}

/// Thread management system
#[cfg(feature = "std")]
#[derive(Default)]
pub struct ThreadManager {
    /// Next available thread ID
    next_thread_id: AtomicU64,
    /// Map of thread ID to thread handle
    threads: RwLock<HashMap<ThreadId, ThreadHandle>>,
    /// Next available sync ID
    next_sync_id: AtomicU64,
    /// Map of sync ID to synchronization primitive
    sync_primitives: RwLock<HashMap<SyncId, SyncPrimitive>>,
}

#[cfg(feature = "std")]
impl ThreadManager {
    /// Create a new thread manager
    pub fn new() -> Self {
        Self {
            next_thread_id: AtomicU64::new(1),
            threads: RwLock::new(HashMap::new()),
            next_sync_id: AtomicU64::new(1),
            sync_primitives: RwLock::new(HashMap::new()),
        }
    }

    /// Spawn a new thread
    ///
    /// # Arguments
    ///
    /// * `function_id` - ID of the function to call
    /// * `args` - Arguments to pass to the function
    /// * `executor` - Function to execute the component function
    ///
    /// # Returns
    ///
    /// The thread ID of the spawned thread
    pub fn spawn<F>(
        &self,
        function_id: u32,
        args: Vec<ComponentValue>,
        executor: F,
    ) -> Result<ThreadId>
    where
        F: FnOnce(u32, Vec<ComponentValue>) -> Result<Vec<ComponentValue>> + Send + 'static,
    {
        // Get new thread ID
        let thread_id = self.next_thread_id.fetch_add(1, Ordering::SeqCst);

        // Create shared state
        let state = Arc::new(RwLock::new(ThreadState::Running));
        let result = Arc::new(RwLock::new(None));
        let error = Arc::new(RwLock::new(None));

        // Capture state for the thread
        let thread_state = state.clone();
        let thread_result = result.clone();
        let thread_error = error.clone();

        // Spawn the actual thread
        let handle = thread::spawn(move || {
            // Execute the function
            let fn_result = executor(function_id, args);

            match fn_result {
                Ok(values) => {
                    // Store the result
                    *thread_result.write().unwrap() = Some(values);
                    *thread_state.write().unwrap() = ThreadState::Completed;
                    Ok(thread_result.read().unwrap().clone().unwrap())
                }
                Err(e) => {
                    // Store the error
                    *thread_error.write().unwrap() = Some(e.to_string());
                    *thread_state.write().unwrap() = ThreadState::Error;
                    Err(e)
                }
            }
        });

        // Store the thread handle
        let thread_handle = ThreadHandle { handle: Some(handle), state, result, error };

        self.threads.write().unwrap().insert(thread_id, thread_handle);

        Ok(thread_id)
    }

    /// Join a thread (wait for it to complete)
    ///
    /// # Arguments
    ///
    /// * `thread_id` - ID of the thread to join
    ///
    /// # Returns
    ///
    /// The result of the thread execution
    pub fn join(&self, thread_id: ThreadId) -> Result<Vec<ComponentValue>> {
        // Find the thread
        let mut threads = self.threads.write().unwrap();
        let thread = threads.get_mut(&thread_id).ok_or_else(|| {
            Error::new(ThreadingError("Component not found"))
        })?;

        // Check if thread is already joined
        if thread.handle.is_none() {
            // Check state to see if it completed or had an error
            let state = *thread.state.read().unwrap();

            match state {
                ThreadState::Completed => {
                    // Return the cached result
                    let result =
                        thread.result.read().unwrap().clone().ok_or_else(|| {
                            Error::new(ThreadingError("Thread result unavailable"))
                        })?;
                    Ok(result)
                }
                ThreadState::Error => {
                    // Return the cached error
                    let err_msg = thread
                        .error
                        .read()
                        .unwrap()
                        .clone()
                        .unwrap_or_else(|| "Unknown thread error".to_string());
                    Err(Error::new(ThreadingError(err_msg)))
                }
                ThreadState::Running => {
                    // This shouldn't happen if handle is None
                    Err(Error::new(ThreadingError("Thread is still running but handle is missing")))
                }
            }
        } else {
            // Take the handle out to join it
            let handle = thread
                .handle
                .take()
                .ok_or_else(|| Error::new(ThreadingError("Thread handle unavailable")))?;

            // Join the thread
            match handle.join() {
                Ok(result) => result,
                Err(_) => {
                    // Thread panicked
                    *thread.state.write().unwrap() = ThreadState::Error;
                    *thread.error.write().unwrap() = Some("Thread panicked".to_string());
                    Err(Error::new(ThreadingError("Thread panicked during execution")))
                }
            }
        }
    }

    /// Check if a thread has completed
    ///
    /// # Arguments
    ///
    /// * `thread_id` - ID of the thread to check
    ///
    /// # Returns
    ///
    /// `true` if the thread has completed, `false` if it's still running
    pub fn is_thread_completed(&self, thread_id: ThreadId) -> Result<bool> {
        // Find the thread
        let threads = self.threads.read().unwrap();
        let thread = threads.get(&thread_id).ok_or_else(|| {
            Error::new(ThreadingError("Component not found"))
        })?;

        // Check the state
        let state = *thread.state.read().unwrap();
        Ok(state != ThreadState::Running)
    }

    /// Create a new mutex
    ///
    /// # Returns
    ///
    /// The ID of the created mutex
    pub fn create_mutex(&self) -> SyncId {
        let sync_id = self.next_sync_id.fetch_add(1, Ordering::SeqCst);

        let mutex = SyncPrimitive::Mutex { lock: Mutex::new(None) };

        self.sync_primitives.write().unwrap().insert(sync_id, mutex);
        sync_id
    }

    /// Create a new condition variable
    ///
    /// # Returns
    ///
    /// The ID of the created condition variable
    pub fn create_condvar(&self) -> SyncId {
        let sync_id = self.next_sync_id.fetch_add(1, Ordering::SeqCst);

        let condvar = SyncPrimitive::CondVar { lock: Mutex::new(None), cvar: Condvar::new() };

        self.sync_primitives.write().unwrap().insert(sync_id, condvar);
        sync_id
    }

    /// Create a new read-write lock
    ///
    /// # Returns
    ///
    /// The ID of the created read-write lock
    pub fn create_rwlock(&self) -> SyncId {
        let sync_id = self.next_sync_id.fetch_add(1, Ordering::SeqCst);

        let rwlock = SyncPrimitive::RwLock { lock: RwLock::new(None) };

        self.sync_primitives.write().unwrap().insert(sync_id, rwlock);
        sync_id
    }

    /// Lock a mutex
    ///
    /// # Arguments
    ///
    /// * `sync_id` - ID of the mutex to lock
    /// * `data` - Optional data to store in the mutex
    ///
    /// # Returns
    ///
    /// The previous data in the mutex, if any
    pub fn lock_mutex(
        &self,
        sync_id: SyncId,
        data: Option<Vec<ComponentValue>>,
    ) -> Result<Option<Vec<ComponentValue>>> {
        // Find the sync primitive
        let primitives = self.sync_primitives.read().unwrap();

        match primitives.get(&sync_id) {
            Some(SyncPrimitive::Mutex { lock }) => {
                // Lock the mutex
                let mut guard = lock.lock().unwrap();

                // Swap the data
                let previous = guard.take();
                *guard = data;

                Ok(previous)
            }
            Some(_) => {
                Err(Error::new(ThreadingError("Component not found")))
            }
            None => Err(Error::new(ThreadingError("Component not found"))),
        }
    }

    /// Wait on a condition variable
    ///
    /// # Arguments
    ///
    /// * `sync_id` - ID of the condition variable to wait on
    /// * `predicate` - Predicate to wait for
    ///
    /// # Returns
    ///
    /// The data stored in the condition variable
    pub fn wait_condvar(
        &self,
        sync_id: SyncId,
        predicate: Box<dyn Fn(&Option<Vec<ComponentValue>>) -> bool + Send>,
    ) -> Result<Option<Vec<ComponentValue>>> {
        // Find the sync primitive
        let primitives = self.sync_primitives.read().unwrap();

        match primitives.get(&sync_id) {
            Some(SyncPrimitive::CondVar { lock, cvar }) => {
                // Lock the mutex
                let mut guard = lock.lock().unwrap();

                // Wait until the predicate is satisfied
                while !predicate(&guard) {
                    guard = cvar.wait(guard).unwrap();
                }

                // Return the data
                Ok(guard.clone())
            }
            Some(_) => Err(Error::new(ThreadingError(format!(
                "Sync ID {} is not a condition variable",
                sync_id
            )))),
            None => Err(Error::new(ThreadingError("Component not found"))),
        }
    }

    /// Signal a condition variable
    ///
    /// # Arguments
    ///
    /// * `sync_id` - ID of the condition variable to signal
    /// * `data` - Optional data to store in the condition variable
    ///
    /// # Returns
    ///
    /// The previous data in the condition variable, if any
    pub fn signal_condvar(
        &self,
        sync_id: SyncId,
        data: Option<Vec<ComponentValue>>,
    ) -> Result<Option<Vec<ComponentValue>>> {
        // Find the sync primitive
        let primitives = self.sync_primitives.read().unwrap();

        match primitives.get(&sync_id) {
            Some(SyncPrimitive::CondVar { lock, cvar }) => {
                // Lock the mutex
                let mut guard = lock.lock().unwrap();

                // Swap the data
                let previous = guard.take();
                *guard = data;

                // Signal one waiting thread
                cvar.notify_one();

                Ok(previous)
            }
            Some(_) => Err(Error::new(ThreadingError(format!(
                "Sync ID {} is not a condition variable",
                sync_id
            )))),
            None => Err(Error::new(ThreadingError("Component not found"))),
        }
    }

    /// Acquire a read lock
    ///
    /// # Arguments
    ///
    /// * `sync_id` - ID of the read-write lock to read from
    ///
    /// # Returns
    ///
    /// The data in the read-write lock, if any
    pub fn read_rwlock(&self, sync_id: SyncId) -> Result<Option<Vec<ComponentValue>>> {
        // Find the sync primitive
        let primitives = self.sync_primitives.read().unwrap();

        match primitives.get(&sync_id) {
            Some(SyncPrimitive::RwLock { lock }) => {
                // Acquire a read lock
                let guard = lock.read().unwrap();

                // Return a clone of the data
                Ok(guard.clone())
            }
            Some(_) => Err(Error::new(ThreadingError(format!(
                "Sync ID {} is not a read-write lock",
                sync_id
            )))),
            None => Err(Error::new(ThreadingError("Component not found"))),
        }
    }

    /// Acquire a write lock
    ///
    /// # Arguments
    ///
    /// * `sync_id` - ID of the read-write lock to write to
    /// * `data` - Optional data to store in the read-write lock
    ///
    /// # Returns
    ///
    /// The previous data in the read-write lock, if any
    pub fn write_rwlock(
        &self,
        sync_id: SyncId,
        data: Option<Vec<ComponentValue>>,
    ) -> Result<Option<Vec<ComponentValue>>> {
        // Find the sync primitive
        let primitives = self.sync_primitives.read().unwrap();

        match primitives.get(&sync_id) {
            Some(SyncPrimitive::RwLock { lock }) => {
                // Acquire a write lock
                let mut guard = lock.write().unwrap();

                // Swap the data
                let previous = guard.take();
                *guard = data;

                Ok(previous)
            }
            Some(_) => Err(Error::new(ThreadingError(format!(
                "Sync ID {} is not a read-write lock",
                sync_id
            )))),
            None => Err(Error::new(ThreadingError("Component not found"))),
        }
    }
}

/// Handler for threading.spawn built-in
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct ThreadingSpawnHandler {
    /// Thread manager
    thread_manager: Arc<ThreadManager>,
    /// Function to execute the component function
    executor: Arc<dyn Fn(u32, Vec<ComponentValue>) -> Result<Vec<ComponentValue>> + Send + Sync>,
}

#[cfg(feature = "std")]
impl ThreadingSpawnHandler {
    /// Create a new threading.spawn handler
    ///
    /// # Arguments
    ///
    /// * `thread_manager` - Thread manager to use
    /// * `executor` - Function to execute the component function
    pub fn new(
        thread_manager: Arc<ThreadManager>,
        executor: Arc<
            dyn Fn(u32, Vec<ComponentValue>) -> Result<Vec<ComponentValue>> + Send + Sync,
        >,
    ) -> Self {
        Self { thread_manager, executor }
    }
}

#[cfg(feature = "std")]
impl BuiltinHandler for ThreadingSpawnHandler {
    fn builtin_type(&self) -> BuiltinType {
        BuiltinType::ThreadingSpawn
    }

    fn execute(&self, args: &[ComponentValue]) -> Result<Vec<ComponentValue>> {
        // Validate arguments
        if args.len() < 1 {
            return Err(Error::new(ThreadingError("threading.spawn requires at least 1 argument")));
        }

        // Extract function ID
        let function_id = match args[0] {
            ComponentValue::U32(id) => id,
            _ => {
                return Err(Error::new(ThreadingError(
                    "threading.spawn first argument must be a function ID",
                )));
            }
        };

        // Extract function arguments
        let function_args = args[1..].to_vec();

        // Create a clone of the executor
        let executor = self.executor.clone();

        // Spawn the thread
        let thread_id =
            self.thread_manager
                .spawn(function_id, function_args, move |id, args| executor(id, args))?;

        // Return the thread ID
        Ok(vec![ComponentValue::U64(thread_id)])
    }

    fn clone_handler(&self) -> Box<dyn BuiltinHandler> {
        Box::new(self.clone())
    }
}

/// Handler for threading.join built-in
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct ThreadingJoinHandler {
    /// Thread manager
    thread_manager: Arc<ThreadManager>,
}

#[cfg(feature = "std")]
impl ThreadingJoinHandler {
    /// Create a new threading.join handler
    ///
    /// # Arguments
    ///
    /// * `thread_manager` - Thread manager to use
    pub fn new(thread_manager: Arc<ThreadManager>) -> Self {
        Self { thread_manager }
    }
}

#[cfg(feature = "std")]
impl BuiltinHandler for ThreadingJoinHandler {
    fn builtin_type(&self) -> BuiltinType {
        BuiltinType::ThreadingJoin
    }

    fn execute(&self, args: &[ComponentValue]) -> Result<Vec<ComponentValue>> {
        // Validate arguments
        if args.len() != 1 {
            return Err(Error::new(ThreadingError("threading.join requires exactly 1 argument")));
        }

        // Extract thread ID
        let thread_id = match args[0] {
            ComponentValue::U64(id) => id,
            _ => {
                return Err(Error::new(ThreadingError(
                    "threading.join argument must be a thread ID",
                )))
            }
        };

        // Join the thread
        self.thread_manager.join(thread_id)
    }

    fn clone_handler(&self) -> Box<dyn BuiltinHandler> {
        Box::new(self.clone())
    }
}

/// Handler for threading.sync built-in
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct ThreadingSyncHandler {
    /// Thread manager
    thread_manager: Arc<ThreadManager>,
    /// Flag to track if this is a no-op in no_std mode
    no_std_mode: AtomicBool,
}

#[cfg(feature = "std")]
impl ThreadingSyncHandler {
    /// Create a new threading.sync handler
    ///
    /// # Arguments
    ///
    /// * `thread_manager` - Thread manager to use
    pub fn new(thread_manager: Arc<ThreadManager>) -> Self {
        Self { thread_manager, no_std_mode: AtomicBool::new(false) }
    }
}

#[cfg(feature = "std")]
impl BuiltinHandler for ThreadingSyncHandler {
    fn builtin_type(&self) -> BuiltinType {
        BuiltinType::ThreadingSync
    }

    fn execute(&self, args: &[ComponentValue]) -> Result<Vec<ComponentValue>> {
        // Check if we're in no_std mode (should never happen if properly feature-gated)
        if self.no_std_mode.load(Ordering::Relaxed) {
            return Err(Error::new(ThreadingError("Threading is not supported in no_std mode")));
        }

        // Validate arguments
        if args.len() < 1 {
            return Err(Error::new(ThreadingError("threading.sync requires at least 1 argument")));
        }

        // Extract operation type
        let op_type = match &args[0] {
            ComponentValue::String(s) => s.as_str(),
            _ => {
                return Err(Error::new(ThreadingError(
                    "threading.sync first argument must be a string",
                )))
            }
        };

        match op_type {
            "create-mutex" => {
                // Create a new mutex
                let mutex_id = self.thread_manager.create_mutex();
                Ok(vec![ComponentValue::U64(mutex_id)])
            }
            "lock-mutex" => {
                // Lock a mutex
                if args.len() < 2 {
                    return Err(Error::new(ThreadingError("lock-mutex requires a mutex ID")));
                }

                let mutex_id = match args[1] {
                    ComponentValue::U64(id) => id,
                    _ => {
                        return Err(Error::new(ThreadingError(
                            "lock-mutex requires a mutex ID as second argument",
                        )));
                    }
                };

                // Extract optional data
                let data = if args.len() > 2 { Some(args[2..].to_vec()) } else { None };

                // Lock the mutex
                let previous = self.thread_manager.lock_mutex(mutex_id, data)?;

                // Return the previous data
                Ok(previous.unwrap_or_default())
            }
            "create-condvar" => {
                // Create a new condition variable
                let condvar_id = self.thread_manager.create_condvar();
                Ok(vec![ComponentValue::U64(condvar_id)])
            }
            "wait-condvar" => {
                // Wait on a condition variable
                if args.len() < 2 {
                    return Err(Error::new(ThreadingError("wait-condvar requires a condvar ID")));
                }

                let condvar_id = match args[1] {
                    ComponentValue::U64(id) => id,
                    _ => {
                        return Err(Error::new(ThreadingError(
                            "wait-condvar requires a condvar ID as second argument",
                        )));
                    }
                };

                // Simple predicate that always waits until signaled
                let predicate = Box::new(|_: &Option<Vec<ComponentValue>>| true);

                // Wait on the condition variable
                let data = self.thread_manager.wait_condvar(condvar_id, predicate)?;

                // Return the data
                Ok(data.unwrap_or_default())
            }
            "signal-condvar" => {
                // Signal a condition variable
                if args.len() < 2 {
                    return Err(Error::new(ThreadingError("signal-condvar requires a condvar ID")));
                }

                let condvar_id = match args[1] {
                    ComponentValue::U64(id) => id,
                    _ => {
                        return Err(Error::new(ThreadingError(
                            "signal-condvar requires a condvar ID as second argument",
                        )));
                    }
                };

                // Extract optional data
                let data = if args.len() > 2 { Some(args[2..].to_vec()) } else { None };

                // Signal the condition variable
                let previous = self.thread_manager.signal_condvar(condvar_id, data)?;

                // Return the previous data
                Ok(previous.unwrap_or_default())
            }
            "create-rwlock" => {
                // Create a new read-write lock
                let rwlock_id = self.thread_manager.create_rwlock();
                Ok(vec![ComponentValue::U64(rwlock_id)])
            }
            "read-rwlock" => {
                // Read from a read-write lock
                if args.len() < 2 {
                    return Err(Error::new(ThreadingError("read-rwlock requires a rwlock ID")));
                }

                let rwlock_id = match args[1] {
                    ComponentValue::U64(id) => id,
                    _ => {
                        return Err(Error::new(ThreadingError(
                            "read-rwlock requires a rwlock ID as second argument",
                        )));
                    }
                };

                // Read from the read-write lock
                let data = self.thread_manager.read_rwlock(rwlock_id)?;

                // Return the data
                Ok(data.unwrap_or_default())
            }
            "write-rwlock" => {
                // Write to a read-write lock
                if args.len() < 2 {
                    return Err(Error::new(ThreadingError("write-rwlock requires a rwlock ID")));
                }

                let rwlock_id = match args[1] {
                    ComponentValue::U64(id) => id,
                    _ => {
                        return Err(Error::new(ThreadingError(
                            "write-rwlock requires a rwlock ID as second argument",
                        )));
                    }
                };

                // Extract optional data
                let data = if args.len() > 2 { Some(args[2..].to_vec()) } else { None };

                // Write to the read-write lock
                let previous = self.thread_manager.write_rwlock(rwlock_id, data)?;

                // Return the previous data
                Ok(previous.unwrap_or_default())
            }
            _ => Err(Error::new(ThreadingError(format!(
                "Unknown threading.sync operation: {}",
                op_type
            )))),
        }
    }

    fn clone_handler(&self) -> Box<dyn BuiltinHandler> {
        Box::new(self.clone())
    }
}

/// Create handlers for threading built-ins
#[cfg(feature = "std")]
pub fn create_threading_handlers(
    executor: Arc<dyn Fn(u32, Vec<ComponentValue>) -> Result<Vec<ComponentValue>> + Send + Sync>,
) -> Vec<Box<dyn BuiltinHandler>> {
    let thread_manager = Arc::new(ThreadManager::new());

    vec![
        Box::new(ThreadingSpawnHandler::new(thread_manager.clone(), executor)),
        Box::new(ThreadingJoinHandler::new(thread_manager.clone())),
        Box::new(ThreadingSyncHandler::new(thread_manager)),
    ]
}

/// Create no-op threading handlers for no_std environments
#[cfg(not(feature = "std"))]
pub fn create_threading_handlers(
    _: Arc<dyn Fn(u32, Vec<ComponentValue>) -> Result<Vec<ComponentValue>> + Send + Sync>,
) -> Vec<Box<dyn BuiltinHandler>> {
    // Threading is not supported in no_std mode
    Vec::new()
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    // Helper function for executing a "function" in tests
    fn test_executor(function_id: u32, args: Vec<ComponentValue>) -> Result<Vec<ComponentValue>> {
        match function_id {
            // Simple echo function
            1 => Ok(args),

            // Function that sleeps for a while then returns
            2 => {
                // Extract sleep duration in milliseconds
                let sleep_ms = if args.len() > 0 {
                    match args[0] {
                        ComponentValue::U32(ms) => ms,
                        _ => 100, // Default
                    }
                } else {
                    100 // Default
                };

                // Sleep
                sleep(Duration::from_millis(sleep_ms as u64));

                // Return a simple result
                Ok(vec![ComponentValue::String("Slept".to_string()), ComponentValue::U32(sleep_ms)])
            }

            // Function that returns an error
            3 => Err(Error::new("Test error")),

            // Unknown function
            _ => Err(Error::new(ThreadingError("Component not found"))),
        }
    }

    #[test]
    fn test_thread_manager_spawn_and_join() {
        let manager = ThreadManager::new();

        // Spawn a thread
        let thread_id = manager
            .spawn(1, vec![ComponentValue::String("Hello".to_string())], test_executor)
            .unwrap();

        // Join it
        let result = manager.join(thread_id).unwrap();

        // Verify result
        assert_eq!(result, vec![ComponentValue::String("Hello".to_string())]);
    }

    #[test]
    fn test_thread_manager_async_join() {
        let manager = ThreadManager::new();

        // Spawn a thread that sleeps
        let thread_id = manager.spawn(2, vec![ComponentValue::U32(50)], test_executor).unwrap();

        // Check if it's completed (should be running)
        assert!(!manager.is_thread_completed(thread_id).unwrap());

        // Sleep a bit longer than the thread
        sleep(Duration::from_millis(100));

        // Now it should be completed
        assert!(manager.is_thread_completed(thread_id).unwrap());

        // Join it
        let result = manager.join(thread_id).unwrap();

        // Verify result
        assert_eq!(
            result,
            vec![ComponentValue::String("Slept".to_string()), ComponentValue::U32(50)]
        );
    }

    #[test]
    fn test_thread_manager_error() {
        let manager = ThreadManager::new();

        // Spawn a thread that returns an error
        let thread_id = manager.spawn(3, vec![], test_executor).unwrap();

        // Wait for it to complete
        sleep(Duration::from_millis(10));

        // Join it (should return an error)
        let result = manager.join(thread_id);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Test error");
    }

    #[test]
    fn test_thread_spawn_handler() {
        let thread_manager = Arc::new(ThreadManager::new());
        let executor = Arc::new(test_executor);
        let handler = ThreadingSpawnHandler::new(thread_manager.clone(), executor);

        // Test with valid arguments
        let args = vec![
            ComponentValue::U32(1),                     // Function ID
            ComponentValue::String("Test".to_string()), // Function argument
        ];

        let result = handler.execute(&args).unwrap();
        assert_eq!(result.len(), 1);

        let thread_id = match result[0] {
            ComponentValue::U64(id) => id,
            _ => panic!("Expected U64 result"),
        };

        // Join the thread
        let join_result = thread_manager.join(thread_id).unwrap();
        assert_eq!(join_result, vec![ComponentValue::String("Test".to_string())]);

        // Test with invalid arguments
        let args = vec![ComponentValue::String("invalid".to_string())];
        assert!(handler.execute(&args).is_err());

        // Test with insufficient arguments
        let args = vec![];
        assert!(handler.execute(&args).is_err());
    }

    #[test]
    fn test_thread_join_handler() {
        let thread_manager = Arc::new(ThreadManager::new());
        let executor = Arc::new(test_executor);

        // Spawn a thread
        let thread_id = thread_manager
            .spawn(1, vec![ComponentValue::String("Test".to_string())], executor)
            .unwrap();

        // Create a join handler
        let handler = ThreadingJoinHandler::new(thread_manager);

        // Test joining the thread
        let result = handler.execute(&[ComponentValue::U64(thread_id)]).unwrap();
        assert_eq!(result, vec![ComponentValue::String("Test".to_string())]);

        // Test with invalid arguments
        let args = vec![ComponentValue::String("invalid".to_string())];
        assert!(handler.execute(&args).is_err());

        // Test with insufficient arguments
        let args = vec![];
        assert!(handler.execute(&args).is_err());
    }

    #[test]
    fn test_thread_sync_handler() {
        let thread_manager = Arc::new(ThreadManager::new());
        let handler = ThreadingSyncHandler::new(thread_manager);

        // Test creating a mutex
        let result =
            handler.execute(&[ComponentValue::String("create-mutex".to_string())]).unwrap();
        assert_eq!(result.len(), 1);

        let mutex_id = match result[0] {
            ComponentValue::U64(id) => id,
            _ => panic!("Expected U64 result"),
        };

        // Test locking the mutex
        let result = handler
            .execute(&[
                ComponentValue::String("lock-mutex".to_string()),
                ComponentValue::U64(mutex_id),
                ComponentValue::String("data".to_string()),
            ])
            .unwrap();

        // First lock should return empty result
        assert_eq!(result.len(), 0);

        // Test locking again (should return previous data)
        let result = handler
            .execute(&[
                ComponentValue::String("lock-mutex".to_string()),
                ComponentValue::U64(mutex_id),
                ComponentValue::String("new-data".to_string()),
            ])
            .unwrap();

        assert_eq!(result, vec![ComponentValue::String("data".to_string())]);

        // Test creating a condvar
        let result =
            handler.execute(&[ComponentValue::String("create-condvar".to_string())]).unwrap();
        let condvar_id = match result[0] {
            ComponentValue::U64(id) => id,
            _ => panic!("Expected U64 result"),
        };

        // Test signaling the condvar
        handler
            .execute(&[
                ComponentValue::String("signal-condvar".to_string()),
                ComponentValue::U64(condvar_id),
                ComponentValue::String("signal-data".to_string()),
            ])
            .unwrap();

        // Test creating an rwlock
        let result =
            handler.execute(&[ComponentValue::String("create-rwlock".to_string())]).unwrap();
        let rwlock_id = match result[0] {
            ComponentValue::U64(id) => id,
            _ => panic!("Expected U64 result"),
        };

        // Test writing to the rwlock
        handler
            .execute(&[
                ComponentValue::String("write-rwlock".to_string()),
                ComponentValue::U64(rwlock_id),
                ComponentValue::String("rwlock-data".to_string()),
            ])
            .unwrap();

        // Test reading from the rwlock
        let result = handler
            .execute(&[
                ComponentValue::String("read-rwlock".to_string()),
                ComponentValue::U64(rwlock_id),
            ])
            .unwrap();

        assert_eq!(result, vec![ComponentValue::String("rwlock-data".to_string())]);

        // Test with invalid operation
        let args = vec![ComponentValue::String("invalid-op".to_string())];
        assert!(handler.execute(&args).is_err());
    }

    #[test]
    fn test_create_threading_handlers() {
        let executor = Arc::new(test_executor);
        let handlers = create_threading_handlers(executor);

        assert_eq!(handlers.len(), 3);
        assert_eq!(handlers[0].builtin_type(), BuiltinType::ThreadingSpawn);
        assert_eq!(handlers[1].builtin_type(), BuiltinType::ThreadingJoin);
        assert_eq!(handlers[2].builtin_type(), BuiltinType::ThreadingSync);
    }
}
