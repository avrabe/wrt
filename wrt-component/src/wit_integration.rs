#[cfg(not(feature = "std"))]
use std::{collections::BTreeMap, vec::Vec};
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use wrt_foundation::{
    bounded_collections::{BoundedString, BoundedVec, MAX_GENERATIVE_TYPES},
    prelude::*,
};

use crate::{
    async_types::{Future, FutureHandle, Stream, StreamHandle},
    generative_types::{BoundKind, GenerativeResourceType, GenerativeTypeRegistry, TypeBound},
    types::{ComponentError, ComponentInstanceId, TypeId, ValType},
};

use wrt_format::wit_parser::{
    WitFunction, WitInterface, WitParseError, WitParser, WitType, WitWorld,
};

#[derive(Debug, Clone)]
pub struct WitComponentBuilder {
    parser: WitParser,
    type_registry: GenerativeTypeRegistry,
    wit_type_mappings: BTreeMap<BoundedString<64, NoStdProvider<65536>>, TypeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentInterface {
    pub name: BoundedString<64, NoStdProvider<65536>>,
    pub imports: BoundedVec<InterfaceFunction, MAX_GENERATIVE_TYPES, NoStdProvider<65536>>,
    pub exports: BoundedVec<InterfaceFunction, MAX_GENERATIVE_TYPES, NoStdProvider<65536>>,
    pub async_imports: BoundedVec<AsyncInterfaceFunction, MAX_GENERATIVE_TYPES, NoStdProvider<65536>>,
    pub async_exports: BoundedVec<AsyncInterfaceFunction, MAX_GENERATIVE_TYPES, NoStdProvider<65536>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceFunction {
    pub name: BoundedString<64, NoStdProvider<65536>>,
    pub params: BoundedVec<TypedParam, 32, NoStdProvider<65536>>,
    pub results: BoundedVec<TypedResult, 16, NoStdProvider<65536>>,
    pub component_type_id: Option<TypeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsyncInterfaceFunction {
    pub name: BoundedString<64, NoStdProvider<65536>>,
    pub params: BoundedVec<TypedParam, 32, NoStdProvider<65536>>,
    pub results: BoundedVec<AsyncTypedResult, 16, NoStdProvider<65536>>,
    pub component_type_id: Option<TypeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedParam {
    pub name: BoundedString<32, NoStdProvider<65536>>,
    pub val_type: ValType,
    pub wit_type: WitType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedResult {
    pub name: Option<BoundedString<32, NoStdProvider<65536>>>,
    pub val_type: ValType,
    pub wit_type: WitType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsyncTypedResult {
    pub name: Option<BoundedString<32, NoStdProvider<65536>>>,
    pub val_type: ValType,
    pub wit_type: WitType,
    pub is_stream: bool,
    pub is_future: bool,
}

impl WitComponentBuilder {
    pub fn new() -> Self {
        Self {
            parser: WitParser::new(),
            type_registry: GenerativeTypeRegistry::new(),
            wit_type_mappings: BTreeMap::new(),
        }
    }

    pub fn parse_world_from_source(
        &mut self,
        source: &str,
        instance_id: ComponentInstanceId,
    ) -> core::result::Result<ComponentInterface, ComponentError> {
        let wit_world = self.parser.parse_world(source).map_err(|e| self.convert_parse_error(e))?;

        self.convert_world_to_interface(wit_world, instance_id)
    }

    pub fn parse_interface_from_source(
        &mut self,
        source: &str,
        instance_id: ComponentInstanceId,
    ) -> core::result::Result<ComponentInterface, ComponentError> {
        let wit_interface =
            self.parser.parse_interface(source).map_err(|e| self.convert_parse_error(e))?;

        self.convert_interface_to_component(wit_interface, instance_id)
    }

    pub fn register_wit_type(
        &mut self,
        wit_type_name: &str,
        component_type_id: TypeId,
    ) -> core::result::Result<(), ComponentError> {
        let name =
            BoundedString::from_str(wit_type_name).map_err(|_| ComponentError::TypeMismatch)?;

        self.wit_type_mappings.insert(name, component_type_id);
        Ok(())
    }

    pub fn create_generative_type_from_wit(
        &mut self,
        wit_type: &WitType,
        instance_id: ComponentInstanceId,
    ) -> core::result::Result<GenerativeResourceType, ComponentError> {
        let val_type = self.parser.convert_to_valtype(wit_type)?;

        let base_resource_type = wrt_foundation::resource::ResourceType::Handle(
            wrt_foundation::resource::ResourceHandle::new(0),
        );

        self.type_registry.create_generative_type(base_resource_type, instance_id)
    }

    pub fn add_type_constraint(
        &mut self,
        type1: TypeId,
        type2: TypeId,
        constraint: BoundKind,
    ) -> core::result::Result<(), ComponentError> {
        let bound = TypeBound { type_id: type1, bound_kind: constraint, target_type: type2 };

        self.type_registry.add_type_bound(type1, bound)
    }

    fn convert_world_to_interface(
        &mut self,
        world: WitWorld,
        instance_id: ComponentInstanceId,
    ) -> core::result::Result<ComponentInterface, ComponentError> {
        let mut interface = ComponentInterface {
            name: world.name,
            imports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            exports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            async_imports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            async_exports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        };

        for import in world.imports.iter() {
            match &import.item {
                crate::wit_parser::WitItem::Function(func) => {
                    if func.is_async {
                        let async_func =
                            self.convert_to_async_interface_function(func, instance_id)?;
                        interface
                            .async_imports
                            .push(async_func)
                            .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
                    } else {
                        let interface_func =
                            self.convert_to_interface_function(func, instance_id)?;
                        interface
                            .imports
                            .push(interface_func)
                            .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
                    }
                }
                _ => {} // Handle other item types as needed
            }
        }

        for export in world.exports.iter() {
            match &export.item {
                crate::wit_parser::WitItem::Function(func) => {
                    if func.is_async {
                        let async_func =
                            self.convert_to_async_interface_function(func, instance_id)?;
                        interface
                            .async_exports
                            .push(async_func)
                            .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
                    } else {
                        let interface_func =
                            self.convert_to_interface_function(func, instance_id)?;
                        interface
                            .exports
                            .push(interface_func)
                            .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
                    }
                }
                _ => {} // Handle other item types as needed
            }
        }

        Ok(interface)
    }

    fn convert_interface_to_component(
        &mut self,
        wit_interface: WitInterface,
        instance_id: ComponentInstanceId,
    ) -> core::result::Result<ComponentInterface, ComponentError> {
        let mut interface = ComponentInterface {
            name: wit_interface.name,
            imports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            exports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            async_imports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            async_exports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        };

        for func in wit_interface.functions.iter() {
            if func.is_async {
                let async_func = self.convert_to_async_interface_function(func, instance_id)?;
                interface
                    .async_exports
                    .push(async_func)
                    .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
            } else {
                let interface_func = self.convert_to_interface_function(func, instance_id)?;
                interface
                    .exports
                    .push(interface_func)
                    .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
            }
        }

        Ok(interface)
    }

    fn convert_to_interface_function(
        &mut self,
        wit_func: &WitFunction,
        instance_id: ComponentInstanceId,
    ) -> core::result::Result<InterfaceFunction, ComponentError> {
        let mut interface_func = InterfaceFunction {
            name: wit_func.name.clone(),
            params: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            results: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            component_type_id: None,
        };

        for param in wit_func.params.iter() {
            let val_type = self.parser.convert_to_valtype(&param.ty)?;
            let typed_param =
                TypedParam { name: param.name.clone(), val_type, wit_type: param.ty.clone() };
            interface_func
                .params
                .push(typed_param)
                .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
        }

        for result in wit_func.results.iter() {
            let val_type = self.parser.convert_to_valtype(&result.ty)?;
            let typed_result =
                TypedResult { name: result.name.clone(), val_type, wit_type: result.ty.clone() };
            interface_func
                .results
                .push(typed_result)
                .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
        }

        Ok(interface_func)
    }

    fn convert_to_async_interface_function(
        &mut self,
        wit_func: &WitFunction,
        instance_id: ComponentInstanceId,
    ) -> core::result::Result<AsyncInterfaceFunction, ComponentError> {
        let mut async_func = AsyncInterfaceFunction {
            name: wit_func.name.clone(),
            params: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            results: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            component_type_id: None,
        };

        for param in wit_func.params.iter() {
            let val_type = self.parser.convert_to_valtype(&param.ty)?;
            let typed_param =
                TypedParam { name: param.name.clone(), val_type, wit_type: param.ty.clone() };
            async_func
                .params
                .push(typed_param)
                .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
        }

        for result in wit_func.results.iter() {
            let val_type = self.parser.convert_to_valtype(&result.ty)?;
            let is_stream = matches!(result.ty, WitType::Stream(_));
            let is_future = matches!(result.ty, WitType::Future(_));

            let async_result = AsyncTypedResult {
                name: result.name.clone(),
                val_type,
                wit_type: result.ty.clone(),
                is_stream,
                is_future,
            };
            async_func
                .results
                .push(async_result)
                .map_err(|_| ComponentError::TooManyGenerativeTypes)?;
        }

        Ok(async_func)
    }

    fn convert_parse_error(&self, error: WitParseError) -> ComponentError {
        match error {
            WitParseError::UnknownType(_) => ComponentError::TypeMismatch,
            WitParseError::TooManyItems => ComponentError::TooManyGenerativeTypes,
            WitParseError::InvalidIdentifier(_) => ComponentError::TypeMismatch,
            WitParseError::DuplicateDefinition(_) => ComponentError::TooManyGenerativeTypes,
            _ => ComponentError::InstantiationFailed,
        }
    }

    pub fn get_type_registry(&self) -> &GenerativeTypeRegistry {
        &self.type_registry
    }

    pub fn get_type_registry_mut(&mut self) -> &mut GenerativeTypeRegistry {
        &mut self.type_registry
    }
}

impl Default for WitComponentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wit_component_builder_creation() {
        let builder = WitComponentBuilder::new();
        assert_eq!(builder.wit_type_mappings.len(), 0);
    }

    #[test]
    fn test_register_wit_type() {
        let mut builder = WitComponentBuilder::new();
        let type_id = TypeId(1);

        assert!(builder.register_wit_type("my-type", type_id).is_ok());
        assert!(builder
            .wit_type_mappings
            .contains_key(&BoundedString::from_str("my-type").unwrap()));
    }

    #[test]
    fn test_parse_simple_world() {
        let mut builder = WitComponentBuilder::new();
        let instance_id = ComponentInstanceId(1);

        let source = r#"
            world test-world {
                import test-func: func() -> u32
                export result-func: func(x: u32) -> string
            }
        "#;

        let result = builder.parse_world_from_source(source, instance_id);
        assert!(result.is_ok());

        let interface = result.unwrap();
        assert_eq!(interface.name.as_str(), "test-world");
        assert_eq!(interface.imports.len(), 1);
        assert_eq!(interface.exports.len(), 1);
    }

    #[test]
    fn test_parse_async_interface() {
        let mut builder = WitComponentBuilder::new();
        let instance_id = ComponentInstanceId(1);

        let source = r#"
            interface async-test {
                async-stream: async func() -> stream<u8>
                async-future: async func(x: u32) -> future<string>
            }
        "#;

        let result = builder.parse_interface_from_source(source, instance_id);
        assert!(result.is_ok());

        let interface = result.unwrap();
        assert_eq!(interface.name.as_str(), "async-test");
        assert_eq!(interface.async_exports.len(), 2);

        let stream_func = &interface.async_exports[0];
        assert!(stream_func.results[0].is_stream);

        let future_func = &interface.async_exports[1];
        assert!(future_func.results[0].is_future);
    }

    #[test]
    fn test_type_constraint_integration() {
        let mut builder = WitComponentBuilder::new();
        let type1 = TypeId(1);
        let type2 = TypeId(2);

        assert!(builder.add_type_constraint(type1, type2, BoundKind::Sub).is_ok());

        let result = builder.type_registry.check_type_bound_simple(type1, type2, BoundKind::Sub);
        assert!(result);
    }
}
