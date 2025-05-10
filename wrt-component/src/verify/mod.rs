//! Verification module for wrt-component using Kani.
//!
//! This module contains verification harnesses for the wrt-component crate.
//! It is only included when the `kani` feature is enabled.

use super::*;
use wrt_error::Result;
use wrt_host::CallbackRegistry;

#[cfg(kani)]
#[kani::proof]
fn verify_component_type() {
    // Create a component type
    let component_type =
        ComponentType { imports: Vec::new(), exports: Vec::new(), instances: Vec::new() };

    // Verify that it can be used to create a component
    let component = Component::new(component_type);

    // Verify basic properties
    assert!(component.exports.is_empty());
    assert!(component.imports.is_empty());
    assert!(component.instances.is_empty());
}

#[cfg(kani)]
#[kani::proof]
fn verify_namespace() {
    // Create a namespace
    let ns = Namespace::from_string("wasi.http.client");

    // Verify properties
    assert_eq!(ns.elements.len(), 3);
    assert_eq!(ns.elements[0], "wasi");
    assert_eq!(ns.elements[1], "http");
    assert_eq!(ns.elements[2], "client");

    // Test matching
    let ns2 = Namespace::from_string("wasi.http.client");
    assert!(ns.matches(&ns2));

    // Test non-matching
    let ns3 = Namespace::from_string("wasi.fs");
    assert!(!ns.matches(&ns3));

    // Test empty
    let empty = Namespace::from_string("");
    assert!(empty.is_empty());
}

#[cfg(kani)]
#[kani::proof]
fn verify_host() {
    // Create a host
    let mut host = Host::new();

    // Verify it starts empty
    assert!(host.get_function("test").is_none());

    // Add a function
    let func_value = FunctionValue {
        ty: FuncType { params: Vec::new(), results: Vec::new() },
        export_name: "test".to_string(),
    };

    host.add_function("test".to_string(), func_value.clone());

    // Verify function is found
    let retrieved = host.get_function("test");
    assert!(retrieved.is_some());
}
