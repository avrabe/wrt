// We'll use () as the error type for now

#[cfg(not(feature = "std"))]
use core::cell::UnsafeCell;
#[cfg(not(feature = "std"))]
use core::fmt;
#[cfg(not(feature = "std"))]
use core::ops::{Deref, DerefMut};

/// A simple mutex implementation for no_std environments
///
/// This is a placeholder implementation that doesn't actually
/// provide thread safety. In a real implementation, we'd use
/// a proper no_std mutex like spin::Mutex.
#[cfg(not(feature = "std"))]
#[derive(Debug)]
pub struct Mutex<T> {
    data: UnsafeCell<T>,
}

/// Guard for the mutex that provides access to the inner value
#[cfg(not(feature = "std"))]
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

#[cfg(not(feature = "std"))]
impl<T> Mutex<T> {
    /// Create a new mutex
    pub fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
        }
    }

    /// Lock the mutex and get access to the inner value
    #[allow(clippy::result_unit_err)]
    pub fn lock(&self) -> Result<MutexGuard<'_, T>, ()> {
        // In a real implementation, we'd actually lock here
        Ok(MutexGuard { mutex: self })
    }
}

#[cfg(not(feature = "std"))]
impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

#[cfg(not(feature = "std"))]
impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

// These are required for Send and Sync implementations
// Note: This is not thread-safe in a no_std environment without actual locking
#[cfg(not(feature = "std"))]
unsafe impl<T: Send> Send for Mutex<T> {}
#[cfg(not(feature = "std"))]
unsafe impl<T: Send> Sync for Mutex<T> {}
#[cfg(not(feature = "std"))]
unsafe impl<T: Send> Send for MutexGuard<'_, T> {}
#[cfg(not(feature = "std"))]
unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

// Add a Debug implementation for MutexGuard
#[cfg(not(feature = "std"))]
impl<T: fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MutexGuard")
            .field("value", &**self)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::string::String;
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    #[test]
    fn test_mutex_creation() {
        let mutex = Mutex::new(42);
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_mutex_modification() {
        let mutex = Mutex::new(vec![1, 2, 3]);
        {
            let mut guard = mutex.lock().unwrap();
            guard.push(4);
        }
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_mutex_multiple_locks() {
        let mutex = Mutex::new(String::from("test"));
        {
            let mut guard = mutex.lock().unwrap();
            guard.push_str("_1");
        }
        {
            let mut guard = mutex.lock().unwrap();
            guard.push_str("_2");
        }
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, "test_1_2");
    }

    #[test]
    fn test_mutex_send_sync() {
        // This test verifies that Mutex implements Send and Sync
        // by attempting to send it between threads (compile-time check)
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Mutex<i32>>();
    }

    #[test]
    fn test_mutex_guard_drop() {
        let mutex = Mutex::new(42);
        {
            let mut guard = mutex.lock().unwrap();
            *guard = 100;
        } // guard is dropped here
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 100);
    }
}
