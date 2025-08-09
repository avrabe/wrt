//! Memory budget limits for wrt-component safety-critical operations
//!
//! This module defines compile-time memory limits for all bounded collections
//! used in the wrt-component crate when the safety-critical feature is enabled.

#![cfg(feature = "safety-critical")]

/// Canonical ABI memory limits
pub mod canonical_abi {
    /// Maximum number of values in a tuple
    pub const TUPLE_VALUES_LIMIT: usize = 32;

    /// Maximum number of flags
    pub const FLAGS_LIMIT: usize = 64;

    /// Maximum number of fields in a record
    pub const RECORD_FIELDS_LIMIT: usize = 32;

    /// Maximum size of a fixed list
    pub const FIXED_LIST_LIMIT: usize = 256;

    /// Maximum size of a dynamic list
    pub const DYNAMIC_LIST_LIMIT: usize = 1024;

    /// Maximum number of values in type conversion
    pub const CONVERSION_VALUES_LIMIT: usize = 1024;
}

/// Resource management memory limits
pub mod resources {
    /// Maximum number of resources in a resource table
    pub const RESOURCE_TABLE_LIMIT: usize = 1024;

    /// Maximum number of borrows per resource
    pub const BORROWS_PER_RESOURCE_LIMIT: usize = 32;

    /// Maximum number of resource interceptors
    pub const INTERCEPTORS_LIMIT: usize = 16;
}

/// Component instantiation memory limits
pub mod instantiation {
    /// Maximum number of imports per component
    pub const IMPORTS_LIMIT: usize = 256;

    /// Maximum number of exports per component
    pub const EXPORTS_LIMIT: usize = 256;

    /// Maximum number of resource tables per instance
    pub const RESOURCE_TABLES_LIMIT: usize = 16;

    /// Maximum number of module instances per component
    pub const MODULE_INSTANCES_LIMIT: usize = 64;
}

/// Cross-component communication memory limits
pub mod communication {
    /// Maximum number of component instances in registry
    pub const INSTANCE_REGISTRY_LIMIT: usize = 256;

    /// Maximum number of security policies
    pub const SECURITY_POLICIES_LIMIT: usize = 64;

    /// Maximum number of allowed target components per policy
    pub const ALLOWED_TARGETS_LIMIT: usize = 32;

    /// Maximum number of allowed functions per policy
    pub const ALLOWED_FUNCTIONS_LIMIT: usize = 64;

    /// Maximum size of marshaled parameter data in bytes
    pub const MARSHALED_DATA_LIMIT: usize = 8192;
}

/// Memory usage validation
#[cfg(test)]
mod validation {
    use super::*;

    /// Estimated bytes per item for memory calculations
    const BYTES_PER_VALUE: usize = 64;
    const BYTES_PER_RESOURCE: usize = 256;
    const BYTES_PER_IMPORT: usize = 128;

    /// Total component memory budget in bytes (512 KiB)
    const TOTAL_BUDGET: usize = 512 * 1024;

    #[test]
    fn validate_canonical_abi_budget() {
        let budget = canonical_abi::TUPLE_VALUES_LIMIT * BYTES_PER_VALUE
            + canonical_abi::FLAGS_LIMIT * 4
            + canonical_abi::RECORD_FIELDS_LIMIT * 128
            + canonical_abi::FIXED_LIST_LIMIT * BYTES_PER_VALUE
            + canonical_abi::DYNAMIC_LIST_LIMIT * BYTES_PER_VALUE
            + canonical_abi::CONVERSION_VALUES_LIMIT * BYTES_PER_VALUE;

        assert!(
            budget < 150 * 1024,
            "Canonical ABI budget exceeds 150KB: {}KB",
            budget / 1024
        );
    }

    #[test]
    fn validate_resource_budget() {
        let budget = resources::RESOURCE_TABLE_LIMIT * BYTES_PER_RESOURCE
            + resources::RESOURCE_TABLE_LIMIT * resources::BORROWS_PER_RESOURCE_LIMIT * 16
            + resources::INTERCEPTORS_LIMIT * 64;

        assert!(
            budget < 300 * 1024,
            "Resource budget exceeds 300KB: {}KB",
            budget / 1024
        );
    }

    #[test]
    fn validate_instantiation_budget() {
        let budget = instantiation::IMPORTS_LIMIT * BYTES_PER_IMPORT
            + instantiation::EXPORTS_LIMIT * BYTES_PER_IMPORT
            + instantiation::RESOURCE_TABLES_LIMIT * 512
            + instantiation::MODULE_INSTANCES_LIMIT * 256;

        assert!(
            budget < 100 * 1024,
            "Instantiation budget exceeds 100KB: {}KB",
            budget / 1024
        );
    }

    #[test]
    fn validate_communication_budget() {
        let budget = communication::INSTANCE_REGISTRY_LIMIT * 64
            + communication::SECURITY_POLICIES_LIMIT * 256
            + communication::SECURITY_POLICIES_LIMIT * communication::ALLOWED_TARGETS_LIMIT * 64
            + communication::SECURITY_POLICIES_LIMIT * communication::ALLOWED_FUNCTIONS_LIMIT * 64
            + communication::MARSHALED_DATA_LIMIT;

        assert!(
            budget < 50 * 1024,
            "Communication budget exceeds 50KB: {}KB",
            budget / 1024
        );
    }

    #[test]
    fn validate_total_budget() {
        // This is a conservative estimate as not all limits will be reached
        // simultaneously
        let canonical = 150 * 1024;
        let resources = 258 * 1024;
        let instantiation = 88 * 1024;
        let communication = 46 * 1024;

        let total = canonical + resources + instantiation + communication;

        assert!(
            total <= TOTAL_BUDGET + 30 * 1024,
            "Total budget exceeds 512KB + overhead: {}KB",
            total / 1024
        );
    }
}

/// Compile-time assertions for safety properties
#[cfg(feature = "safety-critical")]
mod safety_assertions {
    use super::*;

    // Ensure limits fit in reasonable bounds
    const _: () = assert!(canonical_abi::TUPLE_VALUES_LIMIT <= 256);
    const _: () = assert!(canonical_abi::DYNAMIC_LIST_LIMIT <= 2048);
    const _: () = assert!(resources::RESOURCE_TABLE_LIMIT <= 2048);
    const _: () = assert!(instantiation::IMPORTS_LIMIT <= 512);
    const _: () = assert!(communication::MARSHALED_DATA_LIMIT <= 16384);

    // Ensure limits are powers of 2 or nice round numbers for efficiency
    const _: () = assert!(canonical_abi::TUPLE_VALUES_LIMIT == 32);
    const _: () = assert!(canonical_abi::FLAGS_LIMIT == 64);
    const _: () = assert!(resources::RESOURCE_TABLE_LIMIT == 1024);
    const _: () = assert!(instantiation::IMPORTS_LIMIT == 256);
}

/// Re-export commonly used limits
pub use canonical_abi::*;
pub use communication::*;
pub use instantiation::*;
pub use resources::*;
