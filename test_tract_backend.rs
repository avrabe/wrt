//! Simple test for the Tract backend functionality
//!
//! This test verifies that the Tract backend can be created and used
//! without actually loading a real ONNX model.

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Test basic Tract backend creation
    println!("Testing Tract backend creation...");
    
    // Import the necessary types
    use wrt_wasi::nn::{
        capabilities::DynamicNNCapability,
        tract_backend::{TractBackend, TractBackendProvider},
        backend::{BackendProvider, initialize_backends},
        GraphEncoding,
    };
    
    // Create a capability
    let capability = DynamicNNCapability::new();
    println!("✓ Created DynamicNNCapability");
    
    // Create Tract backend
    let backend = TractBackend::new(capability);
    println!("✓ Created TractBackend");
    
    // Test encoding support
    assert!(backend.supports_encoding(GraphEncoding::ONNX));
    assert!(backend.supports_encoding(GraphEncoding::TractNative));
    assert!(!backend.supports_encoding(GraphEncoding::TensorFlow));
    println!("✓ Encoding support verified");
    
    // Test backend provider
    let provider = TractBackendProvider::new();
    assert!(provider.supports_encoding(GraphEncoding::ONNX));
    println!("✓ TractBackendProvider created and tested");
    
    // Test backend registry initialization
    initialize_backends()?;
    println!("✓ Backend registry initialized");
    
    println!("All Tract backend tests passed! 🎉");
    
    Ok(())
}