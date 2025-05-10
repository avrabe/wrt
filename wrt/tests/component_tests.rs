use wrt::interface::{CanonicalABI, InterfaceValue};
use wrt::resource::ResourceData;
use wrt::resource::ResourceRepresentation;
use wrt::resource::ResourceTable;
use wrt::resource::ResourceType;
use wrt::types::*;
use wrt::{Component, Error, Result, ValueType};

use std::sync::Arc;

#[test]
fn test_component_instantiation() -> Result<()> {
    // Create a simple component with a basic component type
    let component_type = wrt::component::ComponentType {
        imports: Vec::new(),
        exports: vec![
            (
                "add".to_string(),
                ExternType::Function(FuncType {
                    params: vec![ValueType::I32, ValueType::I32],
                    results: vec![ValueType::I32],
                }),
            ),
            (
                "hello".to_string(),
                ExternType::Function(FuncType {
                    params: Vec::new(),
                    results: vec![ValueType::I32],
                }),
            ),
        ],
        instances: Vec::new(),
    };

    let mut component = Component::new(component_type);
    component.instantiate(Vec::new())?;

    // Check that the exports are accessible
    let add_export = component.get_export("add")?;
    assert_eq!(add_export.name, "add");
    match &add_export.ty {
        ExternType::Function(func_type) => {
            assert_eq!(func_type.params.len(), 2);
            assert_eq!(func_type.results.len(), 1);
        }
        _ => return Err(Error::Validation("Expected function type".into())),
    };

    let hello_export = component.get_export("hello")?;
    assert_eq!(hello_export.name, "hello");
    match &hello_export.ty {
        ExternType::Function(func_type) => {
            assert_eq!(func_type.params.len(), 0);
            assert_eq!(func_type.results.len(), 1);
        }
        _ => return Err(Error::Validation("Expected function type".into())),
    };

    Ok(())
}

#[test]
fn test_canonical_abi_conversion() -> Result<()> {
    // Test lifting/lowering of primitive values
    let i32_val = wrt::Value::I32(42);
    let i32_type = ComponentType::Primitive(ValueType::I32);
    let interface_val = CanonicalABI::lift(i32_val.clone(), &i32_type, None, None)?;

    assert!(matches!(interface_val, InterfaceValue::S32(42)));

    let lowered_val = CanonicalABI::lower(interface_val, None, None)?;
    assert_eq!(lowered_val, i32_val);

    Ok(())
}

#[test]
fn test_resource_handling() -> Result<()> {
    // Create a resource table
    let mut table = ResourceTable::new();

    // Define a resource type
    let resource_type = ResourceType {
        name: "test:resource".to_string(),
        representation: ResourceRepresentation::Handle32,
        nullable: false,
        borrowable: true,
    };

    // Create some test resource data
    struct TestResourceData {
        value: String,
    }

    impl ResourceData for TestResourceData {
        // ResourceData requires Debug + Send + Sync and as_any
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    impl std::fmt::Debug for TestResourceData {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestResourceData {{ value: {} }}", self.value)
        }
    }

    // Allocate a resource
    let data = Arc::new(TestResourceData { value: "test value".to_string() });

    let id = table.allocate(resource_type.clone(), data);

    // Get the resource back
    let resource = table.get(id)?;

    // Check the resource
    assert_eq!(resource.id, id);
    assert_eq!(resource.resource_type.name, "test:resource");

    // Add some references
    table.add_ref(id)?;
    table.add_ref(id)?;

    // Drop references
    table.drop_ref(id)?;
    table.drop_ref(id)?;

    // Resource should still exist
    assert!(table.get(id).is_ok());

    // Drop the last reference
    table.drop_ref(id)?;

    // Resource should no longer exist
    assert!(table.get(id).is_err());

    Ok(())
}

#[test]
fn test_component_types() {
    // Create nested component types

    // 1. Record type
    let record_type = ComponentType::Record(vec![
        ("name".to_string(), Box::new(ComponentType::Primitive(ValueType::I32))),
        ("age".to_string(), Box::new(ComponentType::Primitive(ValueType::I32))),
    ]);

    assert!(record_type.is_record());

    // 2. List of records
    let list_type = ComponentType::List(Box::new(record_type.clone()));

    assert!(list_type.is_list());

    // 3. Result type with record as ok
    let result_type = ComponentType::Result {
        ok: Some(Box::new(record_type)),
        err: Some(Box::new(ComponentType::Primitive(ValueType::I32))),
    };

    assert!(result_type.is_result());

    // 4. Option type with list as value
    let option_type = ComponentType::Option(Box::new(list_type));

    assert!(option_type.is_option());
}

#[test]
fn test_component_binary_parsing() -> Result<()> {
    // Create a minimal component binary with just the header and mandatory component-type-section
    let component_binary = [
        // Component magic number and version
        0x00, 0x61, 0x73, 0x6D, // magic
        0x0D, 0x00, 0x01, 0x00, // component model version
        // Component Type Section (section code 1)
        0x01, // section code
        0x02, // section size
        0x00, // number of types (0)
        0x00, // padding byte to meet minimum length
    ];

    // Load the component binary using the correct pattern
    let loaded_module = wrt::module::Module::from_bytes(&component_binary)?;

    // Verify that the module contains component-model-info section
    let component_info = loaded_module
        .custom_sections
        .iter()
        .find(|section| section.name == "component-model-info")
        .expect("Component model info section not found");

    // Verify it's marked as a component
    assert_eq!(component_info.data, vec![0x01]);

    Ok(())
}

#[test]
fn test_component_validation() -> Result<()> {
    // Create an invalid component binary (incorrect magic number)
    let invalid_binary = [
        // Invalid magic
        0x00, 0x61, 0x73, 0x00, // wrong magic
        0x0D, 0x00, 0x01, 0x00, // component model version
    ];

    // Loading should fail
    let result = wrt::module::Module::from_bytes(&invalid_binary);
    assert!(result.is_err());

    // Create an invalid component binary (no core module or type section)
    let invalid_binary = [
        // Valid magic and version
        0x00, 0x61, 0x73, 0x6D, // magic
        0x0D, 0x00, 0x01, 0x00, // component model version
              // No sections
    ];

    // Loading should fail
    let result = wrt::module::Module::from_bytes(&invalid_binary);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_component_linking() -> Result<()> {
    // Create a parent component with imports
    let parent_component_type = wrt::component::ComponentType {
        imports: vec![(
            "log".to_string(),
            "wasi".to_string(),
            ExternType::Function(FuncType { params: vec![ValueType::I32], results: vec![] }),
        )],
        exports: vec![(
            "process".to_string(),
            ExternType::Function(FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            }),
        )],
        instances: Vec::new(),
    };

    // Create a child component with imports and exports
    let child_component_type = wrt::component::ComponentType {
        imports: vec![(
            "process".to_string(),
            "parent".to_string(),
            ExternType::Function(FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            }),
        )],
        exports: vec![(
            "transform".to_string(),
            ExternType::Function(FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            }),
        )],
        instances: Vec::new(),
    };

    // Instantiate the parent component
    let mut parent = Component::new(parent_component_type);
    let parent_import = wrt::component::Import {
        name: "log".to_string(),
        ty: ExternType::Function(FuncType { params: vec![ValueType::I32], results: vec![] }),
        value: wrt::component::ExternValue::Function(wrt::component::FunctionValue {
            ty: FuncType { params: vec![ValueType::I32], results: vec![] },
            export_name: "log".to_string(),
        }),
    };
    parent.instantiate(vec![parent_import])?;

    // Instantiate the child component
    let mut child = Component::new(child_component_type);

    // Create an import for the child that is linked to the parent's export
    let child_import = wrt::component::Import {
        name: "process".to_string(),
        ty: ExternType::Function(FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        }),
        value: wrt::component::ExternValue::Function(wrt::component::FunctionValue {
            ty: FuncType { params: vec![ValueType::I32], results: vec![ValueType::I32] },
            export_name: "process".to_string(),
        }),
    };
    child.instantiate(vec![child_import])?;

    // Link the child component to the parent using namespace
    parent.import_component(&child, Some("child"))?;

    // Verify the child's export is accessible from the parent with proper namespace
    let export = parent.get_export("child.transform")?;
    assert_eq!(export.name, "child.transform");
    match &export.ty {
        ExternType::Function(func_type) => {
            assert_eq!(func_type.params.len(), 1);
            assert_eq!(func_type.results.len(), 1);
        }
        _ => panic!("Expected function type"),
    }

    // Validate components
    assert!(parent.validate().is_ok());
    assert!(child.validate().is_ok());

    Ok(())
}
