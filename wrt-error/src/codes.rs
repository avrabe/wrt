//! Error codes for WRT

/// Stack underflow error
pub const STACK_UNDERFLOW: u16 = 1000;
/// Stack overflow error
pub const STACK_OVERFLOW: u16 = 1001;
/// Unaligned memory access error
pub const UNALIGNED_MEMORY_ACCESS: u16 = 1002;
/// Invalid memory access error
pub const INVALID_MEMORY_ACCESS: u16 = 1003;
/// Invalid instance index error
pub const INVALID_INSTANCE_INDEX: u16 = 1004;
/// General execution error
pub const EXECUTION_ERROR: u16 = 1005;
/// Feature not implemented error
pub const NOT_IMPLEMENTED: u16 = 1006;
/// Memory access error
pub const MEMORY_ACCESS_ERROR: u16 = 1007;
/// Initialization error
pub const INITIALIZATION_ERROR: u16 = 1008;
/// Type mismatch error
pub const TYPE_MISMATCH: u16 = 1009;
/// Parse error
pub const PARSE_ERROR: u16 = 1010;
/// Invalid version error
pub const INVALID_VERSION: u16 = 1011;
/// Out of bounds error
pub const OUT_OF_BOUNDS_ERROR: u16 = 1012;
/// Execution instruction index out of bounds error
pub const EXECUTION_INSTRUCTION_INDEX_OUT_OF_BOUNDS: u16 = 1013;
/// Execution invalid frame error
pub const EXECUTION_INVALID_FRAME: u16 = 1014;
/// Execution reader not implemented error
pub const EXECUTION_READER_NOT_IMPLEMENTED: u16 = 1015;
/// Capacity exceeded
pub const CAPACITY_EXCEEDED: u16 = 1013;
/// Gas limit exceeded
pub const GAS_LIMIT_EXCEEDED: u16 = 1014;
/// Call stack exhausted
pub const CALL_STACK_EXHAUSTED: u16 = 1015;

// Component model error codes (2000-2999)
/// Invalid function index error
pub const INVALID_FUNCTION_INDEX: u16 = 2000;
/// Component type mismatch error
pub const COMPONENT_TYPE_MISMATCH: u16 = 2001;
/// Encoding error
pub const ENCODING_ERROR: u16 = 2002;
/// Execution limit exceeded error
pub const EXECUTION_LIMIT_EXCEEDED: u16 = 2003;
/// Component instantiation error
pub const COMPONENT_INSTANTIATION_ERROR: u16 = 2004;
/// Canonical ABI error
pub const CANONICAL_ABI_ERROR: u16 = 2005;
/// Component linking error
pub const COMPONENT_LINKING_ERROR: u16 = 2006;

// Resource error codes (3000-3999)
/// Resource error
pub const RESOURCE_ERROR: u16 = 3000;
/// Resource limit exceeded error
pub const RESOURCE_LIMIT_EXCEEDED: u16 = 3001;
/// Resource access error
pub const RESOURCE_ACCESS_ERROR: u16 = 3002;
/// Resource not found error
pub const RESOURCE_NOT_FOUND: u16 = 3003;
/// Resource invalid handle error
pub const RESOURCE_INVALID_HANDLE: u16 = 3004;
/// Global not found
pub const GLOBAL_NOT_FOUND: u16 = 3005;
/// Memory not found
pub const MEMORY_NOT_FOUND: u16 = 3006;
/// Table not found
pub const TABLE_NOT_FOUND: u16 = 3007;

// Memory error codes (4000-4999)
/// Memory out of bounds error
pub const MEMORY_OUT_OF_BOUNDS: u16 = 4000;
/// Memory grow error
pub const MEMORY_GROW_ERROR: u16 = 4001;
/// Memory access out of bounds error
pub const MEMORY_ACCESS_OUT_OF_BOUNDS: u16 = 4002;
/// Memory access unaligned error
pub const MEMORY_ACCESS_UNALIGNED: u16 = 4003;

// Validation error codes (5000-5999)
/// Validation error
pub const VALIDATION_ERROR: u16 = 5000;
/// Validation failure
pub const VALIDATION_FAILURE: u16 = 5001;
/// Checksum mismatch error
pub const CHECKSUM_MISMATCH: u16 = 5002;
/// Integrity violation error
pub const INTEGRITY_VIOLATION: u16 = 5003;
/// Verification level violation error
pub const VERIFICATION_LEVEL_VIOLATION: u16 = 5004;
/// Validation global type mismatch error
pub const VALIDATION_GLOBAL_TYPE_MISMATCH: u16 = 5005;
/// Validation invalid memory index error
pub const VALIDATION_INVALID_MEMORY_INDEX: u16 = 5006;
/// Validation invalid global index error
pub const VALIDATION_INVALID_GLOBAL_INDEX: u16 = 5007;
/// Validation unsupported feature error
pub const VALIDATION_UNSUPPORTED_FEATURE: u16 = 5008;
/// Validation invalid instruction error
pub const VALIDATION_INVALID_INSTRUCTION: u16 = 5009;
/// Validation empty stack error
pub const VALIDATION_EMPTY_STACK: u16 = 5010;
/// Validation stack size error
pub const VALIDATION_STACK_SIZE_ERROR: u16 = 5011;
/// Validation no binary error
pub const VALIDATION_NO_BINARY: u16 = 5012;
/// Validation function not found error
pub const VALIDATION_FUNCTION_NOT_FOUND: u16 = 5013;
/// Validation export not found error
pub const VALIDATION_EXPORT_NOT_FOUND: u16 = 5014;
/// Validation invalid function type error
pub const VALIDATION_INVALID_FUNCTION_TYPE: u16 = 5015;
/// Validation invalid table index error
pub const VALIDATION_INVALID_TABLE_INDEX: u16 = 5016;
/// Validation invalid element index error
pub const VALIDATION_INVALID_ELEMENT_INDEX: u16 = 5017;
/// Validation invalid data segment index error
pub const VALIDATION_INVALID_DATA_SEGMENT_INDEX: u16 = 5018;
/// Validation duplicate table reference error
pub const VALIDATION_DUPLICATE_TABLE_REFERENCE: u16 = 5019;
/// Validation invalid frame index error
pub const VALIDATION_INVALID_FRAME_INDEX: u16 = 5020;
/// Validation stack underflow error
pub const VALIDATION_STACK_UNDERFLOW: u16 = 5021;
/// Validation: min limit from u64 source exceeds u32 target
pub const VALIDATION_LIMIT_MIN_EXCEEDS_U32: u16 = 5022;
/// Validation: max limit from u64 source exceeds u32 target
pub const VALIDATION_LIMIT_MAX_EXCEEDS_U32: u16 = 5023;
/// Validation: max limit is less than min limit
pub const VALIDATION_LIMIT_MAX_LESS_THAN_MIN: u16 = 5024;
/// Validation: Invalid custom section name
pub const VALIDATION_INVALID_CUSTOM_SECTION_NAME: u16 = 5025;

// Type error codes (6000-6999)
/// Invalid type error
pub const INVALID_TYPE: u16 = 6000;
/// Type mismatch error
pub const TYPE_MISMATCH_ERROR: u16 = 6001;
/// Invalid function type error
pub const INVALID_FUNCTION_TYPE: u16 = 6002;
/// Invalid value type error
pub const INVALID_VALUE_TYPE: u16 = 6003;
/// Parse invalid function index type error
pub const PARSE_INVALID_FUNCTION_INDEX_TYPE: u16 = 6004;
/// Parse invalid table index type error
pub const PARSE_INVALID_TABLE_INDEX_TYPE: u16 = 6005;
/// Parse invalid memory index type error
pub const PARSE_INVALID_MEMORY_INDEX_TYPE: u16 = 6006;
/// Parse invalid global index type error
pub const PARSE_INVALID_GLOBAL_INDEX_TYPE: u16 = 6007;

// Runtime error codes (7000-7999)
/// Runtime error
pub const RUNTIME_ERROR: u16 = 7000;
/// Execution timeout error
pub const EXECUTION_TIMEOUT: u16 = 7001;
/// Fuel exhausted error
pub const FUEL_EXHAUSTED: u16 = 7002;
/// Poisoned lock error
pub const POISONED_LOCK: u16 = 7003;
/// Runtime memory integrity error
pub const RUNTIME_MEMORY_INTEGRITY_ERROR: u16 = 7004;
/// Runtime stack integrity error
pub const RUNTIME_STACK_INTEGRITY_ERROR: u16 = 7005;
/// Runtime label integrity error
pub const RUNTIME_LABEL_INTEGRITY_ERROR: u16 = 7006;
/// Runtime frame integrity error
pub const RUNTIME_FRAME_INTEGRITY_ERROR: u16 = 7007;

// System error codes (8000-8999)
/// System error
pub const SYSTEM_ERROR: u16 = 8000;
/// Unsupported operation error
pub const UNSUPPORTED_OPERATION: u16 = 8001;
/// Conversion error
pub const CONVERSION_ERROR: u16 = 8002;
/// Decoding error
pub const DECODING_ERROR: u16 = 8003;
/// Concurrency error
pub const CONCURRENCY_LOCK_FAILURE: u16 = 8004;
/// Initialization failure
pub const CONCURRENCY_INITIALIZATION_FAILURE: u16 = 8005;
/// Capacity limit exceeded
pub const CAPACITY_LIMIT_EXCEEDED: u16 = 8006;

/// Serialization error
pub const SERIALIZATION_ERROR: u16 = 8007;

// Unknown error code
/// Unknown error
pub const UNKNOWN: u16 = 9999;

// Parser error codes (8100-8199)
/// Parse invalid magic bytes error
pub const PARSE_INVALID_MAGIC_BYTES: u16 = 8101;
/// Parse invalid version bytes error
pub const PARSE_INVALID_VERSION_BYTES: u16 = 8102;
/// Parse invalid section ID error
pub const PARSE_INVALID_SECTION_ID: u16 = 8103;
/// Parse invalid local count error
pub const PARSE_INVALID_LOCAL_COUNT: u16 = 8108;
/// Parse invalid label count error
pub const PARSE_INVALID_LABEL_COUNT: u16 = 8109;
/// Parse invalid type definition error
pub const PARSE_INVALID_TYPE_DEF: u16 = 8110;
/// Parse invalid data definition error
pub const PARSE_INVALID_DATA_DEF: u16 = 8111;
/// Parse invalid element definition error
pub const PARSE_INVALID_ELEMENT_DEF: u16 = 8112;
/// Parse invalid value type byte error
pub const PARSE_INVALID_VALTYPE_BYTE: u16 = 8113;
/// Parse invalid opcode byte error
pub const PARSE_INVALID_OPCODE_BYTE: u16 = 8114;
/// Parse invalid LEB128 encoding error
pub const PARSE_INVALID_LEB128_ENCODING: u16 = 8115;
/// Parse unexpected EOF error
pub const PARSE_UNEXPECTED_EOF: u16 = 8116;
/// Parse malformed UTF-8 string error
pub const PARSE_MALFORMED_UTF8_STRING: u16 = 8117;
/// Parse invalid alignment value error
pub const PARSE_INVALID_ALIGNMENT_VALUE: u16 = 8118;
/// Parse invalid reference type byte error
pub const PARSE_INVALID_REFERENCE_TYPE_BYTE: u16 = 8119;

// Validation error codes (8200-8299)
/// Validation memory type mismatch error
pub const VALIDATION_MEMORY_TYPE_MISMATCH_ERROR: u16 = 8205;
/// Validation table type mismatch error
pub const VALIDATION_TABLE_TYPE_MISMATCH_ERROR: u16 = 8206;
/// Validation value type error
pub const VALIDATION_VALUE_TYPE_ERROR: u16 = 8207;
/// Validation stack overflow error
pub const VALIDATION_STACK_OVERFLOW_ERROR: u16 = 8209;
/// Validation type mismatch error
pub const VALIDATION_TYPE_MISMATCH_ERROR: u16 = 8210;
/// Validation control flow error
pub const VALIDATION_CONTROL_FLOW_ERROR: u16 = 8211;
/// Validation branch target error
pub const VALIDATION_BRANCH_TARGET_ERROR: u16 = 8212;
/// Validation unreachable code error
pub const VALIDATION_UNREACHABLE_CODE_ERROR: u16 = 8213;
/// Validation memory access error
pub const VALIDATION_MEMORY_ACCESS_ERROR: u16 = 8214;
/// Validation start function error
pub const VALIDATION_START_FUNCTION_ERROR: u16 = 8215;

// Memory errors (8400-8499)
/// Memory allocation error
pub const MEMORY_ALLOCATION_ERROR: u16 = 8403;
/// Memory grow failure error
pub const MEMORY_GROW_FAILURE: u16 = 8404;
/// Memory alignment error code
pub const MEMORY_ALIGNMENT_ERROR_CODE: u16 = 8405;
/// Memory size limit error
pub const MEMORY_SIZE_LIMIT_ERROR: u16 = 8406;

// Runtime trap errors (8600-8699)
/// Runtime trap error
pub const RUNTIME_TRAP_ERROR: u16 = 8601;
/// Runtime uninitialized element error
pub const RUNTIME_UNINITIALIZED_ELEMENT_ERROR: u16 = 8602;
/// Runtime unimplemented instruction error
pub const RUNTIME_UNIMPLEMENTED_INSTRUCTION_ERROR: u16 = 8603;
/// Runtime invalid conversion error
pub const RUNTIME_INVALID_CONVERSION_ERROR: u16 = 8604;
/// Runtime division by zero error
pub const RUNTIME_DIVISION_BY_ZERO_ERROR: u16 = 8605;
/// Runtime integer overflow error
pub const RUNTIME_INTEGER_OVERFLOW_ERROR: u16 = 8606;
/// Runtime function not found error
pub const RUNTIME_FUNCTION_NOT_FOUND_ERROR: u16 = 8607;
/// Runtime import not found error
pub const RUNTIME_IMPORT_NOT_FOUND_ERROR: u16 = 8608;
/// Runtime memory integrity violation error
pub const RUNTIME_MEMORY_INTEGRITY_VIOLATION: u16 = 8609;
/// Runtime call indirect type mismatch error
pub const RUNTIME_CALL_INDIRECT_TYPE_MISMATCH_ERROR: u16 = 8610;
/// Runtime invalid argument error
pub const RUNTIME_INVALID_ARGUMENT_ERROR: u16 = 8611;
/// Runtime export not found error
pub const RUNTIME_EXPORT_NOT_FOUND_ERROR: u16 = 8612;

// System errors (8800-8899)
/// System IO error code
pub const SYSTEM_IO_ERROR_CODE: u16 = 8801;
/// System resource limit error
pub const SYSTEM_RESOURCE_LIMIT_ERROR: u16 = 8802;
/// System unsupported feature error
pub const SYSTEM_UNSUPPORTED_FEATURE_ERROR: u16 = 8803;

// Component errors (9000-9099)
/// Component invalid type error
pub const COMPONENT_INVALID_TYPE_ERROR: u16 = 9001;
/// Component export not found error
pub const COMPONENT_EXPORT_NOT_FOUND_ERROR: u16 = 9002;
/// Component import not found error
pub const COMPONENT_IMPORT_NOT_FOUND_ERROR: u16 = 9003;
/// Component conversion error code
pub const COMPONENT_CONVERSION_ERROR_CODE: u16 = 9005;
/// Component invalid state error
pub const COMPONENT_INVALID_STATE_ERROR: u16 = 9007;
/// Component resource limit error
pub const COMPONENT_RESOURCE_LIMIT_ERROR: u16 = 9008;

/// Invalid value error
pub const INVALID_VALUE: u16 = 6010;

/// Out of bounds error
pub const OUT_OF_BOUNDS: u16 = 1020;

/// Mutex error
pub const MUTEX_ERROR: u16 = 7010;

/// Function not found error
pub const FUNCTION_NOT_FOUND: u16 = 2010;

/// Implementation limit reached error
pub const IMPLEMENTATION_LIMIT: u16 = 8010;

/// Invalid state error
pub const INVALID_STATE: u16 = 7020;

/// Value out of range error
pub const VALUE_OUT_OF_RANGE: u16 = 6015;

/// Codes representing WebAssembly runtime trap conditions.
/// These are used when an operation cannot complete normally due to a runtime error
/// defined by the WebAssembly specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)] // Optional: if we want to map them to specific numeric codes easily
pub enum TrapCode {
    /// An `unreachable` instruction was executed.
    Unreachable = 1,
    /// Call to an indirect function with an out-of-bounds index.
    IndirectCallIndexOutOfBounds = 2,
    /// Indirect call to a null table entry.
    IndirectCallNullTableEntry = 3,
    /// Indirect call signature mismatch.
    IndirectCallSignatureMismatch = 4,
    /// Integer division by zero.
    IntegerDivideByZero = 5,
    /// Integer overflow during conversion from a float, or float is NaN/Infinity.
    InvalidConversionToInteger = 6,
    /// Integer overflow for an operation that traps on overflow (e.g. `i32.div_s` specific case).
    IntegerOverflow = 7,
    /// Out-of-bounds memory access.
    MemoryOutOfBounds = 8,
    /// Attempt to grow memory beyond its limit.
    MemoryGrowOutOfBounds = 9, // Not strictly a trap, but a runtime error condition
    /// Uninitialized element in a table.
    UninitializedElement = 10,
    /// Out-of-bounds table access (e.g. `table.get`, `table.set`).
    TableOutOfBounds = 11,
    // Add more specific trap codes as needed based on Wasm spec.
    /// A generic trap for conditions not covered by more specific codes.
    GenericTrap = 12,
}

impl TrapCode {
    /// Provides a default message for the trap code.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Unreachable => "unreachable instruction executed",
            Self::IndirectCallIndexOutOfBounds => "indirect call index out of bounds",
            Self::IndirectCallNullTableEntry => "indirect call to null table entry",
            Self::IndirectCallSignatureMismatch => "indirect call signature mismatch",
            Self::IntegerDivideByZero => "integer divide by zero",
            Self::InvalidConversionToInteger => "invalid conversion to integer",
            Self::IntegerOverflow => "integer overflow",
            Self::MemoryOutOfBounds => "out of bounds memory access",
            Self::MemoryGrowOutOfBounds => {
                "failed to grow memory; limit reached or allocation failed"
            }
            Self::UninitializedElement => "uninitialized element",
            Self::TableOutOfBounds => "out of bounds table access",
            Self::GenericTrap => "a WebAssembly trap occurred",
        }
    }
}

// It might also be useful to have a way to convert TrapCode into a general Error
// This is a sketch and might need adjustment based on how Error is structured.
// Assuming Error::new takes an ErrorCategory, a code (we can use TrapCode as u16), and a message.
impl From<TrapCode> for crate::Error {
    fn from(trap_code: TrapCode) -> Self {
        Self::new(
            crate::ErrorCategory::RuntimeTrap,
            trap_code as u16,    // Use the discriminant value as the code
            trap_code.message(), // trap_code.message() returns &'static str which fulfills Into<String>
        )
    }
}
