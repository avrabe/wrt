//! Version information for wrt-format.
//!
//! This module provides utilities for handling versioning and feature detection in WebAssembly
//! Component Model binaries.

use crate::HashMap;

/// Current state serialization format version
pub const STATE_VERSION: u32 = 1;

/// Magic bytes that identify WRT state sections
pub const STATE_MAGIC: &[u8; 4] = b"WRT\0";

/// Represents the version of a WebAssembly Component Model binary
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentModelVersion {
    /// Draft version (pre-standardization)
    Draft,
    /// Version 1.0
    V1_0,
    // Future versions can be added here
}

impl Default for ComponentModelVersion {
    fn default() -> Self {
        Self::V1_0
    }
}

/// Feature flags for different Component Model capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ComponentModelFeature {
    /// Core module support
    CoreModule,
    /// Core instance support
    CoreInstance,
    /// Core type support
    CoreType,
    /// Component type support
    ComponentType,
    /// Instance support
    Instance,
    /// Alias support
    Alias,
    /// Canonical function conversion support
    Canon,
    /// Start function support
    Start,
    /// Import support
    Import,
    /// Export support
    Export,
    /// Value support (🪙 Experimental)
    Value,
    /// Resource types support (🪙 Experimental)
    ResourceTypes,
}

/// Status of feature support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureStatus {
    /// Feature is not available
    Unavailable,
    /// Feature is available but experimental
    ExperimentalSupported,
    /// Feature is fully supported
    FullySupported,
}

/// Holds information about a Component Model binary version and supported features
#[derive(Debug, Clone)]
pub struct VersionInfo {
    /// The detected version
    pub version: ComponentModelVersion,
    /// Map of features to their support status
    features: HashMap<ComponentModelFeature, FeatureStatus>,
    /// Whether this binary uses any experimental features
    pub uses_experimental: bool,
}

impl Default for VersionInfo {
    fn default() -> Self {
        let mut info = Self {
            version: ComponentModelVersion::default(),
            features: HashMap::new(),
            uses_experimental: false,
        };

        // Initialize with default feature set for V1.0
        info.initialize_v1_0_features();

        info
    }
}

impl VersionInfo {
    /// Create a new VersionInfo from the binary version field
    pub fn from_version_bytes(version_bytes: [u8; 4]) -> Self {
        let mut info = VersionInfo::default();

        // First two bytes are the version, next two are the layer
        let version = [version_bytes[0], version_bytes[1]];

        // Detect version
        match version {
            // Version 1.0
            [0x01, 0x00] => {
                info.version = ComponentModelVersion::V1_0;
                info.initialize_v1_0_features();
            }
            // Unknown/future version - default to V1.0 with minimal features
            _ => {
                info.version = ComponentModelVersion::Draft;
                info.initialize_minimal_features();
            }
        }

        info
    }

    /// Initialize features for version 1.0
    fn initialize_v1_0_features(&mut self) {
        // Standard features in V1.0
        self.features.insert(ComponentModelFeature::CoreModule, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::CoreInstance, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::CoreType, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::ComponentType, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::Instance, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::Alias, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::Canon, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::Start, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::Import, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::Export, FeatureStatus::FullySupported);

        // Experimental features
        #[cfg(feature = "component-model-values")]
        self.features.insert(ComponentModelFeature::Value, FeatureStatus::ExperimentalSupported);
        #[cfg(not(feature = "component-model-values"))]
        self.features.insert(ComponentModelFeature::Value, FeatureStatus::Unavailable);

        #[cfg(feature = "component-model-resources")]
        self.features
            .insert(ComponentModelFeature::ResourceTypes, FeatureStatus::ExperimentalSupported);
        #[cfg(not(feature = "component-model-resources"))]
        self.features.insert(ComponentModelFeature::ResourceTypes, FeatureStatus::Unavailable);
    }

    /// Initialize minimal feature set (for unknown versions)
    fn initialize_minimal_features(&mut self) {
        // Only include core features
        self.features.insert(ComponentModelFeature::CoreModule, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::CoreInstance, FeatureStatus::FullySupported);
        self.features.insert(ComponentModelFeature::CoreType, FeatureStatus::FullySupported);

        // Other features are unavailable
        self.features.insert(ComponentModelFeature::ComponentType, FeatureStatus::Unavailable);
        self.features.insert(ComponentModelFeature::Instance, FeatureStatus::Unavailable);
        self.features.insert(ComponentModelFeature::Alias, FeatureStatus::Unavailable);
        self.features.insert(ComponentModelFeature::Canon, FeatureStatus::Unavailable);
        self.features.insert(ComponentModelFeature::Start, FeatureStatus::Unavailable);
        self.features.insert(ComponentModelFeature::Import, FeatureStatus::Unavailable);
        self.features.insert(ComponentModelFeature::Export, FeatureStatus::Unavailable);
        self.features.insert(ComponentModelFeature::Value, FeatureStatus::Unavailable);
        self.features.insert(ComponentModelFeature::ResourceTypes, FeatureStatus::Unavailable);
    }

    /// Check if a feature is available (either experimental or fully supported)
    pub fn is_feature_available(&self, feature: ComponentModelFeature) -> bool {
        match self.features.get(&feature) {
            Some(status) => *status != FeatureStatus::Unavailable,
            None => false,
        }
    }

    /// Get the status of a feature
    pub fn get_feature_status(&self, feature: ComponentModelFeature) -> FeatureStatus {
        *self.features.get(&feature).unwrap_or(&FeatureStatus::Unavailable)
    }

    /// Determine if the binary uses any experimental features
    pub fn detect_experimental_features(&mut self, binary: &[u8]) -> bool {
        // Placeholder for future implementation
        // In a real implementation, this would scan the binary for sections
        // that correspond to experimental features

        let value_section_present =
            binary.windows(1).any(|window| window[0] == crate::binary::COMPONENT_VALUE_SECTION_ID);
        if value_section_present
            && self.get_feature_status(ComponentModelFeature::Value)
                == FeatureStatus::ExperimentalSupported
        {
            self.uses_experimental = true;
        }

        // Add more checks for other experimental features as needed

        self.uses_experimental
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_detection() {
        // V1.0
        let v1_bytes = [0x01, 0x00, 0x01, 0x00];
        let v1_info = VersionInfo::from_version_bytes(v1_bytes);
        assert_eq!(v1_info.version, ComponentModelVersion::V1_0);

        // Unknown version
        let unknown_bytes = [0x02, 0x00, 0x01, 0x00];
        let unknown_info = VersionInfo::from_version_bytes(unknown_bytes);
        assert_eq!(unknown_info.version, ComponentModelVersion::Draft);
    }

    #[test]
    fn test_feature_availability() {
        let v1_bytes = [0x01, 0x00, 0x01, 0x00];
        let v1_info = VersionInfo::from_version_bytes(v1_bytes);

        // Core features should be available
        assert!(v1_info.is_feature_available(ComponentModelFeature::CoreModule));
        assert!(v1_info.is_feature_available(ComponentModelFeature::CoreInstance));

        // Experimental features depend on compile-time flags
        #[cfg(feature = "component-model-values")]
        assert!(v1_info.is_feature_available(ComponentModelFeature::Value));
        #[cfg(not(feature = "component-model-values"))]
        assert!(!v1_info.is_feature_available(ComponentModelFeature::Value));
    }
}
