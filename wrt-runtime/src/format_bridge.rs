//! Format bridge interface
//!
//! This module provides clean interfaces for receiving format data from
//! wrt-format and converting it to runtime representations. It establishes
//! the runtime side of the boundary between format and runtime layers.

// alloc is imported in lib.rs with proper feature gates

use crate::prelude::*;

#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
// For no_std without alloc, use type aliases with concrete providers
#[cfg(not(any(feature = "std", feature = "alloc")))]
type Vec<T> = wrt_foundation::BoundedVec<T, 16, wrt_foundation::NoStdProvider<1024>>;

/// Trait for types that can be initialized from format representations
pub trait FromFormatBridge<FormatData> {
    /// Initialize from format data with runtime context
    fn from_format_bridge(format_data: FormatData, context: &RuntimeContext) -> Result<Self>
    where
        Self: Sized;
}

/// Runtime context for format bridge operations
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    /// Memory providers available
    pub memory_providers: Vec<RuntimeMemoryProvider>,
    /// Table providers available  
    pub table_providers: Vec<RuntimeTableProvider>,
    /// Maximum initialization steps allowed
    pub max_initialization_steps: usize,
    /// ASIL level constraints
    pub asil_constraints: ASILConstraints,
}

/// Memory provider for runtime operations
#[derive(Debug, Clone)]
pub struct RuntimeMemoryProvider {
    /// Memory index
    pub index: u32,
    /// Current size in pages
    pub current_size: u32,
    /// Maximum size in pages
    pub max_size: Option<u32>,
    /// Whether memory is shared
    pub is_shared: bool,
}

/// Table provider for runtime operations
#[derive(Debug, Clone)]
pub struct RuntimeTableProvider {
    /// Table index
    pub index: u32,
    /// Current size in elements
    pub current_size: u32,
    /// Maximum size in elements
    pub max_size: Option<u32>,
    /// Element type
    pub element_type: RuntimeRefType,
}

/// Runtime reference type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRefType {
    /// Function reference
    FuncRef,
    /// External reference
    ExternRef,
}

/// ASIL safety constraints
#[derive(Debug, Clone)]
pub struct ASILConstraints {
    /// ASIL level (A, B, C, D)
    pub level: ASILLevel,
    /// Maximum memory allocation allowed
    pub max_memory_allocation: usize,
    /// Maximum initialization time allowed (in microseconds)
    pub max_initialization_time_us: u64,
    /// Whether dynamic allocation is allowed
    pub allow_dynamic_allocation: bool,
}

/// ASIL safety level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ASILLevel {
    /// ASIL-A (lowest safety requirements)
    A,
    /// ASIL-B (moderate safety requirements)
    B,
    /// ASIL-C (high safety requirements)
    C,
    /// ASIL-D (highest safety requirements)
    D,
}

/// Runtime data segment initialized from format bridge
#[derive(Debug, Clone)]
pub struct RuntimeDataSegment {
    /// Segment index
    pub index: u32,
    /// Memory index for active segments
    pub memory_index: Option<u32>,
    /// Evaluated offset (runtime computed)
    pub evaluated_offset: Option<u32>,
    /// Data bytes (owned by runtime)
    pub data: Vec<u8>,
    /// Initialization state
    pub initialization_state: InitializationState,
    /// Runtime handle for tracking
    pub runtime_handle: u32,
}

/// Runtime element segment initialized from format bridge
#[derive(Debug, Clone)]
pub struct RuntimeElementSegment {
    /// Segment index
    pub index: u32,
    /// Table index for active segments
    pub table_index: Option<u32>,
    /// Evaluated offset (runtime computed)
    pub evaluated_offset: Option<u32>,
    /// Element data (function indices or evaluated expressions)
    pub elements: Vec<RuntimeElement>,
    /// Initialization state
    pub initialization_state: InitializationState,
    /// Runtime handle for tracking
    pub runtime_handle: u32,
}

/// Runtime element data
#[derive(Debug, Clone)]
pub enum RuntimeElement {
    /// Function index
    FunctionIndex(u32),
    /// Evaluated reference (from expression)
    EvaluatedRef(RuntimeReference),
}

/// Runtime reference
#[derive(Debug, Clone)]
pub struct RuntimeReference {
    /// Reference type
    pub ref_type: RuntimeRefType,
    /// Runtime handle to the referenced object
    pub handle: u32,
}

/// Initialization state for runtime segments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitializationState {
    /// Not yet initialized
    Pending,
    /// Currently being initialized
    InProgress,
    /// Successfully initialized
    Completed,
    /// Initialization failed
    Failed,
}

/// Runtime module initialization manager
#[derive(Debug)]
pub struct RuntimeModuleInitializer {
    /// Runtime context
    pub context: RuntimeContext,
    /// Data segments being managed
    pub data_segments: Vec<RuntimeDataSegment>,
    /// Element segments being managed
    pub element_segments: Vec<RuntimeElementSegment>,
    /// Start function index
    pub start_function: Option<u32>,
    /// Current initialization step
    pub current_step: usize,
    /// Total steps required
    pub total_steps: usize,
}

impl RuntimeModuleInitializer {
    /// Create new initializer with runtime context
    pub fn new(context: RuntimeContext) -> Self {
        Self {
            context,
            data_segments: Vec::new(),
            element_segments: Vec::new(),
            start_function: None,
            current_step: 0,
            total_steps: 0,
        }
    }
    
    /// Initialize from format module runtime data
    pub fn initialize_from_format(
        &mut self,
        runtime_data: FormatModuleRuntimeData,
    ) -> Result<()> {
        // Validate ASIL constraints
        self.validate_asil_constraints(&runtime_data)?;
        
        // Set total steps
        self.total_steps = runtime_data.estimated_initialization_steps;
        
        // Initialize data segments
        for (index, data_extraction) in runtime_data.data_extractions.into_iter().enumerate() {
            let runtime_segment = self.create_runtime_data_segment(index as u32, data_extraction)?;
            self.data_segments.push(runtime_segment);
        }
        
        // Initialize element segments
        for (index, element_extraction) in runtime_data.element_extractions.into_iter().enumerate() {
            let runtime_segment = self.create_runtime_element_segment(index as u32, element_extraction)?;
            self.element_segments.push(runtime_segment);
        }
        
        // Set start function
        self.start_function = runtime_data.start_function;
        
        Ok(())
    }
    
    /// Execute initialization process
    pub fn execute_initialization(&mut self) -> Result<()> {
        // Initialize active data segments - collect indices first to avoid borrowing conflicts
        let data_indices: Vec<usize> = self.data_segments
            .iter()
            .enumerate()
            .filter_map(|(i, segment)| if segment.memory_index.is_some() { Some(i) } else { None })
            .collect();
        
        for idx in data_indices {
            if let Some(segment) = self.data_segments.get_mut(idx) {
                Self::initialize_data_segment_static(segment)?;
                self.current_step += 1;
            }
        }
        
        // Initialize active element segments - collect indices first to avoid borrowing conflicts
        let element_indices: Vec<usize> = self.element_segments
            .iter()
            .enumerate()
            .filter_map(|(i, segment)| if segment.table_index.is_some() { Some(i) } else { None })
            .collect();
        
        for idx in element_indices {
            if let Some(segment) = self.element_segments.get_mut(idx) {
                Self::initialize_element_segment_static(segment)?;
                self.current_step += 1;
            }
        }
        
        // Call start function if present
        if let Some(start_fn) = self.start_function {
            self.call_start_function(start_fn)?;
            self.current_step += 1;
        }
        
        Ok(())
    }
    
    /// Validate ASIL constraints
    fn validate_asil_constraints(&self, runtime_data: &FormatModuleRuntimeData) -> Result<()> {
        // Check initialization step limits
        if runtime_data.estimated_initialization_steps > self.context.max_initialization_steps {
            return Err(Error::resource_exhausted("Too many initialization steps for ASIL constraints";
        }
        
        // Check memory allocation limits for ASIL-D
        if matches!(self.context.asil_constraints.level, ASILLevel::D) {
            let total_data_size: usize = runtime_data.data_extractions
                .iter()
                .map(|e| e.data_size)
                .sum);
                
            if total_data_size > self.context.asil_constraints.max_memory_allocation {
                return Err(Error::resource_exhausted("Data size exceeds ASIL-D memory limits";
            }
        }
        
        Ok(())
    }
    
    /// Create runtime data segment from format extraction
    fn create_runtime_data_segment(
        &self,
        index: u32,
        extraction: FormatDataExtraction,
    ) -> Result<RuntimeDataSegment> {
        Ok(RuntimeDataSegment {
            index,
            memory_index: extraction.memory_index,
            evaluated_offset: None, // Will be computed during initialization
            data: Vec::new(), // Will be populated during initialization
            initialization_state: InitializationState::Pending,
            runtime_handle: self.generate_runtime_handle(),
        })
    }
    
    /// Create runtime element segment from format extraction
    fn create_runtime_element_segment(
        &self,
        index: u32,
        extraction: FormatElementExtraction,
    ) -> Result<RuntimeElementSegment> {
        Ok(RuntimeElementSegment {
            index,
            table_index: extraction.table_index,
            evaluated_offset: None, // Will be computed during initialization
            elements: Vec::new(), // Will be populated during initialization
            initialization_state: InitializationState::Pending,
            runtime_handle: self.generate_runtime_handle(),
        })
    }
    
    /// Initialize a data segment
    fn initialize_data_segment(&mut self, segment: &mut RuntimeDataSegment) -> Result<()> {
        Self::initialize_data_segment_static(segment)
    }
    
    /// Initialize a data segment (static version to avoid borrowing conflicts)
    fn initialize_data_segment_static(segment: &mut RuntimeDataSegment) -> Result<()> {
        segment.initialization_state = InitializationState::InProgress;
        
        // Runtime-specific initialization logic would go here
        // For now, just mark as completed
        segment.initialization_state = InitializationState::Completed;
        
        Ok(())
    }
    
    /// Initialize an element segment
    fn initialize_element_segment(&mut self, segment: &mut RuntimeElementSegment) -> Result<()> {
        Self::initialize_element_segment_static(segment)
    }
    
    /// Initialize an element segment (static version to avoid borrowing conflicts)
    fn initialize_element_segment_static(segment: &mut RuntimeElementSegment) -> Result<()> {
        segment.initialization_state = InitializationState::InProgress;
        
        // Runtime-specific initialization logic would go here
        // For now, just mark as completed
        segment.initialization_state = InitializationState::Completed;
        
        Ok(())
    }
    
    /// Call start function
    fn call_start_function(&mut self, _start_fn: u32) -> Result<()> {
        // Runtime-specific start function execution would go here
        Ok(())
    }
    
    /// Generate unique runtime handle
    fn generate_runtime_handle(&self) -> u32 {
        // Simple handle generation (in real implementation would use proper allocation)
        (self.data_segments.len() + self.element_segments.len()) as u32
    }
}

/// Format module runtime data (received from wrt-format bridge)
#[derive(Debug, Clone)]
pub struct FormatModuleRuntimeData {
    /// Start function index
    pub start_function: Option<u32>,
    /// Data extraction results
    pub data_extractions: Vec<FormatDataExtraction>,
    /// Element extraction results
    pub element_extractions: Vec<FormatElementExtraction>,
    /// Estimated initialization steps
    pub estimated_initialization_steps: usize,
}

/// Format data extraction (received from wrt-format bridge)
#[derive(Debug, Clone)]
pub struct FormatDataExtraction {
    /// Memory index for active segments
    pub memory_index: Option<u32>,
    /// Raw offset expression bytes
    pub offset_expr_bytes: Vec<u8>,
    /// Data size
    pub data_size: usize,
    /// Whether initialization is required
    pub requires_initialization: bool,
}

/// Format element extraction (received from wrt-format bridge)
#[derive(Debug, Clone)]
pub struct FormatElementExtraction {
    /// Table index for active segments
    pub table_index: Option<u32>,
    /// Raw offset expression bytes
    pub offset_expr_bytes: Vec<u8>,
    /// Element initialization type
    pub init_data_type: FormatElementInitType,
    /// Whether initialization is required
    pub requires_initialization: bool,
}

/// Format element initialization type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatElementInitType {
    /// Function indices
    FunctionIndices,
    /// Expression bytes
    ExpressionBytes,
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self {
            memory_providers: Vec::new(),
            table_providers: Vec::new(),
            max_initialization_steps: 1000,
            asil_constraints: ASILConstraints::default(),
        }
    }
}

impl Default for ASILConstraints {
    fn default() -> Self {
        Self {
            level: ASILLevel::A,
            max_memory_allocation: 1024 * 1024, // 1MB default
            max_initialization_time_us: 10_000, // 10ms default
            allow_dynamic_allocation: true,
        }
    }
}