// WRT - wrt-component
// Module: Waitable Set Canonical Operations
// SW-REQ-ID: REQ_WAITABLE_SET_001
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)]

//! Waitable Set Canonical Operations
//!
//! This module provides implementation of the `waitable-set.*` built-in functions
//! required by the WebAssembly Component Model for managing sets of waitable objects.


extern crate alloc;

use std::{boxed::Box, collections::BTreeMap, collections::BTreeSet, vec::Vec};
#[cfg(feature = "std")]
use std::{boxed::Box, collections::HashMap, collections::HashSet, vec::Vec};

use wrt_error::{Error, ErrorCategory, Result};
use wrt_foundation::{
    atomic_memory::AtomicRefCell,
    bounded::{BoundedMap, BoundedSet, BoundedVec},
    component_value::ComponentValue,
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

use crate::async_::async_types::{Future, FutureHandle, Stream, StreamHandle, Waitable, WaitableSet};
use crate::task_builtins::{TaskId as TaskBuiltinId, TaskStatus};

// Constants for no_std environments
#[cfg(not(any(feature = "std", )))]
const MAX_WAITABLE_SETS: usize = 32;
#[cfg(not(any(feature = "std", )))]
const MAX_WAITABLES_PER_SET: usize = 64;
#[cfg(not(any(feature = "std", )))]
const MAX_WAIT_RESULTS: usize = 64;

/// Waitable set identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WaitableSetId(pub u64);

impl WaitableSetId {
    pub fn new() -> Self {
        static COUNTER: core::sync::atomic::AtomicU64 = 
            core::sync::atomic::AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for WaitableSetId {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a wait operation
#[derive(Debug, Clone, PartialEq)]
pub enum WaitResult {
    /// A waitable became ready
    Ready(WaitableEntry),
    /// Wait operation timed out
    Timeout,
    /// Wait operation was cancelled
    Cancelled,
    /// An error occurred during waiting
    Error(Error),
}

impl WaitResult {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    pub fn as_ready(&self) -> Option<&WaitableEntry> {
        match self {
            Self::Ready(entry) => Some(entry),
            _ => None,
        }
    }

    pub fn into_ready(self) -> Option<WaitableEntry> {
        match self {
            Self::Ready(entry) => Some(entry),
            _ => None,
        }
    }
}

/// Entry in a waitable set
#[derive(Debug, Clone, PartialEq)]
pub struct WaitableEntry {
    pub id: WaitableId,
    pub waitable: Waitable,
    pub ready: bool,
}

impl WaitableEntry {
    pub fn new(id: WaitableId, waitable: Waitable) -> Self {
        Self {
            id,
            waitable,
            ready: false,
        }
    }

    pub fn mark_ready(&mut self) {
        self.ready = true;
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn check_ready(&mut self) -> bool {
        self.ready = match &self.waitable {
            Waitable::Future(future) => {
                matches!(future.state, crate::async_::async_types::FutureState::Ready | 
                                     crate::async_::async_types::FutureState::Error)
            }
            Waitable::Stream(stream) => {
                match stream.state {
                    crate::async_::async_types::StreamState::Open => true, // Data available to read
                    crate::async_::async_types::StreamState::Closed => true, // EOF condition
                    _ => false,
                }
            }
            Waitable::WaitableSet(_) => {
                // Nested waitable sets are ready if any of their contents are ready
                // This would require recursive checking in a full implementation
                false
            }
        };
        self.ready
    }
}

/// Waitable identifier within a set
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WaitableId(pub u64);

impl WaitableId {
    pub fn new() -> Self {
        static COUNTER: core::sync::atomic::AtomicU64 = 
            core::sync::atomic::AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for WaitableId {
    fn default() -> Self {
        Self::new()
    }
}

/// A set of waitable objects that can be waited on collectively
#[derive(Debug, Clone)]
pub struct WaitableSetImpl {
    pub id: WaitableSetId,
    #[cfg(feature = "std")]
    pub waitables: BTreeMap<WaitableId, WaitableEntry>,
    #[cfg(not(any(feature = "std", )))]
    pub waitables: BoundedMap<WaitableId, WaitableEntry, MAX_WAITABLES_PER_SET>,
    pub closed: bool,
}

impl WaitableSetImpl {
    pub fn new() -> Self {
        Self {
            id: WaitableSetId::new(),
            #[cfg(feature = "std")]
            waitables: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            waitables: BoundedMap::new(),
            closed: false,
        }
    }

    pub fn add_waitable(&mut self, waitable: Waitable) -> Result<WaitableId> {
        if self.closed {
            return Err(Error::runtime_execution_error("
            ));
        }

        let id = WaitableId::new();
        let entry = WaitableEntry::new(id, waitable);

        #[cfg(feature = ")]
        {
            self.waitables.insert(id, entry);
            Ok(id)
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.waitables.insert(id, entry)
                .map_err(|_| Error::runtime_execution_error("
                ))?;
            Ok(id)
        }
    }

    pub fn remove_waitable(&mut self, id: WaitableId) -> Option<WaitableEntry> {
        self.waitables.remove(&id)
    }

    pub fn contains_waitable(&self, id: WaitableId) -> bool {
        self.waitables.contains_key(&id)
    }

    pub fn waitable_count(&self) -> usize {
        self.waitables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.waitables.is_empty()
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Check all waitables and return those that are ready
    #[cfg(feature = ")]
    pub fn check_ready(&mut self) -> Vec<WaitableEntry> {
        let mut ready = Vec::new();
        for (_, entry) in self.waitables.iter_mut() {
            if entry.check_ready() {
                ready.push(entry.clone());
            }
        }
        ready
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn check_ready(&mut self) -> Result<BoundedVec<WaitableEntry, MAX_WAIT_RESULTS, NoStdProvider<65536>>> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
        let mut ready = BoundedVec::new(provider).map_err(|_| {
            Error::runtime_execution_error("Failed to create ready waitables vector")
        })?;
        for (_, entry) in self.waitables.iter_mut() {
            if entry.check_ready() {
                ready.push(entry.clone())
                    .map_err(|_| Error::runtime_execution_error("Failed to add ready waitable"))?;
            }
        }
        Ok(ready)
    }

    /// Get the first ready waitable if any
    pub fn get_first_ready(&mut self) -> Option<WaitableEntry> {
        for (_, entry) in self.waitables.iter_mut() {
            if entry.check_ready() {
                return Some(entry.clone());
            }
        }
        None
    }

    /// Wait for any waitable to become ready (non-blocking check)
    pub fn poll(&mut self) -> Option<WaitResult> {
        if let Some(ready_entry) = self.get_first_ready() {
            Some(WaitResult::Ready(ready_entry))
        } else {
            None
        }
    }
}

impl Default for WaitableSetImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry for waitable sets
static WAITABLE_SET_REGISTRY: AtomicRefCell<Option<WaitableSetRegistry>> = 
    AtomicRefCell::new(None);

/// Registry that manages all waitable sets
#[derive(Debug)]
pub struct WaitableSetRegistry {
    #[cfg(feature = ")]
    sets: HashMap<WaitableSetId, WaitableSetImpl>,
    #[cfg(not(any(feature = "std", )))]
    sets: BoundedMap<WaitableSetId, WaitableSetImpl, MAX_WAITABLE_SETS>,
}

impl WaitableSetRegistry {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            sets: HashMap::new(),
            #[cfg(not(any(feature = "std", )))]
            sets: BoundedMap::new(),
        }
    }

    pub fn register_set(&mut self, set: WaitableSetImpl) -> Result<WaitableSetId> {
        let id = set.id;
        #[cfg(feature = "std")]
        {
            self.sets.insert(id, set);
            Ok(id)
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.sets.insert(id, set)
                .map_err(|_| Error::runtime_execution_error("
                ))?;
            Ok(id)
        }
    }

    pub fn get_set(&self, id: WaitableSetId) -> Option<&WaitableSetImpl> {
        self.sets.get(&id)
    }

    pub fn get_set_mut(&mut self, id: WaitableSetId) -> Option<&mut WaitableSetImpl> {
        self.sets.get_mut(&id)
    }

    pub fn remove_set(&mut self, id: WaitableSetId) -> Option<WaitableSetImpl> {
        self.sets.remove(&id)
    }

    pub fn set_count(&self) -> usize {
        self.sets.len()
    }
}

impl Default for WaitableSetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Waitable set built-ins providing canonical functions
pub struct WaitableSetBuiltins;

impl WaitableSetBuiltins {
    /// Initialize the global waitable set registry
    pub fn initialize() -> Result<()> {
        let mut registry_ref = WAITABLE_SET_REGISTRY.try_borrow_mut()
            .map_err(|_| Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::INVALID_STATE,
                "))?;
        *registry_ref = Some(WaitableSetRegistry::new());
        Ok(())
    }

    /// Get the global registry
    fn with_registry<F, R>(f: F) -> Result<R>
    where
        F: FnOnce(&WaitableSetRegistry) -> R,
    {
        let registry_ref = WAITABLE_SET_REGISTRY.try_borrow()
            .map_err(|_| Error::runtime_execution_error("
            ))?;
        let registry = registry_ref.as_ref()
            .ok_or_else(|| Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::INVALID_STATE,
                "))?;
        Ok(f(registry))
    }

    /// Get the global registry mutably
    fn with_registry_mut<F, R>(f: F) -> Result<R>
    where
        F: FnOnce(&mut WaitableSetRegistry) -> Result<R>,
    {
        let mut registry_ref = WAITABLE_SET_REGISTRY.try_borrow_mut()
            .map_err(|_| Error::runtime_execution_error("
            ))?;
        let registry = registry_ref.as_mut()
            .ok_or_else(|| Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::INVALID_STATE,
                "))?;
        f(registry)
    }

    /// `waitable-set.new` canonical built-in
    /// Creates a new waitable set
    pub fn waitable_set_new() -> Result<WaitableSetId> {
        let set = WaitableSetImpl::new();
        Self::with_registry_mut(|registry| {
            registry.register_set(set)
        })?
    }

    /// `waitable-set.add` canonical built-in
    /// Adds a waitable to a set
    pub fn waitable_set_add(set_id: WaitableSetId, waitable: Waitable) -> Result<WaitableId> {
        Self::with_registry_mut(|registry| {
            if let Some(set) = registry.get_set_mut(set_id) {
                set.add_waitable(waitable)
            } else {
                Err(Error::runtime_execution_error("
                ))
            }
        })?
    }

    /// `waitable-set.remove` canonical built-in
    /// Removes a waitable from a set
    pub fn waitable_set_remove(set_id: WaitableSetId, waitable_id: WaitableId) -> Result<bool> {
        Self::with_registry_mut(|registry| {
            if let Some(set) = registry.get_set_mut(set_id) {
                Ok(set.remove_waitable(waitable_id).is_some())
            } else {
                Err(Error::new(
                    ErrorCategory::Runtime,
                    wrt_error::codes::RESOURCE_INVALID_HANDLE,
                    "))
            }
        })?
    }

    /// `waitable-set.wait` canonical built-in
    /// Waits for any waitable in the set to become ready
    pub fn waitable_set_wait(set_id: WaitableSetId) -> Result<WaitResult> {
        Self::with_registry_mut(|registry| {
            if let Some(set) = registry.get_set_mut(set_id) {
                Ok(set.poll().unwrap_or(WaitResult::Timeout))
            } else {
                Err(Error::runtime_execution_error("
                ))
            }
        })?
    }

    /// Check if a waitable set contains a specific waitable
    pub fn waitable_set_contains(set_id: WaitableSetId, waitable_id: WaitableId) -> Result<bool> {
        Self::with_registry(|registry| {
            if let Some(set) = registry.get_set(set_id) {
                set.contains_waitable(waitable_id)
            } else {
                false
            }
        })
    }

    /// Get the number of waitables in a set
    pub fn waitable_set_count(set_id: WaitableSetId) -> Result<usize> {
        Self::with_registry(|registry| {
            if let Some(set) = registry.get_set(set_id) {
                set.waitable_count()
            } else {
                0
            }
        })
    }

    /// Close a waitable set (no more waitables can be added)
    pub fn waitable_set_close(set_id: WaitableSetId) -> Result<()> {
        Self::with_registry_mut(|registry| {
            if let Some(set) = registry.get_set_mut(set_id) {
                set.close();
                Ok(())
            } else {
                Err(Error::new(
                    ErrorCategory::Runtime,
                    wrt_error::codes::RESOURCE_INVALID_HANDLE,
                    "))
            }
        })?
    }

    /// Remove a waitable set from the registry
    pub fn waitable_set_drop(set_id: WaitableSetId) -> Result<()> {
        Self::with_registry_mut(|registry| {
            registry.remove_set(set_id);
            Ok(())
        })?
    }

    /// Get all ready waitables from a set
    #[cfg(feature = "std")]
    pub fn waitable_set_poll_all(set_id: WaitableSetId) -> Result<Vec<WaitableEntry>> {
        Self::with_registry_mut(|registry| {
            if let Some(set) = registry.get_set_mut(set_id) {
                Ok(set.check_ready())
            } else {
                Err(Error::runtime_execution_error("
                ))
            }
        })?
    }

    #[cfg(not(any(feature = ")))]
    pub fn waitable_set_poll_all(set_id: WaitableSetId) -> core::result::Result<BoundedVec<WaitableEntry, MAX_WAIT_RESULTS, NoStdProvider<65536>>, NoStdProvider<65536>> {
        Self::with_registry_mut(|registry| {
            if let Some(set) = registry.get_set_mut(set_id) {
                set.check_ready()
            } else {
                Err(Error::runtime_execution_error("
                ))
            }
        })?
    }
}

/// Convenience functions for working with waitable sets
pub mod waitable_set_helpers {
    use super::*;

    /// Create a waitable set with initial waitables
    #[cfg(feature = ")]
    pub fn create_waitable_set_with(waitables: Vec<Waitable>) -> Result<WaitableSetId> {
        let set_id = WaitableSetBuiltins::waitable_set_new()?;
        for waitable in waitables {
            WaitableSetBuiltins::waitable_set_add(set_id, waitable)?;
        }
        Ok(set_id)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn create_waitable_set_with(waitables: &[Waitable]) -> Result<WaitableSetId> {
        let set_id = WaitableSetBuiltins::waitable_set_new()?;
        for waitable in waitables {
            WaitableSetBuiltins::waitable_set_add(set_id, waitable.clone())?;
        }
        Ok(set_id)
    }

    /// Wait for any of multiple futures to complete
    #[cfg(feature = "std")]
    pub fn wait_for_any_future(futures: Vec<Future>) -> Result<WaitResult> {
        let waitables: Vec<Waitable> = futures.into_iter()
            .map(Waitable::Future)
            .collect();
        let set_id = create_waitable_set_with(waitables)?;
        WaitableSetBuiltins::waitable_set_wait(set_id)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn wait_for_any_future(futures: &[Future]) -> Result<WaitResult> {
        let mut waitables = BoundedVec::<Waitable, MAX_WAITABLES_PER_SET>::new();
        for future in futures {
            waitables.push(Waitable::Future(future.clone()))
                .map_err(|_| Error::runtime_execution_error("
                ))?;
        }
        let set_id = create_waitable_set_with(waitables.as_slice())?;
        WaitableSetBuiltins::waitable_set_wait(set_id)
    }

    /// Wait for any of multiple streams to have data available
    #[cfg(feature = ")]
    pub fn wait_for_any_stream(streams: Vec<Stream>) -> Result<WaitResult> {
        let waitables: Vec<Waitable> = streams.into_iter()
            .map(Waitable::Stream)
            .collect();
        let set_id = create_waitable_set_with(waitables)?;
        WaitableSetBuiltins::waitable_set_wait(set_id)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn wait_for_any_stream(streams: &[Stream]) -> Result<WaitResult> {
        let mut waitables = BoundedVec::<Waitable, MAX_WAITABLES_PER_SET>::new();
        for stream in streams {
            waitables.push(Waitable::Stream(stream.clone()))
                .map_err(|_| Error::runtime_execution_error("
                ))?;
        }
        let set_id = create_waitable_set_with(waitables.as_slice())?;
        WaitableSetBuiltins::waitable_set_wait(set_id)
    }

    /// Create a waitable from a future handle
    pub fn waitable_from_future_handle(handle: FutureHandle) -> Waitable {
        Waitable::Future(Future {
            handle,
            state: crate::async_::async_types::FutureState::Pending,
        })
    }

    /// Create a waitable from a stream handle
    pub fn waitable_from_stream_handle(handle: StreamHandle) -> Waitable {
        Waitable::Stream(Stream {
            handle,
            state: crate::async_::async_types::StreamState::Open,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_::async_types::{FutureState, StreamState};

    #[test]
    fn test_waitable_set_id_generation() {
        let id1 = WaitableSetId::new();
        let id2 = WaitableSetId::new();
        assert_ne!(id1, id2);
        assert!(id1.as_u64() > 0);
        assert!(id2.as_u64() > 0);
    }

    #[test]
    fn test_waitable_id_generation() {
        let id1 = WaitableId::new();
        let id2 = WaitableId::new();
        assert_ne!(id1, id2);
        assert!(id1.as_u64() > 0);
        assert!(id2.as_u64() > 0);
    }

    #[test]
    fn test_wait_result_methods() {
        let entry = WaitableEntry::new(
            WaitableId::new(),
            Waitable::Future(Future {
                handle: FutureHandle::new(),
                state: FutureState::Pending,
            })
        );

        let ready_result = WaitResult::Ready(entry.clone());
        assert!(ready_result.is_ready());
        assert!(!ready_result.is_timeout());
        assert!(!ready_result.is_cancelled());
        assert!(!ready_result.is_error());
        assert!(ready_result.as_ready().is_some());

        let timeout_result = WaitResult::Timeout;
        assert!(!timeout_result.is_ready());
        assert!(timeout_result.is_timeout());
    }

    #[test]
    fn test_waitable_entry_ready_check() {
        // Test future waitable
        let mut future_entry = WaitableEntry::new(
            WaitableId::new(),
            Waitable::Future(Future {
                handle: FutureHandle::new(),
                state: FutureState::Pending,
            })
        );
        assert!(!future_entry.check_ready());

        future_entry.waitable = Waitable::Future(Future {
            handle: FutureHandle::new(),
            state: FutureState::Resolved(ComponentValue::Bool(true)),
        });
        assert!(future_entry.check_ready());

        // Test stream waitable
        let mut stream_entry = WaitableEntry::new(
            WaitableId::new(),
            Waitable::Stream(Stream {
                handle: StreamHandle::new(),
                state: StreamState::Pending,
            })
        );
        assert!(!stream_entry.check_ready());

        stream_entry.waitable = Waitable::Stream(Stream {
            handle: StreamHandle::new(),
            state: StreamState::Open,
        });
        assert!(stream_entry.check_ready());
    }

    #[test]
    fn test_waitable_set_operations() {
        let mut set = WaitableSetImpl::new();
        assert!(set.is_empty());
        assert!(!set.is_closed());

        // Add a waitable
        let future = Future {
            handle: FutureHandle::new(),
            state: FutureState::Pending,
        };
        let waitable_id = set.add_waitable(Waitable::Future(future)).unwrap();

        assert!(!set.is_empty());
        assert_eq!(set.waitable_count(), 1);
        assert!(set.contains_waitable(waitable_id));

        // Remove the waitable
        let removed = set.remove_waitable(waitable_id);
        assert!(removed.is_some());
        assert!(set.is_empty());
        assert!(!set.contains_waitable(waitable_id));

        // Close the set
        set.close();
        assert!(set.is_closed());

        // Try to add to closed set
        let future2 = Future {
            handle: FutureHandle::new(),
            state: FutureState::Pending,
        };
        let result = set.add_waitable(Waitable::Future(future2));
        assert!(result.is_err());
    }

    #[test]
    fn test_waitable_set_ready_checking() {
        let mut set = WaitableSetImpl::new();

        // Add pending future
        let pending_future = Future {
            handle: FutureHandle::new(),
            state: FutureState::Pending,
        };
        set.add_waitable(Waitable::Future(pending_future)).unwrap();

        // Add resolved future
        let resolved_future = Future {
            handle: FutureHandle::new(),
            state: FutureState::Resolved(ComponentValue::I32(42)),
        };
        set.add_waitable(Waitable::Future(resolved_future)).unwrap();

        // Check for ready waitables
        #[cfg(feature = ")]
        {
            let ready = set.check_ready();
            assert_eq!(ready.len(), 1);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let ready = set.check_ready().unwrap();
            assert_eq!(ready.len(), 1);
        }

        // Poll for first ready
        let first_ready = set.get_first_ready();
        assert!(first_ready.is_some());
        assert!(first_ready.unwrap().is_ready());
    }

    #[test]
    fn test_waitable_set_registry() {
        let mut registry = WaitableSetRegistry::new();
        assert_eq!(registry.set_count(), 0);

        let set = WaitableSetImpl::new();
        let set_id = set.id;
        registry.register_set(set).unwrap();
        assert_eq!(registry.set_count(), 1);

        let retrieved_set = registry.get_set(set_id);
        assert!(retrieved_set.is_some());
        assert_eq!(retrieved_set.unwrap().id, set_id);

        let removed_set = registry.remove_set(set_id);
        assert!(removed_set.is_some());
        assert_eq!(registry.set_count(), 0);
    }

    #[test]
    fn test_waitable_set_builtins() {
        // Initialize the registry
        WaitableSetBuiltins::initialize().unwrap();

        // Create a new waitable set
        let set_id = WaitableSetBuiltins::waitable_set_new().unwrap();

        // Add a waitable
        let future = Future {
            handle: FutureHandle::new(),
            state: FutureState::Pending,
        };
        let waitable_id = WaitableSetBuiltins::waitable_set_add(set_id, Waitable::Future(future)).unwrap();

        // Check operations
        assert!(WaitableSetBuiltins::waitable_set_contains(set_id, waitable_id).unwrap());
        assert_eq!(WaitableSetBuiltins::waitable_set_count(set_id).unwrap(), 1);

        // Wait operation (should timeout since nothing is ready)
        let wait_result = WaitableSetBuiltins::waitable_set_wait(set_id).unwrap();
        assert!(wait_result.is_timeout());

        // Remove waitable
        assert!(WaitableSetBuiltins::waitable_set_remove(set_id, waitable_id).unwrap());
        assert_eq!(WaitableSetBuiltins::waitable_set_count(set_id).unwrap(), 0);

        // Close set
        WaitableSetBuiltins::waitable_set_close(set_id).unwrap();

        // Drop set
        WaitableSetBuiltins::waitable_set_drop(set_id).unwrap();
    }

    #[test]
    fn test_helper_functions() {
        WaitableSetBuiltins::initialize().unwrap();

        // Test waitable creation helpers
        let future_handle = FutureHandle::new();
        let waitable = waitable_set_helpers::waitable_from_future_handle(future_handle);
        assert!(matches!(waitable, Waitable::Future(_)));

        let stream_handle = StreamHandle::new();
        let waitable = waitable_set_helpers::waitable_from_stream_handle(stream_handle);
        assert!(matches!(waitable, Waitable::Stream(_)));
    }
}