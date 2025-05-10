//! Export mapping for the WebAssembly Component Model
//!
//! This module provides functionality to map between different export representations.

use crate::export::Export;
use crate::prelude::*;

/// Map of export names to exports
#[derive(Debug, Default)]
pub struct ExportMap {
    /// Name-to-export mapping
    exports: HashMap<String, Arc<Export>>,
}

/// Map of export names to exports using SafeMemory
#[cfg(feature = "safe-memory")]
#[derive(Debug)]
pub struct SafeExportMap {
    /// Name-to-export mapping with safe memory guarantees
    exports: wrt_types::safe_memory::SafeStack<(String, Arc<Export>)>,
}

impl ExportMap {
    /// Create a new empty export map
    pub fn new() -> Self {
        Self { exports: HashMap::new() }
    }

    /// Add an export to the map
    pub fn add(&mut self, name: &str, export: Arc<Export>) -> Result<()> {
        self.exports.insert(name.to_string(), export);
        Ok(())
    }

    /// Get an export by name
    pub fn get(&self, name: &str) -> Option<Arc<Export>> {
        self.exports.get(name).cloned()
    }

    /// Remove an export by name
    pub fn remove(&mut self, name: &str) -> Option<Arc<Export>> {
        self.exports.remove(name)
    }

    /// Check if an export exists by name
    pub fn contains(&self, name: &str) -> bool {
        self.exports.contains_key(name)
    }

    /// Get all export names
    pub fn names(&self) -> Vec<String> {
        self.exports.keys().cloned().collect()
    }

    /// Get all exports as a Vec of (name, export) pairs
    pub fn get_all(&self) -> Vec<(String, Arc<Export>)> {
        self.exports.iter().map(|(name, export)| (name.clone(), export.clone())).collect()
    }

    /// Convert this export map to one using SafeMemory containers
    #[cfg(feature = "safe-memory")]
    pub fn to_safe_memory(&self) -> SafeExportMap {
        let mut result = SafeExportMap::new();
        for (name, export) in &self.exports {
            result.add(name, export.clone()).unwrap();
        }
        result
    }
}

#[cfg(feature = "safe-memory")]
impl SafeExportMap {
    /// Create a new empty export map
    pub fn new() -> Self {
        Self { exports: wrt_types::safe_memory::SafeStack::new() }
    }

    /// Add an export to the map
    pub fn add(&mut self, name: &str, export: Arc<Export>) -> Result<()> {
        // Check if the name already exists
        for i in 0..self.exports.len() {
            let (existing_name, _) = self.exports.get(i)?;
            if existing_name == name {
                // Replace the existing export
                self.exports.set(i, (name.to_string(), export))?;
                return Ok(());
            }
        }

        // Name doesn't exist, add a new entry
        self.exports.push((name.to_string(), export))?;
        Ok(())
    }

    /// Get an export by name
    pub fn get(&self, name: &str) -> Option<Arc<Export>> {
        // Search for the name
        for i in 0..self.exports.len() {
            if let Ok((existing_name, export)) = self.exports.get(i) {
                if existing_name == name {
                    return Some(export);
                }
            }
        }
        None
    }

    /// Remove an export by name
    pub fn remove(&mut self, name: &str) -> Option<Arc<Export>> {
        // Search for the name
        for i in 0..self.exports.len() {
            if let Ok((existing_name, _)) = self.exports.get(i) {
                if existing_name == name {
                    // Found the name, remove it
                    if let Ok(items) = self.exports.to_vec() {
                        let export = items[i].1.clone();
                        // Create a new vector without the removed item
                        let mut new_items = Vec::with_capacity(items.len() - 1);
                        for (j, item) in items.into_iter().enumerate() {
                            if j != i {
                                new_items.push(item);
                            }
                        }

                        // Clear and rebuild the stack
                        self.exports.clear();
                        for item in new_items {
                            let _ = self.exports.push(item);
                        }

                        return Some(export);
                    }
                }
            }
        }
        None
    }

    /// Check if an export exists by name
    pub fn contains(&self, name: &str) -> bool {
        // Search for the name
        for i in 0..self.exports.len() {
            if let Ok((existing_name, _)) = self.exports.get(i) {
                if existing_name == name {
                    return true;
                }
            }
        }
        false
    }

    /// Get all export names
    pub fn names(&self) -> Result<Vec<String>> {
        let mut names = Vec::with_capacity(self.exports.len());
        for i in 0..self.exports.len() {
            if let Ok((name, _)) = self.exports.get(i) {
                names.push(name);
            }
        }
        Ok(names)
    }

    /// Get all exports as a Vec of (name, export) pairs
    pub fn get_all(&self) -> Result<Vec<(String, Arc<Export>)>> {
        self.exports.to_vec()
    }

    /// Convert to standard ExportMap
    pub fn to_standard(&self) -> Result<ExportMap> {
        let mut result = ExportMap::new();
        let items = self.exports.to_vec()?;
        for (name, export) in items {
            result.add(&name, export)?;
        }
        Ok(result)
    }

    /// Get the number of exports
    pub fn len(&self) -> usize {
        self.exports.len()
    }

    /// Check if the export map is empty
    pub fn is_empty(&self) -> bool {
        self.exports.is_empty()
    }

    /// Set the verification level for memory operations
    pub fn set_verification_level(&mut self, level: wrt_types::verification::VerificationLevel) {
        self.exports.set_verification_level(level);
    }
}
