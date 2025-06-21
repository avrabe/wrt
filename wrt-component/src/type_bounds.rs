#[cfg(feature = "std")]
use std::collections::BTreeMap;

#[cfg(not(feature = "std"))]
use wrt_foundation::{BoundedMap as BTreeMap, BoundedVec as Vec, safe_memory::NoStdProvider};

// Type aliases for no_std compatibility
#[cfg(not(feature = "std"))]
type TypeBoundsMap<K, V> = BTreeMap<K, V, 64, NoStdProvider<65536>>;

use core::fmt;

use wrt_foundation::{
    bounded::{BoundedVec, MAX_GENERATIVE_TYPES},
};

#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;

#[cfg(not(feature = "std"))]
// For no_std, use a simpler ComponentValue representation
use crate::types::Value as ComponentValue;

use crate::{
    generative_types::{BoundKind, TypeBound},
    types::{ComponentError, TypeId, ValType},
};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeBoundsChecker {
    #[cfg(feature = "std")]
    type_hierarchy: BTreeMap<TypeId, BoundedVec<TypeRelation, MAX_GENERATIVE_TYPES, NoStdProvider<65536>>>,
    #[cfg(not(feature = "std"))]
    type_hierarchy: BTreeMap<TypeId, BoundedVec<TypeRelation, MAX_GENERATIVE_TYPES, NoStdProvider<65536>>, 32, NoStdProvider<65536>>,
    cached_relations: BTreeMap<(TypeId, TypeId), RelationResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeRelation {
    pub sub_type: TypeId,
    pub super_type: TypeId,
    pub relation_kind: RelationKind,
    pub confidence: RelationConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationKind {
    /// Types are equal
    Eq,
    /// sub_type is a subtype of super_type
    Sub,
    /// Types are unrelated
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationConfidence {
    /// Relation is definitively known
    Definite,
    /// Relation is inferred from other relations
    Inferred,
    /// Relation is assumed but not verified
    Assumed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationResult {
    /// Types satisfy the bound
    Satisfied,
    /// Types do not satisfy the bound
    Violated,
    /// Relationship is unknown/undecidable
    Unknown,
}

impl TypeBoundsChecker {
    pub fn new() -> Self {
        Self { type_hierarchy: BTreeMap::new(), cached_relations: BTreeMap::new() }
    }

    pub fn add_type_bound(&mut self, bound: TypeBound) -> core::result::Result<(), ComponentError> {
        let relation = TypeRelation {
            sub_type: bound.type_id,
            super_type: bound.target_type,
            relation_kind: match bound.bound_kind {
                BoundKind::Eq => RelationKind::Eq,
                BoundKind::Sub => RelationKind::Sub,
            },
            confidence: RelationConfidence::Definite,
        };

        self.add_relation(relation)?;
        self.invalidate_cache();
        Ok(())
    }

    pub fn check_type_bound(
        &mut self,
        type1: TypeId,
        type2: TypeId,
        bound_kind: BoundKind,
    ) -> RelationResult {
        let cache_key = (type1, type2);

        if let Some(cached) = self.cached_relations.get(&cache_key) {
            return cached.clone();
        }

        let result = match bound_kind {
            BoundKind::Eq => self.check_equality(type1, type2),
            BoundKind::Sub => self.check_subtype(type1, type2),
        };

        self.cached_relations.insert(cache_key, result.clone());
        result
    }

    pub fn check_equality(&self, type1: TypeId, type2: TypeId) -> RelationResult {
        if type1 == type2 {
            return RelationResult::Satisfied;
        }

        if let Some(relations) = self.type_hierarchy.get(&type1) {
            for relation in relations.iter() {
                if relation.super_type == type2 && relation.relation_kind == RelationKind::Eq {
                    return RelationResult::Satisfied;
                }
            }
        }

        if let Some(relations) = self.type_hierarchy.get(&type2) {
            for relation in relations.iter() {
                if relation.super_type == type1 && relation.relation_kind == RelationKind::Eq {
                    return RelationResult::Satisfied;
                }
            }
        }

        RelationResult::Violated
    }

    pub fn check_subtype(&self, sub_type: TypeId, super_type: TypeId) -> RelationResult {
        if sub_type == super_type {
            return RelationResult::Satisfied;
        }

        if let Some(relations) = self.type_hierarchy.get(&sub_type) {
            for relation in relations.iter() {
                match relation.relation_kind {
                    RelationKind::Sub | RelationKind::Eq => {
                        if relation.super_type == super_type {
                            return RelationResult::Satisfied;
                        }

                        let transitive_result = self.check_subtype(relation.super_type, super_type);
                        if transitive_result == RelationResult::Satisfied {
                            return RelationResult::Satisfied;
                        }
                    }
                    RelationKind::None => {}
                }
            }
        }

        RelationResult::Violated
    }

    pub fn infer_relations(&mut self) -> core::result::Result<usize, ComponentError> {
        let mut inferred_count = 0;
        let max_iterations = 10;

        for _ in 0..max_iterations {
            let mut new_relations = Vec::new();

            for (type_id, relations) in &self.type_hierarchy {
                for relation in relations.iter() {
                    if let Some(super_relations) = self.type_hierarchy.get(&relation.super_type) {
                        for super_relation in super_relations.iter() {
                            let new_relation = TypeRelation {
                                sub_type: *type_id,
                                super_type: super_relation.super_type,
                                relation_kind: self.combine_relations(
                                    &relation.relation_kind,
                                    &super_relation.relation_kind,
                                ),
                                confidence: RelationConfidence::Inferred,
                            };

                            if !self.relation_exists(&new_relation) {
                                new_relations.push(new_relation);
                            }
                        }
                    }
                }
            }

            if new_relations.is_empty() {
                break;
            }

            for relation in new_relations {
                self.add_relation(relation)?;
                inferred_count += 1;
            }
        }

        self.invalidate_cache();
        Ok(inferred_count)
    }

    pub fn validate_consistency(&self) -> core::result::Result<(), ComponentError> {
        for (type_id, relations) in &self.type_hierarchy {
            for relation in relations.iter() {
                if *type_id == relation.super_type && relation.relation_kind == RelationKind::Sub {
                    return Err(ComponentError::InvalidSubtypeRelation(
                        *type_id,
                        relation.super_type,
                    ));
                }

                if self.creates_cycle(*type_id, relation.super_type) {
                    return Err(ComponentError::InvalidSubtypeRelation(
                        *type_id,
                        relation.super_type,
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn get_all_supertypes(&self, type_id: TypeId) -> Vec<TypeId> {
        let mut supertypes = Vec::new();
        self.collect_supertypes(type_id, &mut supertypes);
        supertypes
    }

    pub fn get_all_subtypes(&self, type_id: TypeId) -> Vec<TypeId> {
        let mut subtypes = Vec::new();

        for (sub_type_id, relations) in &self.type_hierarchy {
            for relation in relations.iter() {
                if relation.super_type == type_id
                    && (relation.relation_kind == RelationKind::Sub
                        || relation.relation_kind == RelationKind::Eq)
                {
                    subtypes.push(*sub_type_id);
                }
            }
        }

        subtypes
    }

    fn add_relation(&mut self, relation: TypeRelation) -> core::result::Result<(), ComponentError> {
        // Check if the key exists, if not insert a new BoundedVec
        if !self.type_hierarchy.get(&relation.sub_type).is_ok() || self.type_hierarchy.get(&relation.sub_type).unwrap().is_none() {
            let new_vec = BoundedVec::new(NoStdProvider::<65536>::default()).unwrap();
            self.type_hierarchy.insert(relation.sub_type, new_vec).map_err(|_| ComponentError::TooManyTypeBounds)?;
        }
        
        // Now get the existing vector and add to it
        let relations = self.type_hierarchy.get(&relation.sub_type).unwrap().unwrap();
        // Note: BoundedMap doesn't support mutable access, so we need to remove, modify, and re-insert
        let mut updated_relations = relations.clone();
        updated_relations.push(relation).map_err(|_| ComponentError::TooManyTypeBounds)?;
        self.type_hierarchy.insert(relation.sub_type, updated_relations).map_err(|_| ComponentError::TooManyTypeBounds)?;

        Ok(())
    }

    fn relation_exists(&self, relation: &TypeRelation) -> bool {
        if let Some(relations) = self.type_hierarchy.get(&relation.sub_type) {
            relations.iter().any(|r| {
                r.super_type == relation.super_type && r.relation_kind == relation.relation_kind
            })
        } else {
            false
        }
    }

    fn combine_relations(&self, rel1: &RelationKind, rel2: &RelationKind) -> RelationKind {
        match (rel1, rel2) {
            (RelationKind::Eq, RelationKind::Eq) => RelationKind::Eq,
            (RelationKind::Eq, RelationKind::Sub) | (RelationKind::Sub, RelationKind::Eq) => {
                RelationKind::Sub
            }
            (RelationKind::Sub, RelationKind::Sub) => RelationKind::Sub,
            _ => RelationKind::None,
        }
    }

    fn creates_cycle(&self, start: TypeId, target: TypeId) -> bool {
        self.creates_cycle_helper(start, target, &mut Vec::new())
    }

    fn creates_cycle_helper(
        &self,
        current: TypeId,
        target: TypeId,
        visited: &mut Vec<TypeId>,
    ) -> bool {
        if visited.contains(&current) {
            return current == target;
        }

        visited.push(current);

        if let Some(relations) = self.type_hierarchy.get(&target) {
            for relation in relations.iter() {
                if relation.super_type == current {
                    return true;
                }

                if self.creates_cycle_helper(current, relation.super_type, visited) {
                    return true;
                }
            }
        }

        visited.pop();
        false
    }

    fn collect_supertypes(&self, type_id: TypeId, supertypes: &mut Vec<TypeId>) {
        if let Some(relations) = self.type_hierarchy.get(&type_id) {
            for relation in relations.iter() {
                if !supertypes.contains(&relation.super_type) {
                    supertypes.push(relation.super_type);
                    self.collect_supertypes(relation.super_type, supertypes);
                }
            }
        }
    }

    fn invalidate_cache(&mut self) {
        self.cached_relations.clear();
    }
}

impl Default for TypeBoundsChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RelationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationResult::Satisfied => write!(f, "satisfied"),
            RelationResult::Violated => write!(f, "violated"),
            RelationResult::Unknown => write!(f, "unknown"),
        }
    }
}

impl fmt::Display for RelationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationKind::Eq => write!(f, "="),
            RelationKind::Sub => write!(f, "<:"),
            RelationKind::None => write!(f, "none"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generative_types::BoundKind;

    #[test]
    fn test_type_bounds_checker_creation() {
        let checker = TypeBoundsChecker::new();
        assert_eq!(checker.type_hierarchy.len(), 0);
        assert_eq!(checker.cached_relations.len(), 0);
    }

    #[test]
    fn test_equality_bound() {
        let mut checker = TypeBoundsChecker::new();
        let type1 = TypeId(1);
        let type2 = TypeId(2);

        let bound = TypeBound { type_id: type1, bound_kind: BoundKind::Eq, target_type: type2 };

        assert!(checker.add_type_bound(bound).is_ok());

        let result = checker.check_type_bound(type1, type2, BoundKind::Eq);
        assert_eq!(result, RelationResult::Satisfied);

        let reverse_result = checker.check_type_bound(type2, type1, BoundKind::Eq);
        assert_eq!(reverse_result, RelationResult::Satisfied);
    }

    #[test]
    fn test_subtype_bound() {
        let mut checker = TypeBoundsChecker::new();
        let sub_type = TypeId(1);
        let super_type = TypeId(2);

        let bound =
            TypeBound { type_id: sub_type, bound_kind: BoundKind::Sub, target_type: super_type };

        assert!(checker.add_type_bound(bound).is_ok());

        let result = checker.check_type_bound(sub_type, super_type, BoundKind::Sub);
        assert_eq!(result, RelationResult::Satisfied);

        let reverse_result = checker.check_type_bound(super_type, sub_type, BoundKind::Sub);
        assert_eq!(result, RelationResult::Satisfied);
    }

    #[test]
    fn test_transitive_subtyping() {
        let mut checker = TypeBoundsChecker::new();
        let type_a = TypeId(1);
        let type_b = TypeId(2);
        let type_c = TypeId(3);

        let bound1 = TypeBound { type_id: type_a, bound_kind: BoundKind::Sub, target_type: type_b };
        let bound2 = TypeBound { type_id: type_b, bound_kind: BoundKind::Sub, target_type: type_c };

        assert!(checker.add_type_bound(bound1).is_ok());
        assert!(checker.add_type_bound(bound2).is_ok());

        let result = checker.check_type_bound(type_a, type_c, BoundKind::Sub);
        assert_eq!(result, RelationResult::Satisfied);
    }

    #[test]
    fn test_relation_inference() {
        let mut checker = TypeBoundsChecker::new();
        let type_a = TypeId(1);
        let type_b = TypeId(2);
        let type_c = TypeId(3);

        let bound1 = TypeBound { type_id: type_a, bound_kind: BoundKind::Sub, target_type: type_b };
        let bound2 = TypeBound { type_id: type_b, bound_kind: BoundKind::Sub, target_type: type_c };

        assert!(checker.add_type_bound(bound1).is_ok());
        assert!(checker.add_type_bound(bound2).is_ok());

        let inferred = checker.infer_relations().unwrap();
        assert!(inferred > 0);

        let result = checker.check_type_bound(type_a, type_c, BoundKind::Sub);
        assert_eq!(result, RelationResult::Satisfied);
    }

    #[test]
    fn test_consistency_validation() {
        let mut checker = TypeBoundsChecker::new();
        let type1 = TypeId(1);

        let bound = TypeBound { type_id: type1, bound_kind: BoundKind::Sub, target_type: type1 };

        assert!(checker.add_type_bound(bound).is_ok());
        assert!(checker.validate_consistency().is_err());
    }

    #[test]
    fn test_supertypes_and_subtypes() {
        let mut checker = TypeBoundsChecker::new();
        let type_a = TypeId(1);
        let type_b = TypeId(2);
        let type_c = TypeId(3);

        let bound1 = TypeBound { type_id: type_a, bound_kind: BoundKind::Sub, target_type: type_b };
        let bound2 = TypeBound { type_id: type_b, bound_kind: BoundKind::Sub, target_type: type_c };

        assert!(checker.add_type_bound(bound1).is_ok());
        assert!(checker.add_type_bound(bound2).is_ok());

        let supertypes = checker.get_all_supertypes(type_a);
        assert!(supertypes.contains(&type_b));
        assert!(supertypes.contains(&type_c));

        let subtypes = checker.get_all_subtypes(type_c);
        assert!(subtypes.contains(&type_b));
        assert!(subtypes.contains(&type_a));
    }
}
