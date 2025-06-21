//! Safe WebAssembly Atomic Operation Execution Engine
//!
//! This module implements the runtime execution of WebAssembly 3.0 atomic operations
//! using capability-based memory access to ensure safety without unsafe code.
//!
//! # Safety
//!
//! This module uses the capability system and platform abstractions to provide
//! atomic operations without any unsafe code, suitable for ASIL-D compliance.

extern crate alloc;

use crate::prelude::Debug;
use crate::thread_manager::{ThreadManager, ThreadId, ThreadExecutionStats};
use crate::bounded_runtime_infra::new_atomic_op_map;
use wrt_error::{Error, ErrorCategory, Result, codes};
use wrt_instructions::atomic_ops::{
    AtomicOp, AtomicLoadOp, AtomicStoreOp, AtomicRMWInstr, AtomicCmpxchgInstr,
    AtomicWaitNotifyOp, AtomicFence, AtomicRMWOp, MemoryOrdering,
};
use wrt_foundation::{
    MemArg, 
    traits::BoundedCapacity,
    capabilities::{MemoryCapability, MemoryCapabilityContext, MemoryOperation},
    platform_atomic::{SafeAtomicOps, get_platform_atomic_provider},
};

// Import platform abstractions from wrt-foundation PAI layer
use core::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering, fence};
use core::time::Duration;

#[cfg(feature = "std")]
use std::{vec::Vec, sync::Arc, collections::BTreeMap};
#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, sync::Arc, collections::BTreeMap};

// Type alias for thread ID vectors - use bounded collections consistently
type ThreadIdVec = wrt_foundation::bounded::BoundedVec<ThreadId, 64, wrt_foundation::safe_memory::NoStdProvider<1024>>;

/// Conversion from WebAssembly memory ordering to platform ordering
fn convert_memory_ordering(ordering: MemoryOrdering) -> AtomicOrdering {
    match ordering {
        MemoryOrdering::Unordered => AtomicOrdering::Relaxed,
        MemoryOrdering::SeqCst => AtomicOrdering::SeqCst,
        MemoryOrdering::Release => AtomicOrdering::Release,
        MemoryOrdering::Acquire => AtomicOrdering::Acquire,
        MemoryOrdering::AcqRel => AtomicOrdering::AcqRel,
        MemoryOrdering::Relaxed => AtomicOrdering::Relaxed,
    }
}

/// Safe atomic memory access context using capabilities
#[derive(Debug)]
pub struct SafeAtomicMemoryContext {
    /// Capability context for memory access verification
    capability_context: MemoryCapabilityContext,
    /// Safe atomic operations wrapper
    atomic_ops: SafeAtomicOps<'static>,
    /// Memory size in bytes
    memory_size: AtomicUsize,
    /// Thread manager for coordination
    pub thread_manager: ThreadManager,
    /// Wait/notify coordination data structures
    #[cfg(feature = "std")]
    wait_queues: crate::bounded_runtime_infra::BoundedAtomicOpMap<ThreadIdVec>,
    #[cfg(not(feature = "std"))]
    wait_queues: [(u32, [Option<ThreadId>; 8]); 16],
    /// Atomic operation statistics
    pub stats: AtomicExecutionStats,
}

impl SafeAtomicMemoryContext {
    /// Create new safe atomic memory context
    pub fn new(
        memory_base: *mut u8, 
        memory_size: usize, 
        thread_manager: ThreadManager,
        capability_context: MemoryCapabilityContext,
    ) -> Result<Self> {
        // Create safe atomic operations wrapper
        let atomic_ops = SafeAtomicOps::new(memory_base, memory_size)?;
        
        Ok(Self {
            capability_context,
            atomic_ops,
            memory_size: AtomicUsize::new(memory_size),
            thread_manager,
            wait_queues: new_atomic_op_map()?,
            stats: AtomicExecutionStats::new(),
        })
    }
    
    /// Execute atomic operation
    pub fn execute_atomic(&mut self, thread_id: ThreadId, op: AtomicOp) -> Result<Vec<u32>> {
        self.stats.total_operations += 1;
        
        // Update thread statistics
        if let Ok(context) = self.thread_manager.get_thread_context_mut(thread_id) {
            context.stats.record_atomic_operation();
        }
        
        match op {
            AtomicOp::Load(load_op) => self.execute_atomic_load(thread_id, load_op),
            AtomicOp::Store(store_op) => {
                // Pop value from stack for store operation
                let value = 0u64; // TODO: Should be popped from execution stack
                self.execute_atomic_store(thread_id, store_op, value)?;
                Ok(vec![])
            },
            AtomicOp::RMW(rmw_op) => {
                // Pop value from stack for RMW operation
                let value = 0u64; // TODO: Should be popped from execution stack
                self.execute_atomic_rmw(thread_id, rmw_op, value)
            },
            AtomicOp::Cmpxchg(cmpxchg_op) => {
                // Pop expected and replacement values from stack
                let expected = 0u64; // TODO: Should be popped from execution stack
                let replacement = 0u64; // TODO: Should be popped from execution stack
                self.execute_atomic_cmpxchg(thread_id, cmpxchg_op, expected, replacement)
            },
            AtomicOp::WaitNotify(wait_notify_op) => self.execute_wait_notify(thread_id, wait_notify_op),
            AtomicOp::Fence(fence) => {
                self.execute_atomic_fence(fence)?;
                Ok(vec![])
            },
        }
    }
    
    /// Execute atomic load operation
    fn execute_atomic_load(&mut self, thread_id: ThreadId, load_op: AtomicLoadOp) -> Result<Vec<u32>> {
        self.stats.load_operations += 1;
        
        match load_op {
            AtomicLoadOp::I32AtomicLoad { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 4)?;
                let value = self.atomic_ops.load_u32(addr, convert_memory_ordering(MemoryOrdering::SeqCst))?;
                Ok(vec![value])
            },
            AtomicLoadOp::I64AtomicLoad { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 8)?;
                let value = self.atomic_ops.load_u64(addr, convert_memory_ordering(MemoryOrdering::SeqCst))?;
                Ok(vec![value as u32, (value >> 32) as u32])
            },
            AtomicLoadOp::I32AtomicLoad8U { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 1)?;
                let value = self.atomic_ops.load_u8(addr, convert_memory_ordering(MemoryOrdering::SeqCst))?;
                Ok(vec![value as u32])
            },
            AtomicLoadOp::I32AtomicLoad16U { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 2)?;
                let value = self.atomic_ops.load_u16(addr, convert_memory_ordering(MemoryOrdering::SeqCst))?;
                Ok(vec![value as u32])
            },
            AtomicLoadOp::I64AtomicLoad8U { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 1)?;
                let value = self.atomic_ops.load_u8(addr, convert_memory_ordering(MemoryOrdering::SeqCst))?;
                Ok(vec![value as u32, 0])
            },
            AtomicLoadOp::I64AtomicLoad16U { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 2)?;
                let value = self.atomic_ops.load_u16(addr, convert_memory_ordering(MemoryOrdering::SeqCst))?;
                Ok(vec![value as u32, 0])
            },
            AtomicLoadOp::I64AtomicLoad32U { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 4)?;
                let value = self.atomic_ops.load_u32(addr, convert_memory_ordering(MemoryOrdering::SeqCst))?;
                Ok(vec![value, 0])
            },
        }
    }
    
    /// Execute atomic store operation
    fn execute_atomic_store(&mut self, thread_id: ThreadId, store_op: AtomicStoreOp, value: u64) -> Result<()> {
        self.stats.store_operations += 1;
        
        match store_op {
            AtomicStoreOp::I32AtomicStore { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_write(thread_id, addr, 4)?;
                self.atomic_ops.store_u32(addr, value as u32, convert_memory_ordering(MemoryOrdering::SeqCst))
            },
            AtomicStoreOp::I64AtomicStore { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_write(thread_id, addr, 8)?;
                self.atomic_ops.store_u64(addr, value, convert_memory_ordering(MemoryOrdering::SeqCst))
            },
            AtomicStoreOp::I32AtomicStore8 { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_write(thread_id, addr, 1)?;
                self.atomic_ops.store_u8(addr, value as u8, convert_memory_ordering(MemoryOrdering::SeqCst))
            },
            AtomicStoreOp::I32AtomicStore16 { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_write(thread_id, addr, 2)?;
                self.atomic_ops.store_u16(addr, value as u16, convert_memory_ordering(MemoryOrdering::SeqCst))
            },
            AtomicStoreOp::I64AtomicStore8 { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_write(thread_id, addr, 1)?;
                self.atomic_ops.store_u8(addr, value as u8, convert_memory_ordering(MemoryOrdering::SeqCst))
            },
            AtomicStoreOp::I64AtomicStore16 { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_write(thread_id, addr, 2)?;
                self.atomic_ops.store_u16(addr, value as u16, convert_memory_ordering(MemoryOrdering::SeqCst))
            },
            AtomicStoreOp::I64AtomicStore32 { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_write(thread_id, addr, 4)?;
                self.atomic_ops.store_u32(addr, value as u32, convert_memory_ordering(MemoryOrdering::SeqCst))
            },
        }
    }
    
    /// Execute atomic read-modify-write operation
    fn execute_atomic_rmw(&mut self, thread_id: ThreadId, rmw_op: AtomicRMWInstr, value: u64) -> Result<Vec<u32>> {
        self.stats.rmw_operations += 1;
        let ordering = convert_memory_ordering(MemoryOrdering::SeqCst);
        
        match rmw_op {
            // 32-bit and 64-bit ADD operations
            AtomicRMWInstr::I32AtomicRmwAdd { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_add_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value])
            },
            AtomicRMWInstr::I64AtomicRmwAdd { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 8)?;
                let old_value = self.atomic_ops.fetch_add_u64(addr, value, ordering)?;
                Ok(vec![old_value as u32, (old_value >> 32) as u32])
            },
            // Narrow ADD operations
            AtomicRMWInstr::I32AtomicRmw8AddU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_add_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I32AtomicRmw16AddU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_add_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I64AtomicRmw8AddU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_add_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw16AddU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_add_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw32AddU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_add_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value, 0])
            },
            
            // SUB operations
            AtomicRMWInstr::I32AtomicRmwSub { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_sub_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value])
            },
            AtomicRMWInstr::I64AtomicRmwSub { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 8)?;
                let old_value = self.atomic_ops.fetch_sub_u64(addr, value, ordering)?;
                Ok(vec![old_value as u32, (old_value >> 32) as u32])
            },
            AtomicRMWInstr::I32AtomicRmw8SubU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_sub_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I32AtomicRmw16SubU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_sub_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I64AtomicRmw8SubU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_sub_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw16SubU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_sub_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw32SubU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_sub_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value, 0])
            },
            
            // AND operations
            AtomicRMWInstr::I32AtomicRmwAnd { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_and_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value])
            },
            AtomicRMWInstr::I64AtomicRmwAnd { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 8)?;
                let old_value = self.atomic_ops.fetch_and_u64(addr, value, ordering)?;
                Ok(vec![old_value as u32, (old_value >> 32) as u32])
            },
            AtomicRMWInstr::I32AtomicRmw8AndU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_and_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I32AtomicRmw16AndU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_and_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I64AtomicRmw8AndU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_and_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw16AndU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_and_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw32AndU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_and_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value, 0])
            },
            
            // OR operations
            AtomicRMWInstr::I32AtomicRmwOr { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_or_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value])
            },
            AtomicRMWInstr::I64AtomicRmwOr { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 8)?;
                let old_value = self.atomic_ops.fetch_or_u64(addr, value, ordering)?;
                Ok(vec![old_value as u32, (old_value >> 32) as u32])
            },
            AtomicRMWInstr::I32AtomicRmw8OrU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_or_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I32AtomicRmw16OrU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_or_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I64AtomicRmw8OrU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_or_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw16OrU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_or_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw32OrU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_or_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value, 0])
            },
            
            // XOR operations
            AtomicRMWInstr::I32AtomicRmwXor { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_xor_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value])
            },
            AtomicRMWInstr::I64AtomicRmwXor { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 8)?;
                let old_value = self.atomic_ops.fetch_xor_u64(addr, value, ordering)?;
                Ok(vec![old_value as u32, (old_value >> 32) as u32])
            },
            AtomicRMWInstr::I32AtomicRmw8XorU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_xor_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I32AtomicRmw16XorU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_xor_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I64AtomicRmw8XorU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.fetch_xor_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw16XorU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.fetch_xor_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw32XorU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.fetch_xor_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value, 0])
            },
            
            // XCHG (exchange) operations
            AtomicRMWInstr::I32AtomicRmwXchg { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.swap_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value])
            },
            AtomicRMWInstr::I64AtomicRmwXchg { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 8)?;
                let old_value = self.atomic_ops.swap_u64(addr, value, ordering)?;
                Ok(vec![old_value as u32, (old_value >> 32) as u32])
            },
            AtomicRMWInstr::I32AtomicRmw8XchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.swap_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I32AtomicRmw16XchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.swap_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32])
            },
            AtomicRMWInstr::I64AtomicRmw8XchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.swap_u8(addr, value as u8, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw16XchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.swap_u16(addr, value as u16, ordering)?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicRMWInstr::I64AtomicRmw32XchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.swap_u32(addr, value as u32, ordering)?;
                Ok(vec![old_value, 0])
            },
        }
    }
    
    /// Execute atomic compare-and-exchange operation
    fn execute_atomic_cmpxchg(
        &mut self, 
        thread_id: ThreadId,
        cmpxchg_op: AtomicCmpxchgInstr, 
        expected: u64, 
        replacement: u64
    ) -> Result<Vec<u32>> {
        self.stats.cmpxchg_operations += 1;
        let ordering = convert_memory_ordering(MemoryOrdering::SeqCst);
        
        match cmpxchg_op {
            AtomicCmpxchgInstr::I32AtomicRmwCmpxchg { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.cmpxchg_u32(
                    addr,
                    expected as u32,
                    replacement as u32,
                    ordering,
                    ordering,
                )?;
                Ok(vec![old_value])
            },
            AtomicCmpxchgInstr::I64AtomicRmwCmpxchg { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 8)?;
                let old_value = self.atomic_ops.cmpxchg_u64(
                    addr,
                    expected,
                    replacement,
                    ordering,
                    ordering,
                )?;
                Ok(vec![old_value as u32, (old_value >> 32) as u32])
            },
            // Narrow compare-and-exchange operations
            AtomicCmpxchgInstr::I32AtomicRmw8CmpxchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.cmpxchg_u8(
                    addr,
                    expected as u8,
                    replacement as u8,
                    ordering,
                    ordering,
                )?;
                Ok(vec![old_value as u32])
            },
            AtomicCmpxchgInstr::I32AtomicRmw16CmpxchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.cmpxchg_u16(
                    addr,
                    expected as u16,
                    replacement as u16,
                    ordering,
                    ordering,
                )?;
                Ok(vec![old_value as u32])
            },
            AtomicCmpxchgInstr::I64AtomicRmw8CmpxchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 1)?;
                let old_value = self.atomic_ops.cmpxchg_u8(
                    addr,
                    expected as u8,
                    replacement as u8,
                    ordering,
                    ordering,
                )?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicCmpxchgInstr::I64AtomicRmw16CmpxchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 2)?;
                let old_value = self.atomic_ops.cmpxchg_u16(
                    addr,
                    expected as u16,
                    replacement as u16,
                    ordering,
                    ordering,
                )?;
                Ok(vec![old_value as u32, 0])
            },
            AtomicCmpxchgInstr::I64AtomicRmw32CmpxchgU { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read_write(thread_id, addr, 4)?;
                let old_value = self.atomic_ops.cmpxchg_u32(
                    addr,
                    expected as u32,
                    replacement as u32,
                    ordering,
                    ordering,
                )?;
                Ok(vec![old_value, 0])
            },
        }
    }
    
    /// Execute wait/notify operations
    fn execute_wait_notify(&mut self, thread_id: ThreadId, wait_notify_op: AtomicWaitNotifyOp) -> Result<Vec<u32>> {
        match wait_notify_op {
            AtomicWaitNotifyOp::MemoryAtomicWait32 { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 4)?;
                self.atomic_wait_u32(thread_id, addr, Duration::from_secs(u64::MAX))
            },
            AtomicWaitNotifyOp::MemoryAtomicWait64 { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_read(thread_id, addr, 8)?;
                self.atomic_wait_u64(thread_id, addr, Duration::from_secs(u64::MAX))
            },
            AtomicWaitNotifyOp::MemoryAtomicNotify { memarg } => {
                let addr = self.calculate_address(memarg)?;
                self.verify_capability_for_write(thread_id, addr, 4)?;
                let count = self.atomic_notify(addr, u32::MAX)?;
                Ok(vec![count])
            },
        }
    }
    
    /// Execute atomic fence operation
    fn execute_atomic_fence(&mut self, fence_op: AtomicFence) -> Result<()> {
        self.stats.fence_operations += 1;
        
        // Execute memory fence with specified ordering
        let ordering = convert_memory_ordering(fence_op.ordering);
        fence(ordering);
        
        Ok(())
    }
    
    /// Calculate address from memory argument
    fn calculate_address(&self, memarg: MemArg) -> Result<usize> {
        let addr = memarg.offset as usize;
        if addr >= self.memory_size.load(AtomicOrdering::Relaxed) {
            return Err(Error::new(
                ErrorCategory::Runtime,
                codes::EXECUTION_ERROR,
                "Atomic operation address out of bounds"
            ));
        }
        Ok(addr)
    }
    
    /// Verify capability for read operation
    fn verify_capability_for_read(&self, thread_id: ThreadId, offset: usize, len: usize) -> Result<()> {
        let operation = MemoryOperation::Read { offset, len };
        let crate_id = wrt_foundation::budget_aware_provider::CrateId::from_thread_id(thread_id);
        self.capability_context.verify_operation(crate_id, &operation)
    }
    
    /// Verify capability for write operation
    fn verify_capability_for_write(&self, thread_id: ThreadId, offset: usize, len: usize) -> Result<()> {
        let operation = MemoryOperation::Write { offset, len };
        let crate_id = wrt_foundation::budget_aware_provider::CrateId::from_thread_id(thread_id);
        self.capability_context.verify_operation(crate_id, &operation)
    }
    
    /// Verify capability for read-write operation
    fn verify_capability_for_read_write(&self, thread_id: ThreadId, offset: usize, len: usize) -> Result<()> {
        self.verify_capability_for_read(thread_id, offset, len)?;
        self.verify_capability_for_write(thread_id, offset, len)
    }
    
    /// Atomic wait implementation
    fn atomic_wait_u32(&mut self, thread_id: ThreadId, addr: usize, _timeout: Duration) -> Result<Vec<u32>> {
        self.stats.wait_operations += 1;
        
        // Add thread to wait queue
        #[cfg(feature = "std")]
        {
            let provider = wrt_foundation::safe_memory::NoStdProvider::<1024>::default();
            let default_vec = wrt_foundation::bounded::BoundedVec::new(provider).map_err(|_| {
                Error::runtime_error("Failed to create thread wait queue")
            })?;
            
            match self.wait_queues.get(&(addr as u64))? {
                Some(mut existing_vec) => {
                    existing_vec.push(thread_id).map_err(|_| {
                        Error::runtime_error("Thread wait queue capacity exceeded")
                    })?;
                    self.wait_queues.insert(addr as u64, existing_vec)?;
                }
                None => {
                    let mut new_vec = default_vec;
                    new_vec.push(thread_id).map_err(|_| {
                        Error::runtime_error("Failed to add thread to wait queue")
                    })?;
                    self.wait_queues.insert(addr as u64, new_vec)?;
                }
            }
        }
        
        // Simplified - return 0 for successful wait
        Ok(vec![0])
    }
    
    /// Atomic wait u64 implementation
    fn atomic_wait_u64(&mut self, thread_id: ThreadId, addr: usize, timeout: Duration) -> Result<Vec<u32>> {
        // Same as u32 wait for now
        self.atomic_wait_u32(thread_id, addr, timeout)
    }
    
    /// Atomic notify implementation
    fn atomic_notify(&mut self, addr: usize, count: u32) -> Result<u32> {
        self.stats.notify_operations += 1;
        
        let mut notified = 0u32;
        
        #[cfg(feature = "std")]
        {
            if let Ok(Some(queue)) = self.wait_queues.get_mut(&(addr as u64)) {
                let to_notify = core::cmp::min(count as usize, queue.len());
                for _ in 0..to_notify {
                    if let Ok(Some(_thread_id)) = queue.pop() {
                        // In real implementation, would wake up the thread
                        notified += 1;
                    }
                }
                if queue.is_empty() {
                    self.wait_queues.remove(&(addr as u64))?;
                }
            }
        }
        
        Ok(notified)
    }
}

/// Statistics for atomic operation execution
#[derive(Debug, Clone)]
pub struct AtomicExecutionStats {
    /// Total atomic operations executed
    pub total_operations: u64,
    /// Atomic load operations
    pub load_operations: u64,
    /// Atomic store operations
    pub store_operations: u64,
    /// Atomic read-modify-write operations
    pub rmw_operations: u64,
    /// Atomic compare-exchange operations
    pub cmpxchg_operations: u64,
    /// Atomic fence operations
    pub fence_operations: u64,
    /// Atomic wait operations
    pub wait_operations: u64,
    /// Atomic notify operations
    pub notify_operations: u64,
    /// Memory ordering conflicts detected
    pub ordering_conflicts: u64,
}

impl AtomicExecutionStats {
    fn new() -> Self {
        Self {
            total_operations: 0,
            load_operations: 0,
            store_operations: 0,
            rmw_operations: 0,
            cmpxchg_operations: 0,
            fence_operations: 0,
            wait_operations: 0,
            notify_operations: 0,
            ordering_conflicts: 0,
        }
    }
    
    /// Get atomic operation throughput (operations per call)
    #[must_use] pub fn throughput(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            (self.load_operations + self.store_operations + self.rmw_operations) as f64 / self.total_operations as f64
        }
    }
    
    /// Check if atomic execution is performing well
    #[must_use] pub fn is_healthy(&self) -> bool {
        self.total_operations > 0 && self.ordering_conflicts < self.total_operations / 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thread_manager::ThreadConfig;
    use wrt_foundation::capabilities::{DynamicMemoryCapability, CapabilityMask};
    
    #[test]
    fn test_atomic_execution_stats() {
        let stats = AtomicExecutionStats::new();
        assert_eq!(stats.total_operations, 0);
        assert_eq!(stats.throughput(), 0.0);
        assert!(!stats.is_healthy());
    }
    
    #[test]
    fn test_memory_ordering_conversion() {
        assert_eq!(convert_memory_ordering(MemoryOrdering::Unordered), AtomicOrdering::Relaxed);
        assert_eq!(convert_memory_ordering(MemoryOrdering::SeqCst), AtomicOrdering::SeqCst);
    }
}