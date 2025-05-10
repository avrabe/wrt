// #![allow(unsafe_code)] // Allow unsafe for UnsafeCell, atomics, and Send/Sync impls

use core::cell::UnsafeCell;
use core::fmt;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

/// A simple, `no_std` compatible Read-Write Lock using atomics.
///
/// This is a reader-preference lock. Writers may starve if readers constantly hold the lock.
/// WARNING: This is a basic implementation. It does not handle potential deadlocks involving multiple locks
/// or priority inversion. Use with caution.
const WRITE_LOCK_STATE: usize = usize::MAX;

/// A non-blocking, atomic read-write lock for `no_std` environments.
///
/// This `RwLock` is designed to be efficient and suitable for environments where
/// `std` is not available, making it suitable for use in embedded systems and other `no_std` contexts.
///
/// # Examples
///
/// ```
/// use wrt_sync::WrtRwLock;
///
/// // Create a new RwLock
/// let lock = WrtRwLock::new(42);
///
/// // Acquire a read lock
/// let reader = lock.read();
/// assert_eq!(*reader, 42);
///
/// // Release the read lock by dropping the guard
/// drop(reader);
///
/// // Acquire a write lock
/// let mut writer = lock.write();
/// *writer = 100;
///
/// // Release the write lock by dropping the guard
/// drop(writer);
///
/// // Verify the new value with another reader
/// let reader = lock.read();
/// assert_eq!(*reader, 100);
/// ```
pub struct WrtRwLock<T: ?Sized> {
    /// Atomically tracks the lock state.
    /// Encoding:
    /// - 0: Unlocked
    /// - `usize::MAX`: Write-locked
    /// - n (`1..usize::MAX - 1`): Read-locked by n readers
    state: AtomicUsize,
    data: UnsafeCell<T>,
}

/// A guard that provides read access to the data protected by a `WrtRwLock`.
#[clippy::has_significant_drop]
pub struct WrtRwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a WrtRwLock<T>,
}

/// A guard that provides write access to the data protected by a `WrtRwLock`.
#[clippy::has_significant_drop]
pub struct WrtRwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a WrtRwLock<T>,
}

// Allow the lock to be shared across threads.
// Safety: Requires correct implementation of locking mechanisms.
unsafe impl<T: ?Sized + Send + Sync> Send for WrtRwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for WrtRwLock<T> {}

impl<T> WrtRwLock<T> {
    /// Creates a new `WrtRwLock` protecting the given data.
    #[inline]
    pub const fn new(data: T) -> Self {
        WrtRwLock {
            state: AtomicUsize::new(0), // Start unlocked
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> WrtRwLock<T> {
    /// Acquires a read lock, spinning until available.
    #[inline]
    pub fn read(&self) -> WrtRwLockReadGuard<'_, T> {
        loop {
            let current_state = self.state.load(Ordering::Relaxed);
            // Check if write-locked
            if current_state != WRITE_LOCK_STATE {
                // Attempt to increment reader count
                match self.state.compare_exchange_weak(
                    current_state,
                    current_state + 1,
                    Ordering::Acquire, // Ensure reads happen after lock acquisition
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return WrtRwLockReadGuard { lock: self },
                    Err(_) => continue, // State changed, retry loop
                }
            }
            // If write-locked, spin
            spin_loop();
        }
    }

    /// Acquires a write lock, spinning until available.
    #[inline]
    pub fn write(&self) -> WrtRwLockWriteGuard<'_, T> {
        loop {
            // Attempt to acquire write lock if currently unlocked (state == 0)
            match self.state.compare_exchange_weak(
                0, // Only try if unlocked
                WRITE_LOCK_STATE,
                Ordering::Acquire, // Ensure writes happen after lock acquisition
                Ordering::Relaxed,
            ) {
                Ok(_) => return WrtRwLockWriteGuard { lock: self },
                Err(current_state) => {
                    // If it failed because it was already locked (read or write), spin.
                    // If current_state was 0 but compare_exchange failed spuriously,
                    // the loop retries anyway.
                    if current_state != 0 {
                        spin_loop();
                    }
                    // Continue loop regardless to retry compare_exchange
                }
            }
        }
    }

    /// Attempts to acquire a read lock without blocking.
    #[inline]
    pub fn try_read(&self) -> Option<WrtRwLockReadGuard<'_, T>> {
        let current_state = self.state.load(Ordering::Relaxed);

        if current_state == WRITE_LOCK_STATE {
            None // Write locked
        } else {
            // Attempt to increment reader count if not write-locked.
            // The `writer_waiting` flag is not strictly necessary to check here for `try_read`,
            // as `try_read` should succeed even if a writer is waiting, as per typical RwLock semantics.
            // A writer trying to acquire the lock will wait for readers to release.
            match self.state.compare_exchange(
                current_state,     // Expected: not WRITE_LOCK_STATE
                current_state + 1, // Increment reader count
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => Some(WrtRwLockReadGuard { lock: self }),
                Err(_) => None, // State changed, CAS failed
            }
        }
    }

    /// Attempts to acquire a write lock without blocking.
    #[inline]
    pub fn try_write(&self) -> Option<WrtRwLockWriteGuard<'_, T>> {
        match self.state.compare_exchange(
            // Use strong exchange for try_ versions
            0,
            WRITE_LOCK_STATE,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => Some(WrtRwLockWriteGuard { lock: self }),
            Err(_) => None, // Failed (already locked or state changed)
        }
    }
}

// Read Guard Implementation

impl<T: ?Sized> Deref for WrtRwLockReadGuard<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety: Guard ensures read lock is held.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for WrtRwLockReadGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // Decrement reader count.
        // Release ordering ensures preceding reads are visible before the lock is released (partially).
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

// Write Guard Implementation

impl<T: ?Sized> Deref for WrtRwLockWriteGuard<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety: Guard ensures write lock is held.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for WrtRwLockWriteGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: Guard ensures write lock is held.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for WrtRwLockWriteGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // Set state back to unlocked (0).
        // Release ordering ensures preceding writes are visible before the lock is released.
        self.lock.state.store(0, Ordering::Release);
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for WrtRwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Attempt a non-blocking read for Debug representation if possible,
        // otherwise indicate locked status. Avoids deadlocking Debug.
        if let Some(guard) = self.try_read() {
            f.debug_struct("WrtRwLock").field("data", &&*guard).finish()
        } else {
            // Could be write-locked or contended.
            let state = self.state.load(Ordering::Relaxed);
            let mut ds = f.debug_struct("WrtRwLock");
            if state == WRITE_LOCK_STATE {
                ds.field("state", &"WriteLocked");
            } else if state == 0 {
                ds.field("state", &"Unlocked(contended)");
            } else {
                ds.field("state", &format_args!("ReadLocked({state}) (contended)"));
            }
            ds.field("data", &"<locked>");
            ds.finish()
        }
    }
}

// ======= Parking-based Implementation (for std environments) =======
#[cfg(feature = "std")]
pub mod parking_impl {
    use crate::prelude::{
        fmt as CoreFmt, Arc, AtomicBool, AtomicUsize, Deref as CoreDeref, DerefMut as CoreDerefMut,
        Ordering, UnsafeCell as CoreUnsafeCell, Vec,
    };
    use core::hint::spin_loop;
    use std::sync::RwLock as StdRwLock;
    use std::thread;

    const WRITE_LOCK_STATE: usize = usize::MAX;

    #[derive(Debug)]
    pub struct PoisonError(String);

    impl<Guard> From<std::sync::PoisonError<Guard>> for PoisonError {
        fn from(err: std::sync::PoisonError<Guard>) -> Self {
            PoisonError(err.to_string())
        }
    }

    impl CoreFmt::Display for PoisonError {
        fn fmt(&self, f: &mut CoreFmt::Formatter<'_>) -> CoreFmt::Result {
            write!(f, "lock poisoned: {}", self.0)
        }
    }

    pub struct WrtParkingRwLock<T: ?Sized> {
        state: AtomicUsize,
        writer_waiting: AtomicBool,
        waiters: Arc<WaitQueue>,
        data: CoreUnsafeCell<T>,
    }

    struct WaitQueue {
        readers: StdRwLock<Vec<thread::Thread>>,
        writer: StdRwLock<Option<thread::Thread>>,
    }

    impl WaitQueue {
        fn new() -> Self {
            WaitQueue { readers: StdRwLock::new(Vec::new()), writer: StdRwLock::new(None) }
        }

        fn register_reader(&self) -> Result<(), PoisonError> {
            self.readers.write()?.push(thread::current());
            Ok(())
        }

        fn register_writer(&self) -> Result<bool, PoisonError> {
            let mut writer_guard = self.writer.write()?;
            if writer_guard.is_none() {
                *writer_guard = Some(thread::current());
                Ok(true)
            } else {
                Ok(false)
            }
        }

        fn unregister_writer(&self) -> Result<(), PoisonError> {
            let mut writer_guard = self.writer.write()?;
            if let Some(writer_thread) = writer_guard.take() {
                if writer_thread.id() == thread::current().id() {
                    // Successfully unregistered
                } else {
                    *writer_guard = Some(writer_thread);
                }
            }
            Ok(())
        }

        fn wake_writer(&self) -> Result<(), PoisonError> {
            if let Some(writer_thread) = self.writer.write()?.take() {
                writer_thread.unpark();
            }
            Ok(())
        }

        fn wake_readers(&self) -> Result<(), PoisonError> {
            let readers_guard = self.readers.read()?;
            for reader_thread in readers_guard.iter() {
                reader_thread.unpark();
            }
            Ok(())
        }
    }

    unsafe impl<T: ?Sized + Send + Sync> Send for WrtParkingRwLock<T> {}
    unsafe impl<T: ?Sized + Send + Sync> Sync for WrtParkingRwLock<T> {}

    pub struct WrtParkingRwLockReadGuard<'a, T: ?Sized + 'a> {
        lock: &'a WrtParkingRwLock<T>,
    }

    pub struct WrtParkingRwLockWriteGuard<'a, T: ?Sized + 'a> {
        lock: &'a WrtParkingRwLock<T>,
    }

    impl<T> WrtParkingRwLock<T> {
        pub fn new(data: T) -> Self {
            Self {
                state: AtomicUsize::new(0),
                writer_waiting: AtomicBool::new(false),
                waiters: Arc::new(WaitQueue::new()),
                data: CoreUnsafeCell::new(data),
            }
        }
    }

    impl<T: ?Sized> WrtParkingRwLock<T> {
        pub fn read(&self) -> Result<WrtParkingRwLockReadGuard<'_, T>, PoisonError> {
            loop {
                if self.writer_waiting.load(Ordering::Acquire) {
                    self.waiters.register_reader()?;
                    thread::park();
                    continue;
                }

                let current_state = self.state.load(Ordering::Relaxed);
                if current_state != WRITE_LOCK_STATE {
                    match self.state.compare_exchange(
                        current_state,
                        current_state + 1,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => return Ok(WrtParkingRwLockReadGuard { lock: self }),
                        Err(_) => {}
                    }
                }
                self.waiters.register_reader()?;
                thread::park();
            }
        }

        pub fn write(&self) -> Result<WrtParkingRwLockWriteGuard<'_, T>, PoisonError> {
            loop {
                if !self.writer_waiting.swap(true, Ordering::Acquire) {
                    loop {
                        match self.state.compare_exchange(
                            0,
                            WRITE_LOCK_STATE,
                            Ordering::Acquire,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => {
                                return Ok(WrtParkingRwLockWriteGuard { lock: self });
                            }
                            Err(s) => {
                                if s != 0 {
                                    spin_loop();
                                }
                            }
                        }
                    }
                } else if self.waiters.register_writer()? {
                    loop {
                        match self.state.compare_exchange(
                            0,
                            WRITE_LOCK_STATE,
                            Ordering::Acquire,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => {
                                self.waiters.unregister_writer()?;
                                return Ok(WrtParkingRwLockWriteGuard { lock: self });
                            }
                            Err(s) => {
                                if s != 0 {
                                    thread::park();
                                } else {
                                    spin_loop();
                                }
                            }
                        }
                    }
                } else {
                    thread::park();
                }
                spin_loop();
            }
        }

        pub fn try_read(&self) -> Option<WrtParkingRwLockReadGuard<'_, T>> {
            let current_state = self.state.load(Ordering::Relaxed);

            if current_state == WRITE_LOCK_STATE {
                None
            } else {
                match self.state.compare_exchange(
                    current_state,
                    current_state + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => Some(WrtParkingRwLockReadGuard { lock: self }),
                    Err(_) => None,
                }
            }
        }

        pub fn try_write(&self) -> Option<WrtParkingRwLockWriteGuard<'_, T>> {
            if self
                .writer_waiting
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                return None;
            }

            match self.state.compare_exchange(
                0,
                WRITE_LOCK_STATE,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => Some(WrtParkingRwLockWriteGuard { lock: self }),
                Err(_) => {
                    self.writer_waiting.store(false, Ordering::Release);
                    None
                }
            }
        }
    }

    impl<T: ?Sized> CoreDeref for WrtParkingRwLockReadGuard<'_, T> {
        type Target = T;
        #[inline]
        fn deref(&self) -> &Self::Target {
            unsafe { &*self.lock.data.get() }
        }
    }

    impl<T: ?Sized> Drop for WrtParkingRwLockReadGuard<'_, T> {
        #[inline]
        fn drop(&mut self) {
            let prev_state = self.lock.state.fetch_sub(1, Ordering::Release);
            if prev_state == 1 && self.lock.writer_waiting.load(Ordering::Acquire) {
                let _ = self.lock.waiters.wake_writer();
            }
        }
    }

    impl<T: ?Sized> CoreDeref for WrtParkingRwLockWriteGuard<'_, T> {
        type Target = T;
        #[inline]
        fn deref(&self) -> &Self::Target {
            unsafe { &*self.lock.data.get() }
        }
    }

    impl<T: ?Sized> CoreDerefMut for WrtParkingRwLockWriteGuard<'_, T> {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            unsafe { &mut *self.lock.data.get() }
        }
    }

    impl<T: ?Sized> Drop for WrtParkingRwLockWriteGuard<'_, T> {
        #[inline]
        fn drop(&mut self) {
            self.lock.writer_waiting.store(false, Ordering::Release);
            self.lock.state.store(0, Ordering::Release);

            let _ = self.lock.waiters.wake_readers();
            let _ = self.lock.waiters.wake_writer();
        }
    }

    impl<T: ?Sized + CoreFmt::Debug> CoreFmt::Debug for WrtParkingRwLock<T> {
        fn fmt(&self, f: &mut CoreFmt::Formatter<'_>) -> CoreFmt::Result {
            let state_val = self.state.load(Ordering::Relaxed);
            let is_writer_waiting = self.writer_waiting.load(Ordering::Relaxed);

            let mut d = f.debug_struct("WrtParkingRwLock");

            if state_val == WRITE_LOCK_STATE {
                d.field("status", &"WriteLocked");
            } else if state_val == 0 {
                d.field("status", &"Unlocked");
            } else {
                d.field("status", &format_args!("ReadLocked({state_val})"));
            }

            d.field("writer_waiting", &is_writer_waiting);
            d.field("waiters", &"<WaitQueue>");

            if (state_val != WRITE_LOCK_STATE && state_val != 0)
                || (state_val == 0 && !is_writer_waiting)
            {
                unsafe {
                    d.field("data", &&*self.data.get());
                }
            } else {
                d.field("data", &"<locked_or_writer_pending>");
            }

            d.finish()
        }
    }

    #[cfg(test)]
    mod internal_parking_tests {
        use super::WrtParkingRwLock;
        use crate::prelude::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;

        #[test]
        fn test_parking_rwlock_basic() {
            let lock = WrtParkingRwLock::new(42);

            {
                let r = lock.read().unwrap();
                assert_eq!(*r, 42);
            }
            {
                let mut w = lock.write().unwrap();
                *w = 100;
            }
            let r = lock.read().unwrap();
            assert_eq!(*r, 100);
        }

        #[test]
        fn test_parking_rwlock_concurrent_reads() {
            let lock = Arc::new(WrtParkingRwLock::new(123));
            let mut handles = vec![];

            for _ in 0..5 {
                let lock_clone = Arc::clone(&lock);
                handles.push(thread::spawn(move || {
                    let reader = lock_clone.read().unwrap();
                    assert_eq!(*reader, 123);
                    thread::sleep(std::time::Duration::from_millis(10));
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }
        }

        #[test]
        fn test_parking_rwlock_writer_blocks_readers() {
            let lock = Arc::new(WrtParkingRwLock::new(0));
            let writer_ready = Arc::new(AtomicBool::new(false));
            let reader_finished_first_attempt = Arc::new(AtomicBool::new(false));

            let lock_clone_writer = Arc::clone(&lock);
            let writer_ready_clone = Arc::clone(&writer_ready);

            let writer_handle = thread::spawn(move || {
                let mut w_guard = lock_clone_writer.write().unwrap();
                *w_guard = 10;
                writer_ready_clone.store(true, Ordering::SeqCst);
                thread::sleep(std::time::Duration::from_millis(50));
            });

            while !writer_ready.load(Ordering::SeqCst) {
                thread::yield_now();
            }

            let lock_clone_reader = Arc::clone(&lock);
            let reader_finished_first_attempt_clone = Arc::clone(&reader_finished_first_attempt);
            let reader_handle = thread::spawn(move || {
                let r_guard = lock_clone_reader.read().unwrap();
                assert_eq!(*r_guard, 10);
                reader_finished_first_attempt_clone.store(true, Ordering::SeqCst);
            });

            thread::sleep(std::time::Duration::from_millis(10));
            assert!(
                !reader_finished_first_attempt.load(Ordering::SeqCst),
                "Reader should be blocked by writer"
            );

            writer_handle.join().unwrap();
            reader_handle.join().unwrap();
            assert!(
                reader_finished_first_attempt.load(Ordering::SeqCst),
                "Reader should have finished after writer released"
            );
        }

        #[test]
        fn test_parking_rwlock_try_operations() {
            let lock = WrtParkingRwLock::new(0);
            let w_opt = lock.try_write();
            assert!(w_opt.is_some());
            let mut w_guard = w_opt.unwrap();
            *w_guard = 20;
            drop(w_guard);
            let r_opt = lock.try_read();
            assert!(r_opt.is_some());
            assert_eq!(*r_opt.unwrap(), 20);
            let r1 = lock.read().unwrap();
            assert!(lock.try_write().is_none());
            let r_opt_2 = lock.try_read();
            assert!(r_opt_2.is_some());
            assert_eq!(*r_opt_2.unwrap(), 20);
            drop(r1);
            let mut w = lock.write().unwrap();
            *w = 2;
            assert!(lock.try_read().is_none());
            assert!(lock.try_write().is_none());
            drop(w);
            assert_eq!(*lock.read().unwrap(), 2);
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_rwlock_new_read() {
        let lock = WrtRwLock::new(10);
        let val = lock.read();
        assert_eq!(*val, 10);
    }

    #[test]
    fn test_rwlock_new_write_read() {
        let lock = WrtRwLock::new(0);
        {
            let mut w = lock.write();
            *w = 20;
        }
        let val = lock.read();
        assert_eq!(*val, 20);
    }

    #[test]
    fn test_rwlock_multiple_readers() {
        let lock = WrtRwLock::new(30);
        let r1 = lock.read();
        let r2 = lock.read();
        assert_eq!(*r1, 30);
        assert_eq!(*r2, 30);
        drop(r1);
        assert_eq!(*r2, 30);
    }

    #[test]
    fn test_rwlock_try_read_write() {
        let lock = WrtRwLock::new(5);

        if let Some(r_guard) = lock.try_read() {
            assert_eq!(*r_guard, 5);
            assert!(lock.try_write().is_none());
        } else {
            panic!("try_read failed when it should succeed");
        }

        if let Some(mut w_guard) = lock.try_write() {
            *w_guard = 10;
            assert!(lock.try_read().is_none());
            assert!(lock.try_write().is_none());
        } else {
            panic!("try_write failed when it should succeed");
        }
        assert_eq!(*lock.read(), 10);

        let r_guard = lock.read();
        assert!(lock.try_write().is_none(), "try_write should fail when read lock is held");
        drop(r_guard);

        let w_guard = lock.write();
        assert!(lock.try_read().is_none(), "try_read should fail when write lock is held");
        drop(w_guard);
    }
}
