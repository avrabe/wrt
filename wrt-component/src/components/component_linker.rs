//! Component Linker and Import/Export Resolution System


// Cross-environment imports
#[cfg(feature = "std")]
use std::{boxed::Box, collections::HashMap, format, string::String, vec::Vec};

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
use wrt_foundation::{
    bounded::BoundedString as String, bounded::BoundedVec as Vec, 
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

use crate::components::component_instantiation::{
    create_component_export, create_component_import, ComponentExport, ComponentImport,
    ComponentInstance, ExportType, FunctionSignature, ImportType, InstanceConfig, InstanceId,
    ResolvedImport,
};
use wrt_error::{codes, Error, ErrorCategory, Result};

/// Maximum number of components in linker
const MAX_LINKED_COMPONENTS: usize = 256;

/// Component identifier in the linker
pub type ComponentId = String;

/// Component linker for managing multiple components and their dependencies
#[derive(Debug)]
pub struct ComponentLinker {
    /// Registered components
    components: HashMap<ComponentId, ComponentDefinition>,
    /// Active component instances
    instances: HashMap<InstanceId, ComponentInstance>,
    /// Dependency graph
    link_graph: LinkGraph,
    /// Next available instance ID
    next_instance_id: InstanceId,
    /// Linker configuration
    config: LinkerConfig,
    /// Resolution statistics
    stats: LinkingStats,
}

/// Component definition in the linker
#[derive(Debug, Clone)]
pub struct ComponentDefinition {
    /// Component ID
    pub id: ComponentId,
    /// Component binary (simplified as bytes)
    pub binary: BoundedVec<u8, 1048576, NoStdProvider<65536>>, // 1MB max binary size
    /// Parsed exports
    pub exports: BoundedVec<ComponentExport, 64, NoStdProvider<65536>>,
    /// Parsed imports
    pub imports: BoundedVec<ComponentImport, 64, NoStdProvider<65536>>,
    /// Component metadata
    pub metadata: ComponentMetadata,
}

/// Component metadata for introspection
#[derive(Debug, Clone)]
pub struct ComponentMetadata {
    /// Component name
    pub name: String,
    /// Component version
    pub version: String,
    /// Component description
    pub description: String,
    /// Component author
    pub author: String,
    /// Compilation timestamp
    pub compiled_at: u64,
}

/// Dependency graph for component linking
#[derive(Debug, Clone)]
pub struct LinkGraph {
    /// Nodes (components)
    nodes: Vec<GraphNode>,
    /// Edges (dependencies)
    edges: Vec<GraphEdge>,
}

/// Graph node representing a component
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    /// Component ID
    pub component_id: ComponentId,
    /// Node index in graph
    pub index: usize,
    /// Dependencies (outgoing edges)
    pub dependencies: Vec<usize>,
    /// Dependents (incoming edges)
    pub dependents: Vec<usize>,
}

/// Graph edge representing a dependency relationship
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    /// Source node index
    pub from: usize,
    /// Target node index
    pub to: usize,
    /// Import that creates this dependency
    pub import: ComponentImport,
    /// Export that satisfies this dependency
    pub export: ComponentExport,
    /// Edge weight (for optimization)
    pub weight: u32,
}

/// Linker configuration
#[derive(Debug, Clone)]
pub struct LinkerConfig {
    /// Enable strict type checking
    pub strict_typing: bool,
    /// Allow hot swapping of components
    pub allow_hot_swap: bool,
    /// Maximum memory per instance
    pub max_instance_memory: u32,
    /// Enable dependency validation
    pub validate_dependencies: bool,
    /// Circular dependency handling
    pub circular_dependency_mode: CircularDependencyMode,
}

/// Circular dependency handling modes
#[derive(Debug, Clone, PartialEq)]
pub enum CircularDependencyMode {
    /// Reject circular dependencies
    Reject,
    /// Allow circular dependencies (with limitations)
    Allow,
    /// Warn about circular dependencies but allow them
    Warn,
}

/// Linking statistics
#[derive(Debug, Clone, Default)]
pub struct LinkingStats {
    /// Total components registered
    pub components_registered: u32,
    /// Total instances created
    pub instances_created: u32,
    /// Total links resolved
    pub links_resolved: u32,
    /// Resolution failures
    pub resolution_failures: u32,
    /// Last resolution time (microseconds)
    pub last_resolution_time: u64,
}

impl Default for LinkerConfig {
    fn default() -> Self {
        Self {
            strict_typing: true,
            allow_hot_swap: false,
            max_instance_memory: 64 * 1024 * 1024, // 64MB
            validate_dependencies: true,
            circular_dependency_mode: CircularDependencyMode::Reject,
        }
    }
}

impl Default for ComponentMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: String::new(),
            compiled_at: 0,
        }
    }
}

impl ComponentLinker {
    /// Create a new component linker
    pub fn new() -> Self {
        Self::with_config(LinkerConfig::default())
    }

    /// Create a new component linker with custom configuration
    pub fn with_config(config: LinkerConfig) -> Self {
        Self {
            components: HashMap::new(),
            instances: HashMap::new(),
            link_graph: LinkGraph::new(),
            next_instance_id: 1,
            config,
            stats: LinkingStats::default(),
        }
    }

    /// Add a component to the linker
    pub fn add_component(&mut self, id: ComponentId, binary: &[u8]) -> Result<()> {
        if self.components.len() >= MAX_LINKED_COMPONENTS {
            return Err(Error::resource_exhausted("Maximum number of components reached"));
        }

        // Parse component binary (simplified)
        let (exports, imports, metadata) = self.parse_component_binary(binary)?;

        let definition = ComponentDefinition {
            id: id.clone(),
            binary: binary.to_vec(),
            exports,
            imports,
            metadata,
        };

        // Add to components map
        self.components.insert(id.clone(), definition);

        // Update dependency graph
        self.link_graph.add_component(id)?;

        // Update statistics
        self.stats.components_registered += 1;

        Ok(())
    }

    /// Remove a component from the linker
    pub fn remove_component(&mut self, id: &ComponentId) -> Result<()> {
        // Check if component exists
        if !self.components.contains_key(id) {
            return Err(Error::component_not_found("Component not found"));
        }

        // Check if any instances are using this component
        let dependent_instances: Vec<_> = self
            .instances
            .values()
            .filter(|instance| &instance.name == id)
            .map(|instance| instance.id)
            .collect();

        if !dependent_instances.is_empty() {
            return Err(Error::runtime_execution_error(",
            ));
        }

        // Remove from components and graph
        self.components.remove(id);
        self.link_graph.remove_component(id)?;

        Ok(())
    }

    /// Instantiate a component with dependency resolution
    pub fn instantiate(
        &mut self,
        component_id: &ComponentId,
        config: Option<InstanceConfig>,
    ) -> Result<InstanceId> {
        // Find component definition
        let component = self.components.get(component_id).ok_or_else(|| {
            Error::component_not_found(")
        })?;

        // Resolve dependencies
        let resolved_imports = self.resolve_imports(component_id, &component.imports)?;

        // Create instance
        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        let instance_config = config.unwrap_or_else(InstanceConfig::default);

        let mut instance = ComponentInstance::new(
            instance_id,
            component_id.clone(),
            instance_config,
            component.exports.clone(),
            component.imports.clone(),
        )?;

        // Add resolved imports
        for resolved in resolved_imports {
            instance.add_resolved_import(resolved)?;
        }

        // Initialize instance
        instance.initialize()?;

        // Add to instances map
        self.instances.insert(instance_id, instance);

        // Update statistics
        self.stats.instances_created += 1;

        Ok(instance_id)
    }

    /// Link all components and create instances
    pub fn link_all(&mut self) -> Result<Vec<InstanceId>> {
        let mut instance_ids = Vec::new();

        // Topological sort to determine instantiation order
        let sorted_components = self.link_graph.topological_sort()?;

        // Instantiate components in dependency order
        for component_id in sorted_components {
            let instance_id = self.instantiate(&component_id, None)?;
            instance_ids.push(instance_id);
        }

        Ok(instance_ids)
    }

    /// Get a component instance by ID
    pub fn get_instance(&self, instance_id: InstanceId) -> Option<&ComponentInstance> {
        self.instances.get(&instance_id)
    }

    /// Get a mutable component instance by ID
    pub fn get_instance_mut(&mut self, instance_id: InstanceId) -> Option<&mut ComponentInstance> {
        self.instances.get_mut(&instance_id)
    }

    /// Get linking statistics
    pub fn get_stats(&self) -> &LinkingStats {
        &self.stats
    }

    // Private helper methods

    fn parse_component_binary(
        &self,
        binary: &[u8],
    ) -> core::result::Result<(Vec<ComponentExport>, Vec<ComponentImport>, ComponentMetadata)> {
        // Simplified component parsing
        if binary.is_empty() {
            return Err(Error::runtime_execution_error(",
            ));
        }

        // Create some example exports and imports based on binary content
        #[cfg(feature = ")]
        let exports = vec![create_component_export(
            "main".to_string(),
            ExportType::Function(crate::component_instantiation::create_function_signature(
                "main".to_string(),
                vec![],
                vec![crate::canonical_abi::ComponentType::S32],
            )),
        )];
        
        #[cfg(not(feature = "std"))]
        let exports = {
            let mut exports = Vec::new();
            let mut params = Vec::new();
            let mut results = Vec::new();
            results.push(crate::canonical_abi::ComponentType::S32).map_err(|_| Error::platform_memory_allocation_failed("Memory allocation failed"))?;
            
            let signature = crate::component_instantiation::create_function_signature(
                String::new_from_str("main").map_err(|_| Error::platform_memory_allocation_failed("Memory allocation failed"))?,
                params,
                results,
            );
            
            exports.push(create_component_export(
                String::new_from_str("main").map_err(|_| Error::platform_memory_allocation_failed("Memory allocation failed"))?,
                ExportType::Function(signature),
            )).map_err(|_| Error::platform_memory_allocation_failed("Memory allocation failed"))?;
            exports
        };

        #[cfg(feature = "std")]
        let imports = vec![create_component_import(
            "log".to_string(),
            "env".to_string(),
            ImportType::Function(crate::component_instantiation::create_function_signature(
                "log".to_string(),
                vec![crate::canonical_abi::ComponentType::String],
                vec![],
            )),
        )];

        let metadata = ComponentMetadata::default();

        Ok((exports, imports, metadata))
    }

    fn resolve_imports(
        &mut self,
        component_id: &ComponentId,
        imports: &[ComponentImport],
    ) -> Result<Vec<ResolvedImport>> {
        let mut resolved = Vec::new();

        for import in imports {
            let resolution = self.resolve_single_import(component_id, import)?;
            resolved.push(resolution);
        }

        self.stats.links_resolved += resolved.len() as u32;
        Ok(resolved)
    }

    fn resolve_single_import(
        &self,
        _component_id: &ComponentId,
        import: &ComponentImport,
    ) -> Result<ResolvedImport> {
        // Find a component that exports what we need
        for (provider_id, component) in &self.components {
            for export in &component.exports {
                if self.is_compatible_import_export(import, export)? {
                    return Ok(ResolvedImport {
                        import: import.clone(),
                        provider_id: 1, // Simplified - would map component ID to instance ID
                        provider_export: export.name.clone(),
                    });
                }
            }
        }

        Err(Error::component_not_found("Component not found"))
    }

    fn is_compatible_import_export(
        &self,
        import: &ComponentImport,
        export: &ComponentExport,
    ) -> Result<bool> {
        // Check name compatibility
        if import.name != export.name {
            return Ok(false);
        }

        // Check type compatibility
        match (&import.import_type, &export.export_type) {
            (ImportType::Function(import_sig), ExportType::Function(export_sig)) => {
                Ok(self.is_compatible_function_signature(import_sig, export_sig))
            }
            (ImportType::Memory(import_mem), ExportType::Memory(export_mem)) => {
                Ok(self.is_compatible_memory_config(import_mem, export_mem))
            }
            _ => Ok(false), // Other type combinations
        }
    }

    fn is_compatible_function_signature(
        &self,
        import_sig: &FunctionSignature,
        export_sig: &FunctionSignature,
    ) -> bool {
        // Simplified compatibility check
        import_sig.params == export_sig.params && import_sig.returns == export_sig.returns
    }

    fn is_compatible_memory_config(
        &self,
        _import_mem: &crate::component_instantiation::MemoryConfig,
        _export_mem: &crate::component_instantiation::MemoryConfig,
    ) -> bool {
        // Simplified compatibility check
        true
    }
}

impl LinkGraph {
    /// Create a new empty link graph
    pub fn new() -> Self {
        Self { nodes: Vec::new(), edges: Vec::new() }
    }

    /// Add a component to the graph
    pub fn add_component(&mut self, component_id: ComponentId) -> Result<()> {
        // Check if component already exists
        if self.find_node_index(&component_id).is_some() {
            return Err(Error::runtime_execution_error(",
            ));
        }

        let node = GraphNode {
            component_id,
            index: self.nodes.len(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
        };

        self.nodes.push(node);
        Ok(())
    }

    /// Remove a component from the graph
    pub fn remove_component(&mut self, component_id: &ComponentId) -> Result<()> {
        let node_index = self.find_node_index(component_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::RESOURCE_NOT_FOUND,
                ")
        })?;

        // Remove all edges involving this node
        self.edges.retain(|edge| edge.from != node_index && edge.to != node_index);

        // Remove the node
        self.nodes.remove(node_index);

        // Update indices in remaining nodes and edges
        for node in &mut self.nodes[node_index..] {
            node.index -= 1;
        }

        for edge in &mut self.edges {
            if edge.from > node_index {
                edge.from -= 1;
            }
            if edge.to > node_index {
                edge.to -= 1;
            }
        }

        Ok(())
    }

    /// Perform topological sort to determine instantiation order
    pub fn topological_sort(&self) -> Result<Vec<ComponentId>> {
        #[cfg(feature = "std")]
        {
            let mut visited = vec![false; self.nodes.len()];
            let mut temp_visited = vec![false; self.nodes.len()];
            let mut result = Vec::new();
            
            for i in 0..self.nodes.len() {
                if !visited[i] {
                    self.topological_sort_visit(i, &mut visited, &mut temp_visited, &mut result)?;
                }
            }
            
            result.reverse();
            Ok(result)
        }
        #[cfg(not(feature = "std"))]
        {
            // For no_std, create bounded vectors
            let provider = safe_managed_alloc!(65536, CrateId::Component)?;
            let mut visited = BoundedVec::new(provider).map_err(|_| {
                Error::platform_memory_allocation_failed("Failed to create visited vector")
            })?;
            let provider2 = safe_managed_alloc!(65536, CrateId::Component)?;
            let mut temp_visited = BoundedVec::new(provider2).map_err(|_| {
                Error::platform_memory_allocation_failed("Failed to create temp_visited vector")
            })?;
            let mut result = Vec::new();
            
            // Initialize with false values
            for _ in 0..self.nodes.len() {
                visited.push(false).map_err(|_| Error::platform_memory_allocation_failed("Memory allocation failed"))?;
                temp_visited.push(false).map_err(|_| Error::platform_memory_allocation_failed("Memory allocation failed"))?;
            }
            
            for i in 0..self.nodes.len() {
                if !visited[i] {
                    self.topological_sort_visit(i, &mut visited, &mut temp_visited, &mut result)?;
                }
            }
            
            result.reverse();
            Ok(result)
        }
    }

    fn topological_sort_visit(
        &self,
        node_index: usize,
        visited: &mut Vec<bool>,
        temp_visited: &mut Vec<bool>,
        result: &mut Vec<ComponentId>,
    ) -> Result<()> {
        if temp_visited[node_index] {
            return Err(Error::validation_error("Circular dependency detected"));
        }

        if visited[node_index] {
            return Ok(());
        }

        temp_visited[node_index] = true;

        // Visit dependencies first
        for &dep_index in &self.nodes[node_index].dependencies {
            self.topological_sort_visit(dep_index, visited, temp_visited, result)?;
        }

        temp_visited[node_index] = false;
        visited[node_index] = true;
        result.push(self.nodes[node_index].component_id.clone());

        Ok(())
    }

    fn find_node_index(&self, component_id: &ComponentId) -> Option<usize> {
        self.nodes.iter().find(|node| &node.component_id == component_id).map(|node| node.index)
    }
}

impl Default for ComponentLinker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linker_creation() {
        let linker = ComponentLinker::new();
        assert_eq!(linker.components.len(), 0);
        assert_eq!(linker.instances.len(), 0);
        assert_eq!(linker.next_instance_id, 1);
    }

    #[test]
    fn test_add_component() {
        let mut linker = ComponentLinker::new();
        let binary = vec![0x00, 0x61, 0x73, 0x6d]; // "wasm" magic

        let result = linker.add_component("test_component".to_string(), &binary);
        assert!(result.is_ok());
        assert_eq!(linker.components.len(), 1);
        assert_eq!(linker.stats.components_registered, 1);
    }

    #[test]
    fn test_remove_component() {
        let mut linker = ComponentLinker::new();
        let binary = vec![0x00, 0x61, 0x73, 0x6d];

        linker.add_component("test_component".to_string(), &binary).unwrap();
        assert_eq!(linker.components.len(), 1);

        let result = linker.remove_component(&"test_component".to_string());
        assert!(result.is_ok());
        assert_eq!(linker.components.len(), 0);
    }

    #[test]
    fn test_link_graph_operations() {
        let mut graph = LinkGraph::new();

        // Add components
        graph.add_component("comp1".to_string()).unwrap();
        graph.add_component("comp2".to_string()).unwrap();
        assert_eq!(graph.nodes.len(), 2);

        // Remove component
        graph.remove_component(&"comp1".to_string()).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].component_id, "comp2");
    }

    #[test]
    fn test_topological_sort_empty() {
        let graph = LinkGraph::new();
        let result = graph.topological_sort().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_topological_sort_single() {
        let mut graph = LinkGraph::new();
        graph.add_component("comp1".to_string()).unwrap();

        let result = graph.topological_sort().unwrap();
        assert_eq!(result, vec!["comp1".to_string()]);
    }

    #[test]
    fn test_linker_config_default() {
        let config = LinkerConfig::default();
        assert!(config.strict_typing);
        assert!(!config.allow_hot_swap);
        assert_eq!(config.max_instance_memory, 64 * 1024 * 1024);
        assert!(config.validate_dependencies);
        assert_eq!(config.circular_dependency_mode, CircularDependencyMode::Reject);
    }

    #[test]
    fn test_linking_stats() {
        let mut linker = ComponentLinker::new();
        let binary = vec![0x00, 0x61, 0x73, 0x6d];

        linker.add_component("test".to_string(), &binary).unwrap();

        let stats = linker.get_stats();
        assert_eq!(stats.components_registered, 1);
        assert_eq!(stats.instances_created, 0);
    }
}

// Implement required traits for BoundedVec compatibility  
use wrt_foundation::traits::{Checksummable, ToBytes, FromBytes, WriteStream, ReadStream};

// Macro to implement basic traits
macro_rules! impl_basic_traits {
    ($type:ty, $default_val:expr) => {
        impl Checksummable for $type {
            fn update_checksum(&self, checksum: &mut wrt_foundation::traits::Checksum) {
                0u32.update_checksum(checksum);
            }
        }

        impl ToBytes for $type {
            fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                &self,
                _writer: &mut WriteStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<()> {
                Ok(())
            }
        }

        impl FromBytes for $type {
            fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                _reader: &mut ReadStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<Self> {
                Ok($default_val)
            }
        }
    };
}

// Default implementations for complex types
impl Default for GraphEdge {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            import: ComponentImport {
                name: String::new(),
                module: String::new(),
                import_type: ImportType::Function(FunctionSignature {
                    name: String::new(),
                    params: Vec::new(),
                    returns: Vec::new(),
                }),
            },
            export: ComponentExport {
                name: String::new(),
                export_type: ExportType::Function(FunctionSignature {
                    name: String::new(),
                    params: Vec::new(),
                    returns: Vec::new(),
                }),
            },
            weight: 0,
        }
    }
}

impl Default for GraphNode {
    fn default() -> Self {
        Self {
            component_id: String::new(),
            index: 0,
            dependencies: Vec::new(),
            dependents: Vec::new(),
        }
    }
}

impl_basic_traits!(GraphEdge, GraphEdge::default());
impl_basic_traits!(GraphNode, GraphNode::default());
