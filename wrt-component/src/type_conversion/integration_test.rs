#[cfg(test)]
mod integration_tests {
    use wrt_format::component::{ExternType as FormatExternType, ValType as FormatValType};
    use wrt_foundation::{
        component::{ComponentType, FuncType, InstanceType},
        component_value::ValType as TypesValType,
        types::ValueType,
        ExternType as TypesExternType,
    };

    use super::super::{
        registry::TypeConversionRegistry,
        wrappers::{
            FormatComponentType, FormatInstanceType, RuntimeComponentType, RuntimeInstanceType,
        },
    };

    #[test]
    fn test_complex_type_conversion() {
        // Create a registry with default conversions
        let registry = TypeConversionRegistry::with_defaults();

        // Create a complex nested type
        let format_type = FormatValType::Record(vec![
            (
                "person".to_string(),
                FormatValType::Record(vec![
                    ("name".to_string(), FormatValType::String),
                    ("age".to_string(), FormatValType::U32),
                    ("address".to_string(), FormatValType::Option(Box::new(FormatValType::String))),
                ]),
            ),
            ("skills".to_string(), FormatValType::List(Box::new(FormatValType::String))),
            (
                "status".to_string(),
                FormatValType::Variant(vec![
                    ("active".to_string(), None),
                    ("onLeave".to_string(), Some(FormatValType::U32)),
                    ("terminated".to_string(), Some(FormatValType::String)),
                ]),
            ),
        ]);

        // Convert from format to types
        let types_type = registry.convert::<FormatValType, TypesValType(&format_type).unwrap();

        // Convert back to format
        let format_type_back =
            registry.convert::<TypesValType, FormatValType(&types_type).unwrap();

        // The structure should be preserved after round-trip conversion
        // (In a real test, we'd do deep comparison, but that would require implementing
        // PartialEq)
        if let FormatValType::Record(fields) = &format_type_back {
            assert_eq!(fields.len(), 3);
            assert!(fields.iter().any(|(name, _)| name == "person"));
            assert!(fields.iter().any(|(name, _)| name == "skills"));
            assert!(fields.iter().any(|(name, _)| name == "status"));
        } else {
            panic!("Expected FormatValType::Record");
        }
    }

    #[test]
    fn test_function_type_conversion() {
        // Create a registry with default conversions
        let registry = TypeConversionRegistry::with_defaults();

        // Create a function type
        let format_function = FormatExternType::Function {
            params: vec![
                ("input".to_string(), FormatValType::String),
                (
                    "options".to_string(),
                    FormatValType::Record(vec![
                        ("timeout".to_string(), FormatValType::U32),
                        ("retry".to_string(), FormatValType::Bool),
                    ]),
                ),
            ],
            results: vec![FormatValType::Result(Box::new(FormatValType::Variant(vec![
                ("ok".to_string(), Some(FormatValType::String)),
                (
                    "err".to_string(),
                    Some(FormatValType::Variant(vec![
                        ("notFound".to_string(), None),
                        ("timeout".to_string(), None),
                        ("other".to_string(), Some(FormatValType::String)),
                    ])),
                ),
            ])))],
        };

        // Convert to runtime type
        let types_function =
            registry.convert::<FormatExternType, TypesExternType>(&format_function).unwrap();

        // Convert back to format type
        let format_function_back =
            registry.convert::<TypesExternType, FormatExternType>(&types_function).unwrap();

        // Check structure is preserved
        if let FormatExternType::Function { params, results } = &format_function_back {
            assert_eq!(params.len(), 2);
            assert_eq!(results.len(), 1);
            assert_eq!(params[0].0, "input");
            assert_eq!(params[1].0, "options");

            if let FormatValType::Record(fields) = &params[1].1 {
                assert_eq!(fields.len(), 2);
                assert!(fields.iter().any(|(name, _)| name == "timeout"));
                assert!(fields.iter().any(|(name, _)| name == "retry"));
            } else {
                panic!("Expected FormatValType::Record for options parameter");
            }

            if let FormatValType::Result(inner) = &results[0] {
                if let FormatValType::Variant(cases) = &**inner {
                    assert_eq!(cases.len(), 2);
                    assert!(cases.iter().any(|(name, _)| name == "ok"));
                    assert!(cases.iter().any(|(name, _)| name == "err"));
                } else {
                    panic!("Expected FormatValType::Variant for result type");
                }
            } else {
                panic!("Expected FormatValType::Result for result type");
            }
        } else {
            panic!("Expected FormatExternType::Function");
        }
    }

    #[test]
    fn test_component_instance_conversion() {
        // Create a registry with default conversions
        let registry = TypeConversionRegistry::with_defaults();

        // Create a component type with imports and exports
        let log_func = FuncType::new(vec![("message".to_string(), TypesValType::String)], vec![]);

        let greet_func = FuncType::new(
            vec![("name".to_string(), TypesValType::String)],
            vec![TypesValType::String],
        );

        let component_type = ComponentType {
            imports: vec![(
                "environment".to_string(),
                "log".to_string(),
                TypesExternType::Function(log_func),
            )],
            exports: vec![("greet".to_string(), TypesExternType::Function(greet_func))],
            instances: vec![],
        };

        // Wrap in RuntimeComponentType
        let runtime_type = RuntimeComponentType::new(component_type);

        // Convert to FormatComponentType
        let format_type =
            registry.convert::<RuntimeComponentType, FormatComponentType(&runtime_type).unwrap();

        // Check the structure
        assert_eq!(format_type.imports.len(), 1);
        assert_eq!(format_type.exports.len(), 1);
        assert_eq!(format_type.imports[0].0, "environment");
        assert_eq!(format_type.imports[0].1, "log");
        assert_eq!(format_type.exports[0].0, "greet");

        // Convert back to RuntimeComponentType
        let runtime_type_back =
            registry.convert::<FormatComponentType, RuntimeComponentType(&format_type).unwrap();
        let inner = runtime_type_back.inner();

        // Check the structure is preserved
        assert_eq!(inner.imports.len(), 1);
        assert_eq!(inner.exports.len(), 1);
        assert_eq!(inner.imports[0].0, "environment");
        assert_eq!(inner.imports[0].1, "log");
        assert_eq!(inner.exports[0].0, "greet");

        // Test with instances
        let get_func = FuncType::new(vec![], vec![TypesValType::String]);

        let set_func = FuncType::new(vec![("data".to_string(), TypesValType::String)], vec![]);

        let instance_type = InstanceType {
            exports: vec![
                ("getData".to_string(), TypesExternType::Function(get_func)),
                ("setData".to_string(), TypesExternType::Function(set_func)),
            ],
        };
        let runtime_instance = RuntimeInstanceType::new(instance_type);

        // Convert to format type
        let format_instance =
            registry.convert::<RuntimeInstanceType, FormatInstanceType>(&runtime_instance).unwrap();

        // Check the structure
        assert_eq!(format_instance.exports.len(), 2);
        assert_eq!(format_instance.exports[0].0, "getData");
        assert_eq!(format_instance.exports[1].0, "setData");

        // Convert back to runtime type
        let runtime_instance_back =
            registry.convert::<FormatInstanceType, RuntimeInstanceType>(&format_instance).unwrap();
        let inner = runtime_instance_back.inner();

        // Check the structure is preserved
        assert_eq!(inner.exports.len(), 2);
        assert_eq!(inner.exports[0].0, "getData");
        assert_eq!(inner.exports[1].0, "setData");
    }
}
