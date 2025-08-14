#[cfg(not(feature = "std"))]
use alloc::{
    collections::BTreeMap,
    vec::Vec,
};
use core::sync::atomic::{
    AtomicU32,
    Ordering,
};
#[cfg(feature = "std")]
use std::{
    collections::BTreeMap,
    vec::Vec,
};

use wrt_foundation::{
    bounded::{
        BoundedVec,
        MAX_GENERATIVE_TYPES,
    },
    budget_aware_provider::CrateId,
    component_value::ComponentValue,
    resource::ResourceType,
    safe_managed_alloc,
};

use crate::{
    bounded_component_infra::ComponentProvider,
    resource_management::ResourceHandle,
    type_bounds::{
        RelationResult,
        TypeBoundsChecker,
    },
    types::{
        ComponentError,
        ComponentInstanceId,
        ResourceId,
        TypeId,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct GenerativeResourceType {
    pub base_type:      ResourceType<ComponentProvider>,
    pub instance_id:    ComponentInstanceId,
    pub unique_type_id: TypeId,
    pub generation:     u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeBound {
    pub type_id:     TypeId,
    pub bound_kind:  BoundKind,
    pub target_type: TypeId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoundKind {
    Eq,
    Sub,
}

pub struct GenerativeTypeRegistry {
    next_type_id:      AtomicU32,
    instance_types:
        BTreeMap<ComponentInstanceId, BoundedVec<GenerativeResourceType, MAX_GENERATIVE_TYPES>>,
    type_bounds:       BTreeMap<TypeId, BoundedVec<TypeBound, MAX_GENERATIVE_TYPES>>,
    resource_mappings: BTreeMap<ResourceHandle, GenerativeResourceType>,
    bounds_checker:    TypeBoundsChecker,
}

impl GenerativeTypeRegistry {
    pub fn new() -> Self {
        Self {
            next_type_id:      AtomicU32::new(1),
            instance_types:    BTreeMap::new(),
            type_bounds:       BTreeMap::new(),
            resource_mappings: BTreeMap::new(),
            bounds_checker:    TypeBoundsChecker::new(),
        }
    }

    pub fn create_generative_type(
        &mut self,
        base_type: ResourceType<ComponentProvider>,
        instance_id: ComponentInstanceId,
    ) -> core::result::Result<GenerativeResourceType, ComponentError> {
        let unique_type_id = TypeId(self.next_type_id.fetch_add(1, Ordering::SeqCst));

        let generative_type = GenerativeResourceType {
            base_type,
            instance_id,
            unique_type_id,
            generation: 0,
        };

        let instance_types = self.instance_types.entry(instance_id).or_insert_with(|| {
            let provider = safe_managed_alloc!(65536, CrateId::Component)
                .expect("Failed to allocate memory for instance types");
            BoundedVec::new(provider).expect("Failed to create BoundedVec")
        });

        instance_types
            .push(generative_type.clone())
            .map_err(|_| ComponentError::TooManyGenerativeTypes)?;

        Ok(generative_type)
    }

    pub fn get_generative_type(
        &self,
        type_id: TypeId,
        instance_id: ComponentInstanceId,
    ) -> Option<&GenerativeResourceType> {
        self.instance_types
            .get(&instance_id)?
            .iter()
            .find(|t| t.unique_type_id == type_id)
    }

    pub fn add_type_bound(
        &mut self,
        type_id: TypeId,
        bound: TypeBound,
    ) -> core::result::Result<(), ComponentError> {
        let bounds = self.type_bounds.entry(type_id).or_insert_with(|| {
            let provider = safe_managed_alloc!(65536, CrateId::Component)
                .expect("Failed to allocate memory for type bounds");
            BoundedVec::new(provider).expect("Failed to create BoundedVec")
        });

        bounds.push(bound.clone()).map_err(|_| ComponentError::TooManyTypeBounds)?;

        self.bounds_checker.add_type_bound(bound)?;

        Ok(())
    }

    pub fn check_type_bound(
        &mut self,
        type_id: TypeId,
        target_type: TypeId,
        bound_kind: BoundKind,
    ) -> RelationResult {
        self.bounds_checker.check_type_bound(type_id, target_type, bound_kind)
    }

    pub fn check_type_bound_simple(
        &self,
        type_id: TypeId,
        target_type: TypeId,
        bound_kind: BoundKind,
    ) -> bool {
        if let Some(bounds) = self.type_bounds.get(&type_id) {
            bounds
                .iter()
                .any(|bound| bound.target_type == target_type && bound.bound_kind == bound_kind)
        } else {
            false
        }
    }

    pub fn register_resource_handle(
        &mut self,
        handle: ResourceHandle,
        generative_type: GenerativeResourceType,
    ) -> core::result::Result<(), ComponentError> {
        if self.resource_mappings.contains_key(&handle) {
            return Err(ComponentError::ResourceHandleAlreadyExists);
        }

        self.resource_mappings.insert(handle, generative_type);
        Ok(())
    }

    pub fn get_resource_type(&self, handle: ResourceHandle) -> Option<&GenerativeResourceType> {
        self.resource_mappings.get(&handle)
    }

    pub fn is_same_instance_type(
        &self,
        type1: TypeId,
        type2: TypeId,
        instance_id: ComponentInstanceId,
    ) -> bool {
        if let Some(instance_types) = self.instance_types.get(&instance_id) {
            let t1 = instance_types.iter().find(|t| t.unique_type_id == type1);
            let t2 = instance_types.iter().find(|t| t.unique_type_id == type2);

            match (t1, t2) {
                (Some(type1), Some(type2)) => type1.base_type == type2.base_type,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn get_instance_types(&self, instance_id: ComponentInstanceId) -> Vec<TypeId> {
        if let Some(types) = self.instance_types.get(&instance_id) {
            types.iter().map(|t| t.unique_type_id).collect()
        } else {
            Vec::new()
        }
    }

    pub fn cleanup_instance(&mut self, instance_id: ComponentInstanceId) {
        if let Some(types) = self.instance_types.remove(&instance_id) {
            for generative_type in types.iter() {
                self.type_bounds.remove(&generative_type.unique_type_id);

                self.resource_mappings
                    .retain(|_, mapped_type| mapped_type.instance_id != instance_id);
            }
        }
    }

    pub fn infer_type_relations(&mut self) -> core::result::Result<usize, ComponentError> {
        self.bounds_checker.infer_relations()
    }

    pub fn validate_type_consistency(&self) -> core::result::Result<(), ComponentError> {
        self.bounds_checker.validate_consistency()
    }

    pub fn get_all_supertypes(&self, type_id: TypeId) -> Vec<TypeId> {
        self.bounds_checker.get_all_supertypes(type_id)
    }

    pub fn get_all_subtypes(&self, type_id: TypeId) -> Vec<TypeId> {
        self.bounds_checker.get_all_subtypes(type_id)
    }

    #[cfg(feature = "std")]
    pub fn validate_type_system(&mut self) -> core::result::Result<(), ComponentError> {
        self.infer_type_relations()?;
        self.validate_type_consistency()?;

        for (type_id, bounds) in &self.type_bounds {
            for bound in bounds.iter() {
                if !self.is_valid_type_reference(bound.target_type) {
                    return Err(ComponentError::InvalidTypeReference(
                        *type_id,
                        bound.target_type,
                    ));
                }

                if let BoundKind::Sub = bound.bound_kind {
                    let result = self.bounds_checker.check_subtype(*type_id, bound.target_type);
                    if result != RelationResult::Satisfied {
                        return Err(ComponentError::InvalidSubtypeRelation(
                            *type_id,
                            bound.target_type,
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn is_valid_type_reference(&self, type_id: TypeId) -> bool {
        self.instance_types
            .values()
            .any(|types| types.iter().any(|t| t.unique_type_id == type_id))
    }

    fn check_subtype_relation(&self, sub_type: TypeId, super_type: TypeId) -> bool {
        for instance_types in self.instance_types.values() {
            let sub = instance_types.iter().find(|t| t.unique_type_id == sub_type);
            let sup = instance_types.iter().find(|t| t.unique_type_id == super_type);

            if let (Some(sub_t), Some(sup_t)) = (sub, sup) {
                return self.is_resource_subtype(&sub_t.base_type, &sup_t.base_type);
            }
        }
        false
    }

    fn is_resource_subtype(
        &self,
        sub_type: &ResourceType<ComponentProvider>,
        super_type: &ResourceType<ComponentProvider>,
    ) -> bool {
        match (sub_type, super_type) {
            (
                ResourceType::<ComponentProvider>::Handle(sub_h),
                ResourceType::<ComponentProvider>::Handle(super_h),
            ) => sub_h.type_name() == super_h.type_name(),
            _ => false,
        }
    }
}

impl Default for GenerativeTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use wrt_foundation::resource::ResourceHandle;

    use super::*;

    #[test]
    fn test_generative_type_registry_creation() {
        let mut registry = GenerativeTypeRegistry::new();
        let base_type = ResourceType::<ComponentProvider>::Handle(ResourceHandle::new(42));
        let instance_id = ComponentInstanceId(1);

        let result = registry.create_generative_type(base_type.clone(), instance_id);
        assert!(result.is_ok());

        let gen_type = result.unwrap();
        assert_eq!(gen_type.base_type, base_type);
        assert_eq!(gen_type.instance_id, instance_id);
        assert_eq!(gen_type.generation, 0);
        assert_eq!(gen_type.unique_type_id, TypeId(1));
    }

    #[test]
    fn test_unique_type_ids_across_instances() {
        let mut registry = GenerativeTypeRegistry::new();
        let base_type = ResourceType::<ComponentProvider>::Handle(ResourceHandle::new(42));
        let instance1 = ComponentInstanceId(1);
        let instance2 = ComponentInstanceId(2);

        let type1 = registry.create_generative_type(base_type.clone(), instance1).unwrap();
        let type2 = registry.create_generative_type(base_type, instance2).unwrap();

        assert_ne!(type1.unique_type_id, type2.unique_type_id);
    }

    #[test]
    fn test_type_bounds() {
        let mut registry = GenerativeTypeRegistry::new();
        let type_id = TypeId(1);
        let target_type = TypeId(2);

        let bound = TypeBound {
            type_id,
            bound_kind: BoundKind::Eq,
            target_type,
        };

        assert!(registry.add_type_bound(type_id, bound).is_ok());
        assert!(registry.check_type_bound_simple(type_id, target_type, BoundKind::Eq));
        assert!(!registry.check_type_bound_simple(type_id, target_type, BoundKind::Sub));

        let result = registry.check_type_bound(type_id, target_type, BoundKind::Eq);
        assert_eq!(result, RelationResult::Satisfied);
    }

    #[test]
    fn test_resource_handle_registration() {
        let mut registry = GenerativeTypeRegistry::new();
        let base_type = ResourceType::<ComponentProvider>::Handle(ResourceHandle::new(42));
        let instance_id = ComponentInstanceId(1);
        let handle = ResourceHandle::new(100);

        let gen_type = registry.create_generative_type(base_type, instance_id).unwrap();

        assert!(registry.register_resource_handle(handle, gen_type.clone()).is_ok());
        assert_eq!(registry.get_resource_type(handle), Some(&gen_type));
    }

    #[test]
    fn test_instance_cleanup() {
        let mut registry = GenerativeTypeRegistry::new();
        let base_type = ResourceType::<ComponentProvider>::Handle(ResourceHandle::new(42));
        let instance_id = ComponentInstanceId(1);

        let gen_type = registry.create_generative_type(base_type, instance_id).unwrap();
        assert!(registry.get_generative_type(gen_type.unique_type_id, instance_id).is_some());

        registry.cleanup_instance(instance_id);
        assert!(registry.get_generative_type(gen_type.unique_type_id, instance_id).is_none());
    }

    #[test]
    fn test_transitive_type_bounds() {
        let mut registry = GenerativeTypeRegistry::new();
        let type_a = TypeId(1);
        let type_b = TypeId(2);
        let type_c = TypeId(3);

        let bound1 = TypeBound {
            type_id:     type_a,
            bound_kind:  BoundKind::Sub,
            target_type: type_b,
        };
        let bound2 = TypeBound {
            type_id:     type_b,
            bound_kind:  BoundKind::Sub,
            target_type: type_c,
        };

        assert!(registry.add_type_bound(type_a, bound1).is_ok());
        assert!(registry.add_type_bound(type_b, bound2).is_ok());

        assert!(registry.infer_type_relations().is_ok());

        let result = registry.check_type_bound(type_a, type_c, BoundKind::Sub);
        assert_eq!(result, RelationResult::Satisfied);

        let supertypes = registry.get_all_supertypes(type_a);
        assert!(supertypes.contains(&type_b));
        assert!(supertypes.contains(&type_c));
    }

    #[test]
    fn test_type_consistency_validation() {
        let mut registry = GenerativeTypeRegistry::new();
        assert!(registry.validate_type_consistency().is_ok());

        let type_a = TypeId(1);
        let bound = TypeBound {
            type_id:     type_a,
            bound_kind:  BoundKind::Sub,
            target_type: type_a,
        };

        assert!(registry.add_type_bound(type_a, bound).is_ok());
        assert!(registry.validate_type_consistency().is_err());
    }
}
