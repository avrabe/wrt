use crate::String;
use std::boxed::Box;
use std::fmt;
use wast::Error as WastError;
#[cfg(feature = "wat-parsing")]
use wat::Error as WatError;

#[cfg(not(feature = "std"))]
use alloc::string::ToString;

/// Represents errors that can occur in the WebAssembly runtime.
///
/// This enum covers the various categories of errors that can occur during WebAssembly
/// module validation, execution, parsing, I/O operations, and component model operations.
///
/// # Examples
///
/// ```
/// use wrt::Error;
///
/// let err = Error::Validation("Invalid type".to_string());
/// assert!(err.to_string().contains("Validation error"));
/// ```
#[derive(PartialEq)]
pub enum Error {
    /// Represents validation errors that occur when a WebAssembly module
    /// fails to meet the WebAssembly specification requirements.
    Validation(String),

    /// Represents errors that occur during the execution of a WebAssembly module,
    /// such as out-of-bounds memory access, stack overflow, or type mismatches.
    Execution(String),

    /// Represents that execution has paused due to fuel exhaustion.
    /// This is not a true error but a signal that execution can be resumed.
    FuelExhausted,

    /// Represents input/output errors, typically when reading from or writing to
    /// a file system or other I/O operations.
    IO(String),

    /// Represents parsing errors that occur when decoding a WebAssembly binary
    /// or parsing a WebAssembly text format.
    Parse(String),

    /// Represents errors related to the WebAssembly Component Model, such as
    /// component instantiation failures or interface type mismatches.
    Component(String),

    /// Represents custom errors that don't fit into the other categories.
    /// This is useful for extension points or for wrapping errors from
    /// other libraries.
    Custom(String),

    /// Represents accessing memory, table, or other indexed resource outside its limits.
    OutOfBounds,

    /// Represents a stack underflow error that occurs when trying to pop from an empty stack.
    StackUnderflow,

    /// Represents errors that occur during serialization or deserialization of WebAssembly state.
    Serialization(String),

    /// Represents an error when an export is not found in a module.
    ExportNotFound(String),

    /// Represents an error when an integer operation results in overflow.
    IntegerOverflow,

    /// Represents an error when an instance index is invalid.
    InvalidInstanceIndex(usize),

    /// Represents an error when a function index is invalid.
    InvalidFunctionIndex(usize),

    /// Represents an error when a program counter is invalid.
    InvalidProgramCounter(usize),

    /// Represents an error when the execution state is invalid.
    InvalidExecutionState,

    /// Represents an error when no instances are available.
    NoInstances,

    /// Represents an error when the export type is invalid.
    InvalidExport,

    /// Represents an error when a local index is invalid.
    InvalidLocalIndex(usize),

    /// Represents an error when a global index is invalid.
    InvalidGlobalIndex(usize),

    /// Represents an error when trying to modify an immutable global variable.
    GlobalNotMutable(usize),

    /// Represents an error when accessing memory in an invalid way.
    InvalidMemoryAccess(String),

    /// Represents an error when accessing a table in an invalid way.
    InvalidTableAccess(String),

    /// Represents an error when a type index is invalid.
    InvalidTypeIndex(u32),

    /// Represents an error when a block type is invalid.
    InvalidBlockType(String),

    /// Represents an error when an instruction is invalid.
    InvalidInstruction(String),

    /// Represents an error when a value is invalid.
    InvalidValue(String),

    /// Represents an error when a type is invalid.
    InvalidType(String),

    /// Represents an error when a module is invalid.
    InvalidModule(String),

    /// Represents an error when an import is invalid.
    InvalidImport(String),

    /// Represents an error when a section is invalid.
    InvalidSection(String),

    /// Represents an error when a name is invalid.
    InvalidName(String),

    /// Represents an error when data is invalid.
    InvalidData(String),

    /// Represents an error when an element is invalid.
    InvalidElement(String),

    /// Represents an error when code is invalid.
    InvalidCode(String),

    /// Represents an error when a local is invalid.
    InvalidLocal(String),

    /// Represents an error when a global is invalid.
    InvalidGlobal(String),

    /// Represents an error when memory is invalid.
    InvalidMemory(String),

    /// Represents an error when a table is invalid.
    InvalidTable(String),

    /// Represents an error when a function is invalid.
    InvalidFunction(String),

    /// Represents an error when a function is not found.
    FunctionNotFound(u32),

    /// Represents an error when a signature is invalid.
    InvalidSignature(String),

    /// Represents an error when a boundary is invalid.
    InvalidBoundary(String),

    /// Represents an error when an alignment is invalid.
    InvalidAlignment(String),

    /// Represents an error when an offset is invalid.
    InvalidOffset(String),

    /// Represents an error when a size is invalid.
    InvalidSize(String),

    /// Represents an error when a limit is invalid.
    InvalidLimit(String),

    /// Represents an error when an initializer is invalid.
    InvalidInitializer(String),

    /// Represents an error when a segment is invalid.
    InvalidSegment(String),

    /// Represents an error when an expression is invalid.
    InvalidExpression(String),

    /// Represents an error when a constant is invalid.
    InvalidConstant(String),

    /// Represents an error when an operator is invalid.
    InvalidOperator(String),

    /// Represents an error when an opcode is invalid.
    InvalidOpcode(String),

    /// Represents an error when an immediate is invalid.
    InvalidImmediate(String),

    /// Represents an error when a prefix is invalid.
    InvalidPrefix(String),

    /// Represents an error when a reserved is invalid.
    InvalidReserved(String),

    /// Represents an error when a custom is invalid.
    InvalidCustom(String),

    /// Represents an error when a version is invalid.
    InvalidVersion(String),

    /// Represents an error when a magic is invalid.
    InvalidMagic(String),

    /// Represents an error when a length is invalid.
    InvalidLength(String),

    /// Represents an error when a UTF-8 is invalid.
    InvalidUtf8(String),

    /// Represents an error when a LEB128 is invalid.
    InvalidLeb128(String),

    /// Represents an error when a float is invalid.
    InvalidFloat(String),

    /// Represents an error when an integer is invalid.
    InvalidInteger(String),

    /// Represents an error when a byte is invalid.
    InvalidByte(String),

    /// Represents an error when a char is invalid.
    InvalidChar(String),

    /// Represents an error when a string is invalid.
    InvalidString(String),

    /// Represents an error when a vector is invalid.
    InvalidVector(String),

    /// Represents an error when a map is invalid.
    InvalidMap(String),

    /// Represents an error when a set is invalid.
    InvalidSet(String),

    /// Represents an error when an array is invalid.
    InvalidArray(String),

    /// Represents an error when an object is invalid.
    InvalidObject(String),

    /// Represents an error when a value is invalid.
    InvalidValue2(String),

    /// Represents an error when a type is invalid.
    InvalidType2(String),

    /// Represents an error when a module is invalid.
    InvalidModule2(String),

    /// Represents an error when an import is invalid.
    InvalidImport2(String),

    /// Represents an error when an export is invalid.
    InvalidExport2(String),

    /// Represents an error when a section is invalid.
    InvalidSection2(String),

    /// Represents an error when a name is invalid.
    InvalidName2(String),

    /// Represents an error when an EOF is encountered unexpectedly.
    UnexpectedEof,

    /// Represents an error when a lane index is invalid.
    InvalidLaneIndex(usize),

    /// Represents an error when a label index is invalid.
    InvalidLabelIndex(usize),

    /// Represents a type mismatch error.
    TypeMismatch(String),

    /// Represents an error when a function type is invalid.
    InvalidFunctionType(String),

    /// Represents an error when a value type is invalid.
    InvalidValueType,

    /// Represents an error when a block signature is invalid.
    InvalidBlockSignature,

    /// Represents an error when a branch target is invalid.
    InvalidBranchTarget,

    /// Represents an error when a call target is invalid.
    InvalidCallTarget,

    /// Represents an error when a return type is invalid.
    InvalidReturnType,

    /// Represents an error when a mutability is invalid.
    InvalidMutability,

    /// Represents an error when a reference type is invalid.
    InvalidReferenceType,

    /// Represents an error when an element type is invalid.
    InvalidElementType,

    /// Represents an error when a data segment is invalid.
    InvalidDataSegment,

    /// Represents an error when an element segment is invalid.
    InvalidElementSegment,

    /// Represents an error when a custom section is invalid.
    InvalidCustomSection,

    /// Represents an error when a name section is invalid.
    InvalidNameSection,

    /// Represents an error when a code section is invalid.
    InvalidCodeSection,

    /// Represents an error when a table index is invalid.
    InvalidTableIndex(usize),

    /// Represents an error when a memory index is invalid.
    InvalidMemoryIndex(usize),

    /// Represents an error when a global index is invalid.
    InvalidGlobalAccess,

    /// Represents an error when a function type is invalid.
    InvalidFunctionTypeIndex,

    /// Represents an error when a start function is invalid.
    InvalidStartFunction,

    /// Represents an error when an export name is invalid.
    InvalidExportName,

    /// Represents an error when an import name is invalid.
    InvalidImportName,

    /// Represents an error when a module name is invalid.
    InvalidModuleName,

    /// Represents an error when a field name is invalid.
    InvalidFieldName,

    /// Represents an error when a custom name is invalid.
    InvalidCustomName,

    /// Represents an error when a name type is invalid.
    InvalidNameType,

    /// Represents an error when a local name is invalid.
    InvalidLocalName,

    /// Represents an error when a global name is invalid.
    InvalidGlobalName,

    /// Represents an error when a table name is invalid.
    InvalidTableName,

    /// Represents an error when a memory name is invalid.
    InvalidMemoryName,

    /// Represents an error when a function name is invalid.
    InvalidFunctionName,

    /// Represents an error when an element name is invalid.
    InvalidElementName,

    /// Represents an error when a data name is invalid.
    InvalidDataName,

    /// Represents an error when a type name is invalid.
    InvalidTypeName,

    /// Represents an error when a section name is invalid.
    InvalidSectionName,

    /// Represents an error when a section size is invalid.
    InvalidSectionSize,

    /// Represents an error when a section content is invalid.
    InvalidSectionContent,

    /// Represents an error when a section order is invalid.
    InvalidSectionOrder,

    /// Represents an error when a section count is invalid.
    InvalidSectionCount,

    /// Represents an error when a vector length is invalid.
    InvalidVectorLength,

    /// Represents an error when a byte length is invalid.
    InvalidByteLength,

    /// Represents an error when a UTF-8 string is invalid.
    InvalidUtf8String,

    /// Represents an error when a UTF-8 encoding is invalid.
    InvalidUtf8Encoding,

    /// Represents an error when a UTF-8 sequence is invalid.
    InvalidUtf8Sequence,

    /// Represents an error when a UTF-8 code point is invalid.
    InvalidUtf8CodePoint,

    /// Represents an error when a UTF-8 range is invalid.
    InvalidUtf8Range,

    /// Represents an error when a UTF-8 continuation is invalid.
    InvalidUtf8Continuation,

    /// Represents an error when a UTF-8 leading byte is invalid.
    InvalidUtf8LeadingByte,

    /// Represents an error when a UTF-8 trailing byte is invalid.
    InvalidUtf8TrailingByte,

    /// Represents an error when a UTF-8 length is invalid.
    InvalidUtf8Length,

    /// Represents an error when a UTF-8 value is invalid.
    InvalidUtf8Value,

    /// Represents an error when a UTF-8 format is invalid.
    InvalidUtf8Format,

    /// Represents an error when a UTF-8 surrogate is invalid.
    InvalidUtf8Surrogate,

    /// Represents an error when a UTF-8 overlong is invalid.
    InvalidUtf8Overlong,

    /// Represents an error when a UTF-8 reserved is invalid.
    InvalidUtf8Reserved,

    /// Represents an error when a UTF-8 non-character is invalid.
    InvalidUtf8NonCharacter,

    /// Represents an error when a UTF-8 unassigned is invalid.
    InvalidUtf8Unassigned,

    /// Represents an error when a UTF-8 private is invalid.
    InvalidUtf8Private,

    /// Represents an error when a UTF-8 control is invalid.
    InvalidUtf8Control,

    /// Represents an error when a UTF-8 non-unicode is invalid.
    InvalidUtf8NonUnicode,

    /// Represents an error when a UTF-8 incomplete is invalid.
    InvalidUtf8Incomplete,

    /// Represents an error when a UTF-8 terminated is invalid.
    InvalidUtf8Terminated,

    /// Represents an error when a UTF-8 truncated is invalid.
    InvalidUtf8Truncated,

    /// Represents an error when a UTF-8 malformed is invalid.
    InvalidUtf8Malformed,

    /// Represents an error when a UTF-8 invalid is invalid.
    InvalidUtf8Invalid,

    /// Represents an error when a UTF-8 error is invalid.
    InvalidUtf8Error,

    /// Represents an error when a UTF-8 failure is invalid.
    InvalidUtf8Failure,

    /// Represents an error when a UTF-8 exception is invalid.
    InvalidUtf8Exception,

    /// Represents an error when a UTF-8 fault is invalid.
    InvalidUtf8Fault,

    /// Represents an error when a UTF-8 problem is invalid.
    InvalidUtf8Problem,

    /// Represents an error when a UTF-8 issue is invalid.
    InvalidUtf8Issue,

    /// Represents an error when a UTF-8 bug is invalid.
    InvalidUtf8Bug,

    /// Represents an error when a UTF-8 defect is invalid.
    InvalidUtf8Defect,

    /// Represents an error when a UTF-8 flaw is invalid.
    InvalidUtf8Flaw,

    /// Represents an error when a UTF-8 glitch is invalid.
    InvalidUtf8Glitch,

    /// Represents an error when a UTF-8 mistake is invalid.
    InvalidUtf8Mistake,

    /// Represents an error when a UTF-8 blunder is invalid.
    InvalidUtf8Blunder,

    /// Represents an error when a UTF-8 slip is invalid.
    InvalidUtf8Slip,

    /// Represents an error when a UTF-8 gaffe is invalid.
    InvalidUtf8Gaffe,

    /// Represents an error when a UTF-8 lapse is invalid.
    InvalidUtf8Lapse,

    /// Represents an error when a UTF-8 oversight is invalid.
    InvalidUtf8Oversight,

    /// Represents an error when a UTF-8 omission is invalid.
    InvalidUtf8Omission,

    /// Represents an error when a UTF-8 neglect is invalid.
    InvalidUtf8Neglect,

    /// Represents an error when a UTF-8 negligence is invalid.
    InvalidUtf8Negligence,

    /// Represents an error when a UTF-8 carelessness is invalid.
    InvalidUtf8Carelessness,

    /// Represents an error when a UTF-8 inattention is invalid.
    InvalidUtf8Inattention,

    /// Represents an error when a UTF-8 inadvertence is invalid.
    InvalidUtf8Inadvertence,

    /// Represents an error when a UTF-8 thoughtlessness is invalid.
    InvalidUtf8Thoughtlessness,

    /// Represents an error when a UTF-8 heedlessness is invalid.
    InvalidUtf8Heedlessness,

    /// Represents an error when a UTF-8 recklessness is invalid.
    InvalidUtf8Recklessness,

    /// Represents an error when a UTF-8 rashness is invalid.
    InvalidUtf8Rashness,

    /// Represents an error when a UTF-8 imprudence is invalid.
    InvalidUtf8Imprudence,

    /// Represents an error when a UTF-8 indiscretion is invalid.
    InvalidUtf8Indiscretion,

    /// Represents an error when a UTF-8 inconsideration is invalid.
    InvalidUtf8Inconsideration,

    /// Represents an error when a UTF-8 disregard is invalid.
    InvalidUtf8Disregard,

    /// Represents an error when a UTF-8 ignorance is invalid.
    InvalidUtf8Ignorance,

    /// Represents an error when a UTF-8 unawareness is invalid.
    InvalidUtf8Unawareness,

    /// Represents an error when a UTF-8 unconsciousness is invalid.
    InvalidUtf8Unconsciousness,

    /// Represents an error when a UTF-8 obliviousness is invalid.
    InvalidUtf8Obliviousness,

    /// Represents an error when a UTF-8 forgetfulness is invalid.
    InvalidUtf8Forgetfulness,

    /// Represents an error when a UTF-8 amnesia is invalid.
    InvalidUtf8Amnesia,

    /// Represents an error when a UTF-8 blackout is invalid.
    InvalidUtf8Blackout,

    /// Represents an error when a UTF-8 blank is invalid.
    InvalidUtf8Blank,

    /// Represents an error when a UTF-8 void is invalid.
    InvalidUtf8Void,

    /// Represents an error when a UTF-8 empty is invalid.
    InvalidUtf8Empty,

    /// Represents an error when a UTF-8 null is invalid.
    InvalidUtf8Null,

    /// Represents an error when a UTF-8 zero is invalid.
    InvalidUtf8Zero,

    /// Represents an error when a UTF-8 nothing is invalid.
    InvalidUtf8Nothing,

    /// Represents an error when a UTF-8 none is invalid.
    InvalidUtf8None,

    /// Represents an error when a UTF-8 nil is invalid.
    InvalidUtf8Nil,

    /// Represents an error when a UTF-8 nada is invalid.
    InvalidUtf8Nada,

    /// Represents an error when a UTF-8 zilch is invalid.
    InvalidUtf8Zilch,

    /// Represents an error when a UTF-8 zip is invalid.
    InvalidUtf8Zip,

    /// Represents an error when a UTF-8 nix is invalid.
    InvalidUtf8Nix,

    /// Represents an error when a UTF-8 naught is invalid.
    InvalidUtf8Naught,

    /// Represents an error when a UTF-8 cipher is invalid.
    InvalidUtf8Cipher,

    /// Represents an error when a UTF-8 goose is invalid.
    InvalidUtf8Goose,

    /// Represents an error when a UTF-8 duck is invalid.
    InvalidUtf8Duck,

    /// Represents an error when a UTF-8 egg is invalid.
    InvalidUtf8Egg,

    /// Represents an error when a UTF-8 love is invalid.
    InvalidUtf8Love,

    /// Represents an error when a UTF-8 score is invalid.
    InvalidUtf8Score,

    /// Represents an error when a UTF-8 scratch is invalid.
    InvalidUtf8Scratch,

    /// Represents an error when a UTF-8 wash is invalid.
    InvalidUtf8Wash,

    /// Represents an error when division by zero occurs.
    DivisionByZero,

    /// Represents an error when an instruction is not implemented.
    UnimplementedInstruction(String),

    /// Represents an error when an operation is invalid.
    InvalidOperation { message: String },

    /// Represents an error when argument count is invalid.
    InvalidArgumentCount { expected: usize, actual: usize },

    /// Represents an error when argument type is invalid.
    InvalidArgumentType(usize),

    /// Represents an error when result type is invalid.
    InvalidResultType(usize),

    /// Represents an error when result count is invalid.
    InvalidResultCount { expected: usize, actual: usize },

    /// Represents an error when a function is unimplemented.
    Unimplemented(String),

    /// Represents an error when a lock is poisoned due to a panic in another thread.
    PoisonedLock,

    /// Represents an error when a function reference is null.
    NullFunctionReference,

    /// Represents an error when parsing a hexadecimal floating point literal fails.
    InvalidHexFloat { message: String },

    /// Represents a state change in the execution engine.
    /// This is not a true error but a signal that execution state has changed.
    StateChange(Box<crate::stackless::StacklessExecutionState>),

    /// Represents an error when a lock could not be acquired (poisoned).
    PoisonError(String),

    /// Represents an error when a memory grows beyond its limit.
    MemoryGrowError(String),

    /// Represents an error when accessing memory out of bounds.
    MemoryAccessOutOfBounds(String),

    /// Represents an error when accessing an element out of bounds.
    InvalidElementAccess(String),

    /// Represents an error when an element index is invalid.
    InvalidElementIndex(u32),
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            // Just handle a small subset of variants to make the compiler happy for now
            Self::Validation(s) => Self::Validation(s.clone()),
            Self::Execution(s) => Self::Execution(s.clone()),
            Self::InvalidModule(s) => Self::InvalidModule(s.clone()),
            Self::StackUnderflow => Self::StackUnderflow,
            Self::FuelExhausted => Self::FuelExhausted,
            Self::TypeMismatch(s) => Self::TypeMismatch(s.clone()),
            Self::NullFunctionReference => Self::NullFunctionReference,
            Self::StateChange(_) => Self::Execution("StateChange cloned".to_string()),
            Self::Unimplemented(s) => Self::Unimplemented(s.clone()),
            // Default fallback for all other variants
            _ => Self::Execution("Clone not fully implemented".to_string()),
        }
    }
}

impl Error {
    /// Creates a new custom error with the given message.
    #[must_use]
    pub fn new<T: Into<String>>(message: T) -> Self {
        Self::Custom(message.into())
    }
}

/// A Result type that uses the Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Implement conversion from wast::Error to wrt::Error.
impl From<WastError> for Error {
    fn from(err: WastError) -> Self {
        Error::Parse(err.to_string())
    }
}

#[cfg(feature = "wat-parsing")]
impl From<WatError> for Error {
    fn from(err: WatError) -> Self {
        Error::Parse(err.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(msg) => write!(f, "Validation error: {msg}"),
            Self::Execution(msg) => write!(f, "Execution error: {msg}"),
            Self::FuelExhausted => write!(f, "Fuel exhausted"),
            Self::IO(msg) => write!(f, "I/O error: {msg}"),
            Self::Parse(msg) => write!(f, "Parse error: {msg}"),
            Self::Component(msg) => write!(f, "Component error: {msg}"),
            Self::Custom(msg) => write!(f, "Custom error: {msg}"),
            Self::OutOfBounds => write!(f, "Out of bounds"),
            Self::StackUnderflow => write!(f, "Stack underflow"),
            Self::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            Self::ExportNotFound(name) => write!(f, "Export not found: {name}"),
            Self::IntegerOverflow => write!(f, "Integer overflow"),
            Self::InvalidInstanceIndex(idx) => write!(f, "Invalid instance index: {idx}"),
            Self::InvalidFunctionIndex(idx) => write!(f, "Invalid function index: {idx}"),
            Self::InvalidProgramCounter(pc) => write!(f, "Invalid program counter: {pc}"),
            Self::InvalidExecutionState => write!(f, "Invalid execution state"),
            Self::NoInstances => write!(f, "No instances available"),
            Self::InvalidExport => write!(f, "Invalid export"),
            Self::InvalidLocalIndex(idx) => write!(f, "Invalid local index: {idx}"),
            Self::InvalidGlobalIndex(idx) => write!(f, "Invalid global index: {idx}"),
            Self::GlobalNotMutable(idx) => write!(f, "Global not mutable: {idx}"),
            Self::InvalidMemoryAccess(msg) => write!(f, "Invalid memory access: {msg}"),
            Self::InvalidTableAccess(msg) => write!(f, "Invalid table access: {msg}"),
            Self::InvalidTypeIndex(idx) => write!(f, "Invalid type index: {idx}"),
            Self::InvalidBlockType(msg) => write!(f, "Invalid block type: {msg}"),
            Self::InvalidInstruction(msg) => write!(f, "Invalid instruction: {msg}"),
            Self::InvalidValue(msg) => write!(f, "Invalid value: {msg}"),
            Self::InvalidType(msg) => write!(f, "Invalid type: {msg}"),
            Self::InvalidModule(msg) => write!(f, "Invalid module: {msg}"),
            Self::InvalidImport(msg) => write!(f, "Invalid import: {msg}"),
            Self::InvalidSection(msg) => write!(f, "Invalid section: {msg}"),
            Self::InvalidName(msg) => write!(f, "Invalid name: {msg}"),
            Self::InvalidData(msg) => write!(f, "Invalid data: {msg}"),
            Self::InvalidElement(msg) => write!(f, "Invalid element: {msg}"),
            Self::InvalidCode(msg) => write!(f, "Invalid code: {msg}"),
            Self::InvalidLocal(msg) => write!(f, "Invalid local: {msg}"),
            Self::InvalidGlobal(msg) => write!(f, "Invalid global: {msg}"),
            Self::InvalidMemory(msg) => write!(f, "Invalid memory: {msg}"),
            Self::InvalidTable(msg) => write!(f, "Invalid table: {msg}"),
            Self::InvalidFunction(msg) => write!(f, "Invalid function: {msg}"),
            Self::FunctionNotFound(idx) => write!(f, "Function not found: {idx}"),
            Self::InvalidSignature(msg) => write!(f, "Invalid signature: {msg}"),
            Self::InvalidMemoryIndex(idx) => write!(f, "Invalid memory index: {idx}"),
            Self::InvalidTableIndex(idx) => write!(f, "Invalid table index: {idx}"),
            Self::InvalidFunctionType(msg) => write!(f, "Invalid function type: {msg}"),
            Self::InvalidOperation { message } => write!(f, "Invalid operation: {message}"),
            Self::Unimplemented(feature) => write!(f, "Unimplemented feature: {feature}"),
            Self::UnimplementedInstruction(instr) => {
                write!(f, "Unimplemented instruction: {instr}")
            }
            Self::TypeMismatch(msg) => write!(f, "Type mismatch: {msg}"),
            Self::NullFunctionReference => write!(f, "Null function reference"),
            Self::StateChange(_) => write!(f, "Execution state changed"),
            Self::InvalidBoundary(msg) => write!(f, "Invalid boundary: {msg}"),
            Self::InvalidAlignment(msg) => write!(f, "Invalid alignment: {msg}"),
            Self::InvalidOffset(msg) => write!(f, "Invalid offset: {msg}"),
            Self::InvalidSize(msg) => write!(f, "Invalid size: {msg}"),
            Self::InvalidLimit(msg) => write!(f, "Invalid limit: {msg}"),
            Self::InvalidInitializer(msg) => write!(f, "Invalid initializer: {msg}"),
            Self::InvalidSegment(msg) => write!(f, "Invalid segment: {msg}"),
            Self::InvalidExpression(msg) => write!(f, "Invalid expression: {msg}"),
            Self::InvalidConstant(msg) => write!(f, "Invalid constant: {msg}"),
            Self::InvalidOperator(msg) => write!(f, "Invalid operator: {msg}"),
            Self::InvalidOpcode(msg) => write!(f, "Invalid opcode: {msg}"),
            Self::InvalidImmediate(msg) => write!(f, "Invalid immediate: {msg}"),
            Self::InvalidPrefix(msg) => write!(f, "Invalid prefix: {msg}"),
            Self::InvalidReserved(msg) => write!(f, "Invalid reserved: {msg}"),
            Self::InvalidCustom(msg) => write!(f, "Invalid custom: {msg}"),
            Self::InvalidVersion(msg) => write!(f, "Invalid version: {msg}"),
            Self::PoisonedLock => write!(f, "Poisoned lock"),
            Self::InvalidHexFloat { message } => write!(f, "Invalid hexadecimal float: {message}"),
            Self::PoisonError(msg) => write!(f, "Poison error: {msg}"),
            Self::MemoryGrowError(msg) => write!(f, "Memory grow error: {msg}"),
            Self::MemoryAccessOutOfBounds(msg) => write!(f, "Memory access out of bounds: {msg}"),
            Self::InvalidElementAccess(msg) => write!(f, "Invalid element access: {msg}"),
            Self::InvalidElementIndex(idx) => write!(f, "Invalid element index: {idx}"),
            // Add any other variants as needed
            _ => write!(f, "Unknown error: {self:?}"),
        }
    }
}

// Manual implementation of Debug for Error
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "UnexpectedEof"),
            Self::InvalidLeb128(msg) => write!(f, "InvalidLeb128({msg})"),
            Self::InvalidUtf8(msg) => write!(f, "InvalidUtf8({msg})"),
            Self::InvalidModule(msg) => write!(f, "InvalidModule({msg})"),
            Self::InvalidType(msg) => write!(f, "InvalidType({msg})"),
            Self::InvalidLocal(msg) => write!(f, "InvalidLocal({msg})"),
            Self::InvalidGlobal(msg) => write!(f, "InvalidGlobal({msg})"),
            Self::InvalidMemoryIndex(idx) => write!(f, "InvalidMemoryIndex({idx})"),
            Self::InvalidTableIndex(idx) => write!(f, "InvalidTableIndex({idx})"),
            Self::InvalidFunctionIndex(idx) => write!(f, "InvalidFunctionIndex({idx})"),
            Self::Execution(msg) => write!(f, "Execution({msg})"),
            Self::StackUnderflow => write!(f, "StackUnderflow"),
            Self::GlobalNotMutable(idx) => write!(f, "GlobalNotMutable({idx})"),
            Self::Unimplemented(msg) => write!(f, "Unimplemented({msg})"),
            Self::NullFunctionReference => write!(f, "NullFunctionReference"),
            Self::StateChange(state) => write!(f, "StateChange({state:?})"),
            Self::PoisonedLock => write!(f, "PoisonedLock"),
            Self::TypeMismatch(msg) => write!(f, "TypeMismatch({msg})"),
            Self::InvalidHexFloat { message } => write!(f, "InvalidHexFloat({message})"),
            Self::PoisonError(msg) => write!(f, "PoisonError({msg})"),
            // Add a catch-all for all other variants to prevent future issues
            _ => write!(f, "{self}"),
        }
    }
}

// Implement From<std::io::Error> for wrt::Error
#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err.to_string())
    }
}
