// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

// Note: ResourceOperation not currently available in wrt_format::component
// use wrt_format::component::ResourceOperation as FormatResourceOperation;
use wrt_foundation::resource::ResourceOperation;

// Convert from local ResourceOperation enum to format ResourceOperation
// Temporarily disabled due to missing FormatResourceOperation type
// pub fn to_format_resource_operation(
// op: ResourceOperation,
// type_idx: u32,
// ) -> FormatResourceOperation {
// use wrt_format::component::ResourceOperation as FormatOp;
// use wrt_foundation::resource::{ResourceDrop, ResourceNew, ResourceRep};
//
// match op {
// ResourceOperation::Read => FormatOp::Rep(ResourceRep { type_idx }),
// ResourceOperation::Write => FormatOp::Transfer,
// ResourceOperation::Execute => FormatOp::Execute,
// ResourceOperation::Create => FormatOp::New(ResourceNew { type_idx }),
// ResourceOperation::Delete => FormatOp::Drop(ResourceDrop { type_idx }),
// ResourceOperation::Reference => FormatOp::Borrow,
// ResourceOperation::Dereference => FormatOp::Dereference,
// }
// }

// Convert from format ResourceOperation to local ResourceOperation
// Temporarily disabled due to missing FormatResourceOperation type
// pub fn from_format_resource_operation(op: &FormatResourceOperation) ->
// ResourceOperation { use wrt_format::component::ResourceOperation as FormatOp;
//
// match op {
// FormatOp::Rep(_) => ResourceOperation::Read,
// FormatOp::Transfer => ResourceOperation::Write,
// FormatOp::Execute => ResourceOperation::Execute,
// FormatOp::New(_) => ResourceOperation::Create,
// FormatOp::Drop(_) => ResourceOperation::Delete,
// FormatOp::Destroy(_) => ResourceOperation::Delete,
// FormatOp::Borrow => ResourceOperation::Reference,
// FormatOp::Dereference => ResourceOperation::Dereference,
// _ => ResourceOperation::Read, // Default to read for unknown operations
// }
// }

// Tests temporarily disabled due to missing FormatResourceOperation type
// #[cfg(test)]
// mod tests {
// use wrt_format::component::ResourceOperation as FormatOp;
// use wrt_foundation::resource::{ResourceDrop, ResourceNew, ResourceRep};
//
// use super::*;
//
// #[test]
// fn test_format_conversion() {
// Test conversion to format types
// let type_idx = 42;
//
// let read_op = to_format_resource_operation(ResourceOperation::Read, type_idx;
// if let FormatOp::Rep(rep) = read_op {
// assert_eq!(rep.type_idx, type_idx;
// } else {
// panic!("Unexpected operation type";
// }
//
// let create_op = to_format_resource_operation(ResourceOperation::Create,
// type_idx; if let FormatOp::New(new) = create_op {
// assert_eq!(new.type_idx, type_idx;
// } else {
// panic!("Unexpected operation type";
// }
//
// Test conversion from format types
// assert_eq!(
// from_format_resource_operation(&FormatOp::Rep(ResourceRep { type_idx })),
// ResourceOperation::Read
// ;
//
// assert_eq!(
// from_format_resource_operation(&FormatOp::New(ResourceNew { type_idx })),
// ResourceOperation::Create
// ;
//
// assert_eq!(
// from_format_resource_operation(&FormatOp::Drop(ResourceDrop { type_idx })),
// ResourceOperation::Delete
// ;
// }
// }
