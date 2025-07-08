// Re-export ResourceOperation for convenience
pub use wrt_foundation::resource::ResourceOperation;

use crate::prelude::*;

/// Convert from local ResourceOperation enum to format ResourceOperation
pub fn to_format_resource_operation(
    op: ResourceOperation,
    type_idx: u32,
) -> wrt_format::component::FormatResourceOperation {
    use wrt_format::component::FormatResourceOperation as FormatOp;
    use wrt_foundation::resource::{ResourceDrop, ResourceNew, ResourceRep};

    match op {
        ResourceOperation::Read => FormatOp::Rep(ResourceRep { type_idx }),
        ResourceOperation::Write => FormatOp::Rep(ResourceRep { type_idx }), // Map to Rep
        ResourceOperation::Execute => FormatOp::Rep(ResourceRep { type_idx }), // Map to Rep
        ResourceOperation::Create => FormatOp::New(ResourceNew { type_idx }),
        ResourceOperation::Delete => FormatOp::Drop(ResourceDrop { type_idx }),
        ResourceOperation::Reference => FormatOp::Rep(ResourceRep { type_idx }), // Map to Rep
        ResourceOperation::Dereference => FormatOp::Rep(ResourceRep { type_idx }), // Map to Rep
    }
}

/// Convert from format ResourceOperation to local ResourceOperation
pub fn from_format_resource_operation(
    op: &wrt_format::component::FormatResourceOperation,
) -> ResourceOperation {
    use wrt_format::component::FormatResourceOperation as FormatOp;

    match op {
        FormatOp::Rep(_) => ResourceOperation::Read,
        FormatOp::New(_) => ResourceOperation::Create,
        FormatOp::Drop(_) => ResourceOperation::Delete,
    }
}

/// Convert a Core ResourceOperation to a Format ResourceOperation
#[cfg(not(feature = "safe-memoryMissing messageMissing messageMissing message"))]
pub fn core_to_format_resource_operation(
    op: &wrt_foundation::ResourceOperation,
) -> ResourceOperation {
    match op {
        wrt_foundation::ResourceOperation::New => ResourceOperation::New,
        wrt_foundation::ResourceOperation::Drop => ResourceOperation::Drop,
        wrt_foundation::ResourceOperation::Rep => ResourceOperation::Rep,
    }
}

/// Convert a Format ResourceOperation to a Core ResourceOperation
#[cfg(not(feature = "safe-memoryMissing messageMissing messageMissing message"))]
pub fn format_to_core_resource_operation(
    op: &ResourceOperation,
) -> wrt_foundation::ResourceOperation {
    match op {
        ResourceOperation::New => wrt_foundation::ResourceOperation::New,
        ResourceOperation::Drop => wrt_foundation::ResourceOperation::Drop,
        ResourceOperation::Rep => wrt_foundation::ResourceOperation::Rep,
    }
}

#[cfg(feature = "safe-memoryMissing message")]
mod safe_memory {
    use wrt_foundation::ResourceOperation as FormatOp;

    use crate::prelude::*;

    /// Convert a Core ResourceOperation to a Format ResourceOperation
    pub fn core_to_format_resource_operation(op: &wrt_foundation::ResourceOperation) -> FormatOp {
        match op {
            wrt_foundation::ResourceOperation::New => FormatOp::New,
            wrt_foundation::ResourceOperation::Drop => FormatOp::Drop,
            wrt_foundation::ResourceOperation::Rep => FormatOp::Rep,
        }
    }

    /// Convert a Format ResourceOperation to a Core ResourceOperation
    pub fn format_to_core_resource_operation(op: &FormatOp) -> wrt_foundation::ResourceOperation {
        match op {
            FormatOp::New => wrt_foundation::ResourceOperation::New,
            FormatOp::Drop => wrt_foundation::ResourceOperation::Drop,
            FormatOp::Rep => wrt_foundation::ResourceOperation::Rep,
        }
    }
}

#[cfg(test)]
mod tests {
    use wrt_format::component::FormatResourceOperation as FormatOp;
    use wrt_foundation::resource::{ResourceDrop, ResourceNew, ResourceRep};

    use super::*;

    #[test]
    fn test_operation_permissions() {
        assert!(ResourceOperation::Read.requires_read();
        assert!(!ResourceOperation::Read.requires_write();

        assert!(ResourceOperation::Write.requires_write();
        assert!(!ResourceOperation::Write.requires_read();

        assert!(ResourceOperation::Execute.requires_read();
        assert!(!ResourceOperation::Execute.requires_write();

        assert!(ResourceOperation::Create.requires_write();
        assert!(!ResourceOperation::Create.requires_read();

        assert!(ResourceOperation::Delete.requires_write();
        assert!(!ResourceOperation::Delete.requires_read();

        assert!(ResourceOperation::Reference.requires_write();
        assert!(!ResourceOperation::Reference.requires_read();

        assert!(ResourceOperation::Dereference.requires_read();
        assert!(!ResourceOperation::Dereference.requires_write();
    }

    #[test]
    fn test_format_conversion() {
        // Test conversion to format types
        let type_idx = 42;

        let read_op = to_format_resource_operation(ResourceOperation::Read, type_idx);
        if let FormatOp::Rep(rep) = read_op {
            assert_eq!(rep.type_idx, type_idx);
        } else {
            panic!("Unexpected operation typeMissing message");
        }

        let create_op = to_format_resource_operation(ResourceOperation::Create, type_idx);
        if let FormatOp::New(new) = create_op {
            assert_eq!(new.type_idx, type_idx);
        } else {
            panic!("Unexpected operation typeMissing message");
        }

        // Test conversion from format types
        assert_eq!(
            from_format_resource_operation(&FormatOp::Rep(ResourceRep { type_idx })),
            ResourceOperation::Read
        );

        assert_eq!(
            from_format_resource_operation(&FormatOp::New(ResourceNew { type_idx })),
            ResourceOperation::Create
        );

        assert_eq!(
            from_format_resource_operation(&FormatOp::Drop(ResourceDrop { type_idx })),
            ResourceOperation::Delete
        );
    }
}
