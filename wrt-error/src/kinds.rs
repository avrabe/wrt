// Define a custom string-like type that works in all environments
#[cfg(feature = "std")]
extern crate std;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Import ToString trait implementations
#[cfg(feature = "std")]
use std::string::ToString;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::string::ToString;

// Use alloc String if alloc is enabled but not std
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use alloc::format;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use alloc::string::String;

// Use std String if std is enabled
#[cfg(feature = "std")]
pub use std::format;
#[cfg(feature = "std")]
pub use std::string::String;

/// A minimal string type for no-std/no-alloc environments.
#[cfg(not(any(feature = "std", feature = "alloc")))]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct String {
    _private: (),
}

#[cfg(not(any(feature = "std", feature = "alloc")))]
impl String {
    /// Creates a new empty string (no-std, no-alloc).
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Utility method to create a String from a static str (no-std, no-alloc).
    pub fn from_static(_msg: &'static str) -> Self {
        Self::new()
    }
}

// Implement From for &'static str in no_std mode - just creates an empty placeholder
#[cfg(not(any(feature = "std", feature = "alloc")))]
impl From<&'static str> for String {
    fn from(_value: &'static str) -> Self {
        Self::new()
    }
}

/// Validation error for integrity or consistency checks
#[derive(Debug, Clone)]
pub struct ValidationError(pub String);

/// Out of bounds error for memory access or index violations
#[derive(Debug, Clone)]
pub struct OutOfBoundsError(pub String);

/// Parse error for decoding binary formats
#[derive(Debug, Clone)]
pub struct ParseError(pub String);

/// Type error for type mismatches or invalid types
#[derive(Debug, Clone)]
pub struct InvalidType(pub String);

/// Conversion error
#[derive(Debug, Clone)]
pub struct ConversionError(pub String);

/// Division by zero error
#[derive(Debug, Clone)]
pub struct DivisionByZeroError;

/// Integer overflow error
#[derive(Debug, Clone)]
pub struct IntegerOverflowError;

/// Stack underflow error
#[derive(Debug, Clone)]
pub struct StackUnderflow;

/// Type mismatch error
#[derive(Debug, Clone)]
pub struct TypeMismatch(pub String);

/// Invalid table index error
#[derive(Debug, Clone)]
pub struct InvalidTableIndexError(pub u32);

/// Invalid local index error
#[derive(Debug, Clone)]
pub struct InvalidLocalIndexError(pub u32);

/// Resource error for resource access or creation issues
#[derive(Debug, Clone)]
pub struct ResourceError(pub String);

/// Component error for component instantiation or linking issues
#[derive(Debug, Clone)]
pub struct ComponentError(pub String);

/// Runtime error for generic execution issues
#[derive(Debug, Clone)]
pub struct RuntimeError(pub String);

/// Poisoned lock error for mutex failures
#[derive(Debug, Clone)]
pub struct PoisonedLockError(pub String);

/// Memory access out of bounds error
#[derive(Debug, Clone)]
pub struct MemoryAccessOutOfBoundsError {
    /// The memory address that was accessed
    pub address: u64,
    /// The length of memory being accessed
    pub length: u64,
}

/// Type mismatch error
#[derive(Debug, Clone)]
pub struct TypeMismatchError(pub String);

/// Table access out of bounds error
#[derive(Debug, Clone)]
pub struct TableAccessOutOfBounds;

/// Arithmetic error for math operations
#[derive(Debug, Clone)]
pub struct ArithmeticError(pub String);

/// Memory access error
#[derive(Debug, Clone)]
pub struct MemoryAccessError(pub String);

/// Resource exhaustion error
#[derive(Debug, Clone)]
pub struct ResourceExhaustionError(pub String);

/// Invalid index error
#[derive(Debug, Clone)]
pub struct InvalidIndexError(pub String);

/// Error during execution
#[derive(Debug, Clone)]
pub struct ExecutionError(pub String);

/// Stack underflow error (empty stack)
#[derive(Debug, Clone)]
pub struct StackUnderflowError(pub String);

/// Export not found error
#[derive(Debug, Clone)]
pub struct ExportNotFoundError(pub String);

/// Invalid instance index error
#[derive(Debug, Clone)]
pub struct InvalidInstanceIndexError(pub u32);

/// Invalid function index error
#[derive(Debug, Clone)]
pub struct InvalidFunctionIndexError(pub u32);

/// Invalid element index error
#[derive(Debug, Clone)]
pub struct InvalidElementIndexError(pub usize);

/// Invalid memory index error
#[derive(Debug, Clone)]
pub struct InvalidMemoryIndexError(pub u32);

/// Invalid global index error
#[derive(Debug, Clone)]
pub struct InvalidGlobalIndexError(pub u32);

/// Invalid data segment index error
#[derive(Debug, Clone)]
pub struct InvalidDataSegmentIndexError(pub usize);

/// Invalid function type error
#[derive(Debug, Clone)]
pub struct InvalidFunctionTypeError(pub String);

/// Not implemented error
#[derive(Debug, Clone)]
pub struct NotImplementedError(pub String);

/// Out of bounds access error
#[derive(Debug, Clone)]
pub struct OutOfBoundsAccess(pub String);

/// Invalid value error
#[derive(Debug, Clone)]
pub struct InvalidValue(pub String);

/// Value out of range error
#[derive(Debug, Clone)]
pub struct ValueOutOfRangeError(pub String);

/// Invalid state error
#[derive(Debug, Clone)]
pub struct InvalidState(pub String);

/// Decoding error
#[derive(Debug, Clone)]
pub struct DecodingError(pub String);

/// Execution limit exceeded error
#[derive(Debug, Clone)]
pub struct ExecutionLimitExceeded(pub String);

/// Execution timeout error
#[derive(Debug, Clone)]
pub struct ExecutionTimeoutError(pub String);

/// Resource limit exceeded error
#[derive(Debug, Clone)]
pub struct ResourceLimitExceeded(pub String);

/// Invalid argument error
#[derive(Debug, Clone)]
pub struct InvalidArgumentError(pub String);

/// Error when a Wasm 3.0 specific construct is encountered in a Wasm 2.0 module or context.
#[derive(Debug, Clone)]
pub struct UnsupportedWasm30ConstructInWasm20Module {
    /// Name or description of the Wasm 3.0 construct.
    pub construct_name: String,
}

/// Error for malformed or invalid immediates for a Wasm 3.0 instruction.
#[derive(Debug, Clone)]
pub struct InvalidWasm30InstructionImmediate {
    /// Name of the Wasm 3.0 instruction.
    pub instruction: String,
}

/// Error for a malformed Wasm 3.0 `TypeInformation` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedWasm30TypeInformationSection(pub String);

/// Error for an invalid memory index used with Wasm 3.0 multi-memory features.
#[derive(Debug, Clone)]
pub struct InvalidMemoryIndexWasm30 {
    /// The invalid memory index that was used.
    pub index: u32,
    /// The maximum number of allowed memories (if applicable).
    pub max_memories: u32,
}

/// Error for an unknown opcode for the detected/specified Wasm version.
#[derive(Debug, Clone)]
pub struct UnknownOpcodeForVersion {
    /// Major version number of Wasm (e.g., 2 for Wasm 2.0, 3 for Wasm 3.0).
    pub version_major: u16,
    /// Minor version number of Wasm.
    pub version_minor: u16,
    /// The first byte of the opcode.
    pub opcode_byte1: u8,
    /// The second byte of the opcode, if it's a multi-byte opcode.
    pub opcode_byte2: Option<u8>,
}

/// Error for an invalid import/export kind byte for the detected/specified Wasm version.
#[derive(Debug, Clone)]
pub struct InvalidImportExportKindForVersion {
    /// Major version number of Wasm.
    pub version_major: u16,
    /// Minor version number of Wasm.
    pub version_minor: u16,
    /// The kind byte that was encountered.
    pub kind_byte: u8,
}

/// Helper function for creating `ValidationError`
pub fn validation_error(message: impl Into<String>) -> ValidationError {
    ValidationError(message.into())
}

/// Helper function for creating `OutOfBoundsError`
pub fn out_of_bounds_error(message: impl Into<String>) -> OutOfBoundsError {
    OutOfBoundsError(message.into())
}

/// Helper function for creating `ParseError`
pub fn parse_error(message: impl Into<String>) -> ParseError {
    ParseError(message.into())
}

/// Helper function for creating `InvalidType`
pub fn invalid_type(message: impl Into<String>) -> InvalidType {
    InvalidType(message.into())
}

/// Helper function for creating `ConversionError`
pub fn conversion_error(message: impl Into<String>) -> ConversionError {
    ConversionError(message.into())
}

/// Helper function for creating `DivisionByZeroError`
#[must_use]
pub const fn division_by_zero_error() -> DivisionByZeroError {
    DivisionByZeroError
}

/// Helper function for creating `IntegerOverflowError`
#[must_use]
pub const fn integer_overflow_error() -> IntegerOverflowError {
    IntegerOverflowError
}

/// Helper function for creating `StackUnderflow`
#[must_use]
pub const fn stack_underflow() -> StackUnderflow {
    StackUnderflow
}

/// Helper function for creating `TypeMismatch`
pub fn type_mismatch(message: impl Into<String>) -> TypeMismatch {
    TypeMismatch(message.into())
}

/// Helper function for creating `InvalidTableIndexError`
#[must_use]
pub const fn invalid_table_index_error(index: u32) -> InvalidTableIndexError {
    InvalidTableIndexError(index)
}

/// Helper function for creating `ResourceError`
pub fn resource_error(message: impl Into<String>) -> ResourceError {
    ResourceError(message.into())
}

/// Helper function for creating `ComponentError`
pub fn component_error(message: impl Into<String>) -> ComponentError {
    ComponentError(message.into())
}

/// Helper function for creating `RuntimeError`
pub fn runtime_error(message: impl Into<String>) -> RuntimeError {
    RuntimeError(message.into())
}

/// Helper function for creating `PoisonedLockError`
pub fn poisoned_lock_error(message: impl Into<String>) -> PoisonedLockError {
    PoisonedLockError(message.into())
}

/// Helper function for creating `TypeMismatchError`
pub fn type_mismatch_error(message: impl Into<String>) -> TypeMismatchError {
    TypeMismatchError(message.into())
}

/// Helper function for creating `ArithmeticError`
pub fn arithmetic_error(message: impl Into<String>) -> ArithmeticError {
    ArithmeticError(message.into())
}

/// Helper function for creating `MemoryAccessError`
pub fn memory_access_error(message: impl Into<String>) -> MemoryAccessError {
    MemoryAccessError(message.into())
}

/// Helper function for creating `ResourceExhaustionError`
pub fn resource_exhaustion_error(message: impl Into<String>) -> ResourceExhaustionError {
    ResourceExhaustionError(message.into())
}

/// Helper function for creating `InvalidIndexError`
pub fn invalid_index_error(message: impl Into<String>) -> InvalidIndexError {
    InvalidIndexError(message.into())
}

/// Helper function for creating `ExecutionError`
pub fn execution_error(message: impl Into<String>) -> ExecutionError {
    ExecutionError(message.into())
}

/// Helper function for creating `StackUnderflowError`
pub fn stack_underflow_error(message: impl Into<String>) -> StackUnderflowError {
    StackUnderflowError(message.into())
}

/// Helper function for creating `ExportNotFoundError`
pub fn export_not_found_error(name: impl Into<String>) -> ExportNotFoundError {
    ExportNotFoundError(name.into())
}

/// Helper function for creating `InvalidInstanceIndexError`
#[must_use]
pub const fn invalid_instance_index_error(index: u32) -> InvalidInstanceIndexError {
    InvalidInstanceIndexError(index)
}

/// Helper function for creating `InvalidFunctionIndexError`
#[must_use]
pub const fn invalid_function_index_error(index: u32) -> InvalidFunctionIndexError {
    InvalidFunctionIndexError(index)
}

/// Helper function for creating `InvalidElementIndexError`
#[must_use]
pub const fn invalid_element_index_error(index: usize) -> InvalidElementIndexError {
    InvalidElementIndexError(index)
}

/// Helper function for creating `InvalidMemoryIndexError`
#[must_use]
pub const fn invalid_memory_index_error(index: u32) -> InvalidMemoryIndexError {
    InvalidMemoryIndexError(index)
}

/// Helper function for creating `InvalidGlobalIndexError`
#[must_use]
pub const fn invalid_global_index_error(index: u32) -> InvalidGlobalIndexError {
    InvalidGlobalIndexError(index)
}

/// Helper function for creating `InvalidDataSegmentIndexError`
#[must_use]
pub const fn invalid_data_segment_index_error(index: usize) -> InvalidDataSegmentIndexError {
    InvalidDataSegmentIndexError(index)
}

/// Helper function for creating `InvalidFunctionTypeError`
pub fn invalid_function_type_error(message: impl Into<String>) -> InvalidFunctionTypeError {
    InvalidFunctionTypeError(message.into())
}

/// Helper function for creating `NotImplementedError`
pub fn not_implemented_error(message: impl Into<String>) -> NotImplementedError {
    NotImplementedError(message.into())
}

/// Helper function for creating `OutOfBoundsAccess`
pub fn out_of_bounds_access(message: impl Into<String>) -> OutOfBoundsAccess {
    OutOfBoundsAccess(message.into())
}

/// Helper function for creating `InvalidValue`
pub fn invalid_value(message: impl Into<String>) -> InvalidValue {
    InvalidValue(message.into())
}

/// Helper function for creating `ValueOutOfRangeError`
pub fn value_out_of_range_error(message: impl Into<String>) -> ValueOutOfRangeError {
    ValueOutOfRangeError(message.into())
}

/// Helper function for creating `InvalidState`
pub fn invalid_state(message: impl Into<String>) -> InvalidState {
    InvalidState(message.into())
}

/// Helper function for creating `DecodingError`
pub fn decoding_error(message: impl Into<String>) -> DecodingError {
    DecodingError(message.into())
}

/// Helper function for creating `ExecutionLimitExceeded`
pub fn execution_limit_exceeded(message: impl Into<String>) -> ExecutionLimitExceeded {
    ExecutionLimitExceeded(message.into())
}

/// Helper function for creating `ExecutionTimeoutError`
pub fn execution_timeout_error(message: impl Into<String>) -> ExecutionTimeoutError {
    ExecutionTimeoutError(message.into())
}

/// Helper function for creating `ResourceLimitExceeded`
pub fn resource_limit_exceeded(message: impl Into<String>) -> ResourceLimitExceeded {
    ResourceLimitExceeded(message.into())
}

/// Helper function for creating `InvalidArgumentError`
pub fn invalid_argument_error(message: impl Into<String>) -> InvalidArgumentError {
    InvalidArgumentError(message.into())
}

/// Helper function for creating `UnsupportedWasm30ConstructInWasm20Module`
pub fn unsupported_wasm30_construct_in_wasm20_module(
    construct_name: impl Into<String>,
) -> UnsupportedWasm30ConstructInWasm20Module {
    UnsupportedWasm30ConstructInWasm20Module { construct_name: construct_name.into() }
}

/// Helper function for creating `InvalidWasm30InstructionImmediate`
pub fn invalid_wasm30_instruction_immediate(
    instruction: impl Into<String>,
) -> InvalidWasm30InstructionImmediate {
    InvalidWasm30InstructionImmediate { instruction: instruction.into() }
}

/// Helper function for creating `MalformedWasm30TypeInformationSection`
pub fn malformed_wasm30_type_information_section(
    message: impl Into<String>,
) -> MalformedWasm30TypeInformationSection {
    MalformedWasm30TypeInformationSection(message.into())
}

/// Helper function for creating `InvalidMemoryIndexWasm30`
#[must_use]
pub const fn invalid_memory_index_wasm30(
    index: u32,
    max_memories: u32,
) -> InvalidMemoryIndexWasm30 {
    InvalidMemoryIndexWasm30 { index, max_memories }
}

/// Helper function for creating `UnknownOpcodeForVersion`
#[must_use]
pub const fn unknown_opcode_for_version(
    version_major: u16,
    version_minor: u16,
    opcode_byte1: u8,
    opcode_byte2: Option<u8>,
) -> UnknownOpcodeForVersion {
    UnknownOpcodeForVersion { version_major, version_minor, opcode_byte1, opcode_byte2 }
}

/// Helper function for creating `InvalidImportExportKindForVersion`
#[must_use]
pub const fn invalid_import_export_kind_for_version(
    version_major: u16,
    version_minor: u16,
    kind_byte: u8,
) -> InvalidImportExportKindForVersion {
    InvalidImportExportKindForVersion { version_major, version_minor, kind_byte }
}

/// Implementation of the Display trait for `ValidationError`
impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Validation error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Validation error");
    }
}

/// Implementation of the Display trait for `OutOfBoundsError`
impl core::fmt::Display for OutOfBoundsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Out of bounds error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Out of bounds error");
    }
}

/// Implementation of the Display trait for `ParseError`
impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Parse error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Parse error");
    }
}

/// Implementation of the Display trait for `InvalidType`
impl core::fmt::Display for InvalidType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Invalid type: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Invalid type");
    }
}

/// Implementation of the Display trait for `ConversionError`
impl core::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Conversion error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Conversion error");
    }
}

/// Implementation of the Display trait for `DivisionByZeroError`
impl core::fmt::Display for DivisionByZeroError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Division by zero error")
    }
}

/// Implementation of the Display trait for `IntegerOverflowError`
impl core::fmt::Display for IntegerOverflowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Integer overflow error")
    }
}

/// Implementation of the Display trait for `StackUnderflow`
impl core::fmt::Display for StackUnderflow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Stack underflow")
    }
}

/// Implementation of the Display trait for `TypeMismatch`
impl core::fmt::Display for TypeMismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Type mismatch: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Type mismatch");
    }
}

/// Implementation of the Display trait for `InvalidTableIndexError`
impl core::fmt::Display for InvalidTableIndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid table index: {}", self.0)
    }
}

/// Implementation of the Display trait for `ResourceError`
impl core::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Resource error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Resource error");
    }
}

/// Implementation of the Display trait for `ComponentError`
impl core::fmt::Display for ComponentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Component error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Component error");
    }
}

/// Implementation of the Display trait for `RuntimeError`
impl core::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Runtime error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Runtime error");
    }
}

/// Implementation of the Display trait for `PoisonedLockError`
impl core::fmt::Display for PoisonedLockError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Poisoned lock error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Poisoned lock error");
    }
}

/// Implementation of the Display trait for `MemoryAccessOutOfBoundsError`
impl core::fmt::Display for MemoryAccessOutOfBoundsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(
            f,
            "Memory access out of bounds: address 0x{:x}, length {}",
            self.address, self.length
        );

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Memory access out of bounds");
    }
}

/// Implementation of the Display trait for `TypeMismatchError`
impl core::fmt::Display for TypeMismatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Type mismatch: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Type mismatch");
    }
}

/// Implementation of the Display trait for `TableAccessOutOfBounds`
impl core::fmt::Display for TableAccessOutOfBounds {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Table access out of bounds")
    }
}

/// Implementation of the Display trait for `ArithmeticError`
impl core::fmt::Display for ArithmeticError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Arithmetic error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Arithmetic error");
    }
}

/// Implementation of the Display trait for `MemoryAccessError`
impl core::fmt::Display for MemoryAccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Memory access error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Memory access error");
    }
}

/// Implementation of the Display trait for `ResourceExhaustionError`
impl core::fmt::Display for ResourceExhaustionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Resource exhaustion error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Resource exhaustion error");
    }
}

/// Implementation of the Display trait for `InvalidIndexError`
impl core::fmt::Display for InvalidIndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Invalid index error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Invalid index error");
    }
}

/// Implementation of the Display trait for `ExecutionError`
impl core::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Execution error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Execution error");
    }
}

/// Implementation of the Display trait for `StackUnderflowError`
impl core::fmt::Display for StackUnderflowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Stack underflow error: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Stack underflow error");
    }
}

/// Implementation of the Display trait for `ExportNotFoundError`
impl core::fmt::Display for ExportNotFoundError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Export not found: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Export not found");
    }
}

/// Implementation of the Display trait for `InvalidInstanceIndexError`
impl core::fmt::Display for InvalidInstanceIndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid instance index: {}", self.0)
    }
}

/// Implementation of the Display trait for `InvalidFunctionIndexError`
impl core::fmt::Display for InvalidFunctionIndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid function index: {}", self.0)
    }
}

/// Implementation of the Display trait for `InvalidElementIndexError`
impl core::fmt::Display for InvalidElementIndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid element index: {}", self.0)
    }
}

/// Implementation of the Display trait for `InvalidMemoryIndexError`
impl core::fmt::Display for InvalidMemoryIndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid memory index: {}", self.0)
    }
}

/// Implementation of the Display trait for `InvalidGlobalIndexError`
impl core::fmt::Display for InvalidGlobalIndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid global index: {}", self.0)
    }
}

/// Implementation of the Display trait for `InvalidDataSegmentIndexError`
impl core::fmt::Display for InvalidDataSegmentIndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid data segment index: {}", self.0)
    }
}

/// Implementation of the Display trait for `InvalidFunctionTypeError`
impl core::fmt::Display for InvalidFunctionTypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Invalid function type: {}", self.0);

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Invalid function type");
    }
}

/// Implementation of the Display trait for `NotImplementedError`
impl core::fmt::Display for NotImplementedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Feature not implemented: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Feature not implemented");
    }
}

/// Implementation of the Display trait for `OutOfBoundsAccess`
impl core::fmt::Display for OutOfBoundsAccess {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Out of bounds access: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Out of bounds access");
    }
}

/// Implementation of the Display trait for `InvalidValue`
impl core::fmt::Display for InvalidValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Invalid value: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Invalid value");
    }
}

/// Implementation of the Display trait for `ValueOutOfRangeError`
impl core::fmt::Display for ValueOutOfRangeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Value out of range: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Value out of range");
    }
}

/// Implementation of the Display trait for `InvalidState`
impl core::fmt::Display for InvalidState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Invalid state: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Invalid state");
    }
}

/// Implementation of the Display trait for `DecodingError`
impl core::fmt::Display for DecodingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Decoding error: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Decoding error");
    }
}

/// Implementation of the Display trait for `ExecutionLimitExceeded`
impl core::fmt::Display for ExecutionLimitExceeded {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Execution limit exceeded: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Execution limit exceeded");
    }
}

/// Implementation of the Display trait for `ExecutionTimeoutError`
impl core::fmt::Display for ExecutionTimeoutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Execution timeout: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Execution timeout");
    }
}

/// Implementation of the Display trait for `ResourceLimitExceeded`
impl core::fmt::Display for ResourceLimitExceeded {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Resource limit exceeded: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Resource limit exceeded");
    }
}

/// Implementation of the Display trait for `InvalidArgumentError`
impl core::fmt::Display for InvalidArgumentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        return write!(f, "Invalid argument: {}", self.0);
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        return write!(f, "Invalid argument");
    }
}

/// Implementation of the Display trait for `UnsupportedWasm30ConstructInWasm20Module`
impl core::fmt::Display for UnsupportedWasm30ConstructInWasm20Module {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        write!(f, "Unsupported Wasm 3.0 construct in Wasm 2.0 context: {}", self.construct_name)?;
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        write!(f, "Unsupported Wasm 3.0 construct in Wasm 2.0 context")?;
        Ok(())
    }
}

/// Implementation of the Display trait for `InvalidWasm30InstructionImmediate`
impl core::fmt::Display for InvalidWasm30InstructionImmediate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        write!(f, "Invalid immediate for Wasm 3.0 instruction: {}", self.instruction)?;
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        write!(f, "Invalid immediate for Wasm 3.0 instruction")?;
        Ok(())
    }
}

/// Implementation of the Display trait for `MalformedWasm30TypeInformationSection`
impl core::fmt::Display for MalformedWasm30TypeInformationSection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        write!(f, "Malformed Wasm 3.0 `TypeInformation` section: {}", self.0)?;
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        write!(f, "Malformed Wasm 3.0 `TypeInformation` section")?;
        Ok(())
    }
}

/// Implementation of the Display trait for `InvalidMemoryIndexWasm30`
impl core::fmt::Display for InvalidMemoryIndexWasm30 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Invalid memory index {} used with Wasm 3.0 (max memories: {})",
            self.index, self.max_memories
        )
    }
}

/// Implementation of the Display trait for `UnknownOpcodeForVersion`
impl core::fmt::Display for UnknownOpcodeForVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(byte2) = self.opcode_byte2 {
            write!(
                f,
                "Unknown opcode (0x{:02X} 0x{:02X}) for Wasm version {}.{}",
                self.opcode_byte1, byte2, self.version_major, self.version_minor
            )
        } else {
            write!(
                f,
                "Unknown opcode (0x{:02X}) for Wasm version {}.{}",
                self.opcode_byte1, self.version_major, self.version_minor
            )
        }
    }
}

/// Implementation of the Display trait for `InvalidImportExportKindForVersion`
impl core::fmt::Display for InvalidImportExportKindForVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Invalid import/export kind byte (0x{:02X}) for Wasm version {}.{}",
            self.kind_byte, self.version_major, self.version_minor
        )
    }
}

// Add From implementations for the new error types

#[cfg(feature = "alloc")]
impl From<ConversionError> for String {
    fn from(e: ConversionError) -> Self {
        e.to_string()
    }
}

#[cfg(feature = "alloc")]
impl From<DivisionByZeroError> for String {
    fn from(e: DivisionByZeroError) -> Self {
        e.to_string()
    }
}

#[cfg(feature = "alloc")]
impl From<IntegerOverflowError> for String {
    fn from(e: IntegerOverflowError) -> Self {
        e.to_string()
    }
}

#[cfg(feature = "alloc")]
impl From<StackUnderflow> for String {
    fn from(e: StackUnderflow) -> Self {
        e.to_string()
    }
}

#[cfg(feature = "alloc")]
impl From<TypeMismatch> for String {
    fn from(e: TypeMismatch) -> Self {
        e.to_string()
    }
}

#[cfg(feature = "alloc")]
impl From<InvalidTableIndexError> for String {
    fn from(e: InvalidTableIndexError) -> Self {
        e.to_string()
    }
}

#[cfg(feature = "alloc")]
impl From<ValidationError> for String {
    fn from(e: ValidationError) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<NotImplementedError> for String {
    fn from(e: NotImplementedError) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<OutOfBoundsAccess> for String {
    fn from(e: OutOfBoundsAccess) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<InvalidValue> for String {
    fn from(e: InvalidValue) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<ValueOutOfRangeError> for String {
    fn from(e: ValueOutOfRangeError) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<InvalidState> for String {
    fn from(e: InvalidState) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<DecodingError> for String {
    fn from(e: DecodingError) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<ExecutionLimitExceeded> for String {
    fn from(e: ExecutionLimitExceeded) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<ExecutionTimeoutError> for String {
    fn from(e: ExecutionTimeoutError) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<ResourceLimitExceeded> for String {
    fn from(e: ResourceLimitExceeded) -> Self {
        e.to_string()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl From<InvalidArgumentError> for String {
    fn from(e: InvalidArgumentError) -> Self {
        e.to_string()
    }
}

// --- START Wasm 2.0 Specific Errors ---

/// Error when a Wasm 2.0 specific construct is encountered in a context that does not support it (e.g. Wasm 1.0).
#[derive(Debug, Clone)]
pub struct UnsupportedWasm20Feature {
    /// Name or description of the Wasm 2.0 feature.
    pub feature_name: String,
}

/// Error for malformed or invalid usage of reference types (externref, funcref).
#[derive(Debug, Clone)]
pub struct InvalidReferenceTypeUsage {
    /// Detailed message about the invalid usage.
    pub message: String,
}

/// Error related to Wasm 2.0 bulk memory or table operations.
#[derive(Debug, Clone)]
pub struct BulkOperationError {
    /// Name of the bulk operation instruction (e.g., "memory.copy", "table.init").
    pub operation_name: String,
    /// Detailed message about the error.
    pub reason: String,
}

/// Error specific to Wasm 2.0 SIMD operations.
#[derive(Debug, Clone)]
pub struct SimdOperationError {
    /// Name of the SIMD instruction.
    pub instruction_name: String,
    /// Detailed message about the SIMD error (e.g., invalid lane index, type mismatch).
    pub reason: String,
}

/// Error related to Wasm 2.0 tail call instructions.
#[derive(Debug, Clone)]
pub struct TailCallError {
    /// Detailed message about the tail call error.
    pub message: String,
}

// --- END Wasm 2.0 Specific Errors ---

// --- START Wasm 2.0 Factory Functions ---

/// Creates a new `UnsupportedWasm20Feature` error.
#[cfg(any(feature = "std", feature = "alloc"))]
pub fn unsupported_wasm20_feature(feature_name: impl Into<String>) -> UnsupportedWasm20Feature {
    UnsupportedWasm20Feature { feature_name: feature_name.into() }
}

/// Fallback: Creates a new `UnsupportedWasm20Feature` error (no-std, no-alloc).
#[cfg(not(any(feature = "std", feature = "alloc")))]
pub fn unsupported_wasm20_feature(
    _feature_name: impl core::fmt::Display,
) -> UnsupportedWasm20Feature {
    UnsupportedWasm20Feature { feature_name: String::new() }
}

/// Creates a new `InvalidReferenceTypeUsage` error.
#[cfg(any(feature = "std", feature = "alloc"))]
pub fn invalid_reference_type_usage(message: impl Into<String>) -> InvalidReferenceTypeUsage {
    InvalidReferenceTypeUsage { message: message.into() }
}

/// Fallback: Creates a new `InvalidReferenceTypeUsage` error (no-std, no-alloc).
#[cfg(not(any(feature = "std", feature = "alloc")))]
pub fn invalid_reference_type_usage(
    _message: impl core::fmt::Display,
) -> InvalidReferenceTypeUsage {
    InvalidReferenceTypeUsage { message: String::new() }
}

/// Creates a new `BulkOperationError` error.
#[cfg(any(feature = "std", feature = "alloc"))]
pub fn bulk_operation_error(
    operation_name: impl Into<String>,
    reason: impl Into<String>,
) -> BulkOperationError {
    BulkOperationError { operation_name: operation_name.into(), reason: reason.into() }
}

/// Fallback: Creates a new `BulkOperationError` error (no-std, no-alloc).
#[cfg(not(any(feature = "std", feature = "alloc")))]
pub fn bulk_operation_error(
    _operation_name: impl core::fmt::Display,
    _reason: impl core::fmt::Display,
) -> BulkOperationError {
    BulkOperationError { operation_name: String::new(), reason: String::new() }
}

/// Creates a new `SimdOperationError` error.
#[cfg(any(feature = "std", feature = "alloc"))]
pub fn simd_operation_error(
    instruction_name: impl Into<String>,
    reason: impl Into<String>,
) -> SimdOperationError {
    SimdOperationError { instruction_name: instruction_name.into(), reason: reason.into() }
}

/// Fallback: Creates a new `SimdOperationError` error (no-std, no-alloc).
#[cfg(not(any(feature = "std", feature = "alloc")))]
pub fn simd_operation_error(
    _instruction_name: impl core::fmt::Display,
    _reason: impl core::fmt::Display,
) -> SimdOperationError {
    SimdOperationError { instruction_name: String::new(), reason: String::new() }
}

/// Creates a new `TailCallError` error.
#[cfg(any(feature = "std", feature = "alloc"))]
pub fn tail_call_error(message: impl Into<String>) -> TailCallError {
    TailCallError { message: message.into() }
}

/// Fallback: Creates a new `TailCallError` error (no-std, no-alloc).
#[cfg(not(any(feature = "std", feature = "alloc")))]
pub fn tail_call_error(_message: impl core::fmt::Display) -> TailCallError {
    TailCallError { message: String::new() }
}

// --- END Wasm 2.0 Factory Functions ---

// --- START Wasm 2.0 Display Impls ---
impl core::fmt::Display for UnsupportedWasm20Feature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        write!(f, "Unsupported Wasm 2.0 feature: {}", self.feature_name)?;
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        write!(f, "Unsupported Wasm 2.0 feature")?;
        Ok(())
    }
}

impl core::fmt::Display for InvalidReferenceTypeUsage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        write!(f, "Invalid reference type usage: {}", self.message)?;
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        write!(f, "Invalid reference type usage")?;
        Ok(())
    }
}

impl core::fmt::Display for BulkOperationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        write!(f, "Bulk operation error in '{}': {}", self.operation_name, self.reason)?;
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        write!(f, "Bulk operation error")?;
        Ok(())
    }
}

impl core::fmt::Display for SimdOperationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        write!(f, "SIMD operation error in '{}': {}", self.instruction_name, self.reason)?;
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        write!(f, "SIMD operation error")?;
        Ok(())
    }
}

impl core::fmt::Display for TailCallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(feature = "std", feature = "alloc"))]
        write!(f, "Tail call error: {}", self.message)?;
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        write!(f, "Tail call error")?;
        Ok(())
    }
}
// --- END Wasm 2.0 Display Impls ---
