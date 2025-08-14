use std::boxed::Box;

pub use wrt_error::{
    Error,
    ErrorCategory,
};
pub use wrt_foundation::{
    resource::ResourceRepresentation,
    Result,
};

// Comment: BlockType, FuncType, RefType, ValueType now require MemoryProvider
// parameters
