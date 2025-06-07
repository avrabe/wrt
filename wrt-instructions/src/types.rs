//! Type aliases for no_std compatibility

use crate::prelude::*;
#[cfg(not(feature = "std"))]
use wrt_foundation::NoStdProvider;

// CFI-specific types
/// Maximum number of CFI targets
pub const MAX_CFI_TARGETS: usize = 16;
/// Maximum number of CFI requirements
pub const MAX_CFI_REQUIREMENTS: usize = 16;
/// Maximum number of CFI target types
pub const MAX_CFI_TARGET_TYPES: usize = 8;

/// CFI target vector type
#[cfg(feature = "std")]
pub type CfiTargetVec = Vec<u32>;

/// CFI target vector type (no_std)
#[cfg(not(feature = "std"))]
pub type CfiTargetVec = BoundedVec<u32, MAX_CFI_TARGETS, NoStdProvider<1024>>;

/// CFI requirement vector type
#[cfg(feature = "std")]
pub type CfiRequirementVec = Vec<crate::cfi_control_ops::CfiValidationRequirement>;

#[cfg(not(feature = "std"))]
pub type CfiRequirementVec = BoundedVec<crate::cfi_control_ops::CfiValidationRequirement, MAX_CFI_REQUIREMENTS, NoStdProvider<1024>>;

/// CFI target type vector
#[cfg(feature = "std")]
pub type CfiTargetTypeVec = Vec<crate::cfi_control_ops::CfiTargetType>;

/// CFI target type vector (no_std)
#[cfg(not(feature = "std"))]
pub type CfiTargetTypeVec = BoundedVec<crate::cfi_control_ops::CfiTargetType, MAX_CFI_TARGET_TYPES, NoStdProvider<1024>>;

// Additional CFI collection types
/// Maximum shadow stack size
pub const MAX_SHADOW_STACK: usize = 1024;
/// Maximum landing pad expectations
pub const MAX_LANDING_PAD_EXPECTATIONS: usize = 64;
/// Maximum CFI expected values
pub const MAX_CFI_EXPECTED_VALUES: usize = 16;

#[cfg(feature = "std")]
pub type ShadowStackVec = Vec<crate::cfi_control_ops::ShadowStackEntry>;

#[cfg(not(feature = "std"))]
pub type ShadowStackVec = BoundedVec<crate::cfi_control_ops::ShadowStackEntry, MAX_SHADOW_STACK, NoStdProvider<{ MAX_SHADOW_STACK * 64 }>>;

#[cfg(feature = "std")]
pub type LandingPadExpectationVec = Vec<crate::cfi_control_ops::LandingPadExpectation>;

#[cfg(not(feature = "std"))]
pub type LandingPadExpectationVec = BoundedVec<crate::cfi_control_ops::LandingPadExpectation, MAX_LANDING_PAD_EXPECTATIONS, NoStdProvider<{ MAX_LANDING_PAD_EXPECTATIONS * 64 }>>;

#[cfg(feature = "std")]
pub type CfiExpectedValueVec = Vec<crate::cfi_control_ops::CfiExpectedValue>;

#[cfg(not(feature = "std"))]
pub type CfiExpectedValueVec = BoundedVec<crate::cfi_control_ops::CfiExpectedValue, MAX_CFI_EXPECTED_VALUES, NoStdProvider<{ MAX_CFI_EXPECTED_VALUES * 32 }>>;

// Collection type aliases that work across all configurations
#[cfg(feature = "std")]
pub type InstructionVec<T> = Vec<T>;

#[cfg(not(feature = "std"))]
pub type InstructionVec<T> = BoundedVec<T, 256, NoStdProvider<{ 256 * 32 }>>;

// Stack type with reasonable size for WASM
pub const MAX_STACK_SIZE: usize = 1024;

#[cfg(feature = "std")]
pub type ValueStack = Vec<Value>;

#[cfg(not(feature = "std"))]
pub type ValueStack = BoundedStack<Value, MAX_STACK_SIZE, NoStdProvider<{ MAX_STACK_SIZE * 16 }>>;

// Table storage
pub const MAX_TABLES: usize = 16;
pub const MAX_TABLE_SIZE: usize = 65536;

#[cfg(feature = "std")]
pub type TableVec = Vec<Vec<RefValue>>;

#[cfg(not(feature = "std"))]
pub type TableVec = BoundedVec<BoundedVec<RefValue, MAX_TABLE_SIZE, NoStdProvider<{ MAX_TABLE_SIZE * 16 }>>, MAX_TABLES, NoStdProvider<{ MAX_TABLES * 256 }>>;

// Locals and globals storage
pub const MAX_LOCALS: usize = 1024;
pub const MAX_GLOBALS: usize = 1024;

#[cfg(feature = "std")]
pub type LocalsVec = Vec<Value>;

#[cfg(not(feature = "std"))]
pub type LocalsVec = BoundedVec<Value, MAX_LOCALS, NoStdProvider<{ MAX_LOCALS * 16 }>>;

#[cfg(feature = "std")]
pub type GlobalsVec = Vec<Value>;

#[cfg(not(feature = "std"))]
pub type GlobalsVec = BoundedVec<Value, MAX_GLOBALS, NoStdProvider<{ MAX_GLOBALS * 16 }>>;

// Reference value type (for tables)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefValue {
    /// Null reference (default)
    #[default]
    Null,
    /// Function reference
    FuncRef(u32),
    /// External reference  
    ExternRef(u32),
}

impl wrt_foundation::traits::Checksummable for RefValue {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        match self {
            Self::Null => checksum.update_slice(&[0u8]),
            Self::FuncRef(id) => {
                checksum.update_slice(&[1u8]);
                checksum.update_slice(&id.to_le_bytes());
            },
            Self::ExternRef(id) => {
                checksum.update_slice(&[2u8]);
                checksum.update_slice(&id.to_le_bytes());
            },
        }
    }
}

impl wrt_foundation::traits::ToBytes for RefValue {
    fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::Result<()> {
        match self {
            Self::Null => writer.write_u8(0u8),
            Self::FuncRef(id) => {
                writer.write_u8(1u8)?;
                writer.write_all(&id.to_le_bytes())
            },
            Self::ExternRef(id) => {
                writer.write_u8(2u8)?;
                writer.write_all(&id.to_le_bytes())
            },
        }
    }
}

impl wrt_foundation::traits::FromBytes for RefValue {
    fn from_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream,
        _provider: &PStream,
    ) -> wrt_foundation::Result<Self> {
        let discriminant = reader.read_u8()?;
        match discriminant {
            0 => Ok(Self::Null),
            1 => {
                let mut id_bytes = [0u8; 4];
                reader.read_exact(&mut id_bytes)?;
                let id = u32::from_le_bytes(id_bytes);
                Ok(Self::FuncRef(id))
            },
            2 => {
                let mut id_bytes = [0u8; 4];
                reader.read_exact(&mut id_bytes)?;
                let id = u32::from_le_bytes(id_bytes);
                Ok(Self::ExternRef(id))
            },
            _ => Err(wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Validation,
                wrt_foundation::codes::VALIDATION_ERROR,
                "Invalid discriminant for RefValue",
            )),
        }
    }
}

// Helper to create vectors in both modes
#[cfg(feature = "std")]
#[macro_export]
macro_rules! make_vec {
    () => { Vec::new() };
    ($($elem:expr),*) => { vec![$($elem),*] };
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! make_vec {
    () => { BoundedVec::new(NoStdProvider::default()).unwrap() };
    ($($elem:expr),*) => {{
        let mut v = BoundedVec::new(NoStdProvider::default()).unwrap();
        $(v.push($elem).unwrap();)*
        v
    }};
}