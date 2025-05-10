//! Component registry for the WebAssembly Component Model
//!
//! This module provides registry functionality for components.

use crate::component::Component;
use crate::prelude::*;

/// Registry for WebAssembly components
#[derive(Debug, Default)]
pub struct ComponentRegistry {
    /// Name-to-component mapping
    components: HashMap<String, Arc<Component>>,
}

impl ComponentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self { components: HashMap::new() }
    }

    /// Register a component by name
    pub fn register(&mut self, name: &str, component: Arc<Component>) -> Result<()> {
        self.components.insert(name.to_string(), component);
        Ok(())
    }

    /// Get a component by name
    pub fn get(&self, name: &str) -> Option<Arc<Component>> {
        self.components.get(name).cloned()
    }

    /// Remove a component by name
    pub fn remove(&mut self, name: &str) -> Option<Arc<Component>> {
        self.components.remove(name)
    }

    /// Check if a component exists by name
    pub fn contains(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    /// Get all component names
    pub fn names(&self) -> Vec<String> {
        self.components.keys().cloned().collect()
    }
}
