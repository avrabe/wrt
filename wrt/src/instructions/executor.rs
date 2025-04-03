//! Instruction executor implementation.

use crate::{
    behavior::{FrameBehavior, InstructionExecutor, StackBehavior},
    error::{Error, Result},
    instructions::simd,
    instructions::{arithmetic, comparison, control, memory, numeric, parametric, table, variable},
    module_instance::ModuleInstance,
    stack::Stack,
    types::BlockType,
    Instruction, StacklessEngine,
};
use std::sync::Arc;

// Implement the InstructionExecutor trait for Instruction
impl InstructionExecutor for Instruction {
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        engine: &StacklessEngine,
    ) -> Result<()> {
        match self {
            // Control flow instructions
            Self::Block(block_type) => control::block_dyn(stack, frame, block_type.clone(), engine),
            Self::Loop(block_type) => control::loop_dyn(stack, frame, block_type.clone(), engine),
            Self::If(block_type) => control::if_dyn(stack, frame, block_type.clone(), engine),
            Self::Else => control::else_dyn(stack, frame, engine),
            Self::End => control::end_dyn(stack, frame, engine),
            Self::Br(label_idx) => control::br_dyn(stack, frame, *label_idx, engine),
            Self::BrIf(label_idx) => control::br_if_dyn(stack, frame, *label_idx, engine),
            Instruction::BrTable(label_indices, default_label) => {
                control::br_table_dyn(stack, frame, label_indices.clone(), *default_label, engine)
            }
            Self::Return => control::return_dyn(stack, frame, engine),
            Self::Unreachable => control::unreachable_dyn(stack, frame, engine),
            Self::Nop => control::nop_dyn(stack, frame, engine),

            // Call instructions
            Self::Call(func_idx) => control::call_dyn(stack, frame, *func_idx, engine),
            Self::CallIndirect(type_idx, table_idx) => {
                control::call_indirect_dyn(stack, frame, *type_idx, *table_idx, engine)
            }
            Self::ReturnCall(func_idx) => control::return_call_dyn(stack, frame, *func_idx, engine),
            Self::ReturnCallIndirect(type_idx, table_idx) => {
                control::return_call_indirect_dyn(stack, frame, *type_idx, *table_idx, engine)
            }

            // Parametric instructions
            Self::Drop => parametric::drop(stack, frame, engine),
            Self::Select => parametric::select(stack, frame, engine),
            Self::SelectTyped(value_type) => {
                parametric::select_typed(stack, frame, *value_type, engine)
            }

            // Variable instructions
            Self::LocalGet(idx) => variable::local_get(stack, frame, engine, *idx),
            Self::LocalSet(idx) => variable::local_set(stack, frame, engine, *idx),
            Self::LocalTee(idx) => variable::local_tee(stack, frame, engine, *idx),
            Self::GlobalGet(idx) => variable::global_get(stack, frame, engine, *idx),
            Self::GlobalSet(idx) => variable::global_set(stack, frame, engine, *idx),

            // Table instructions
            Self::TableGet(idx) => table::table_get(stack, frame, engine, *idx),
            Self::TableSet(idx) => table::table_set(stack, frame, engine, *idx),
            Self::TableSize(idx) => table::table_size(stack, frame, engine, *idx),
            Self::TableGrow(idx) => table::table_grow(stack, frame, engine, *idx),
            Self::TableInit(table_idx, elem_idx) => {
                table::table_init(stack, frame, engine, *table_idx, *elem_idx)
            }
            Self::TableCopy(dst_idx, src_idx) => {
                table::table_copy(stack, frame, engine, *dst_idx, *src_idx)
            }
            Self::TableFill(idx) => table::table_fill(stack, frame, engine, *idx),
            Self::ElemDrop(idx) => table::elem_drop(stack, frame, engine, *idx),

            // Memory instructions
            Self::I32Load(offset, align) => memory::i32_load(stack, frame, *offset, *align, engine),
            Self::I64Load(offset, align) => memory::i64_load(stack, frame, *offset, *align, engine),
            Self::F32Load(offset, align) => memory::f32_load(stack, frame, *offset, *align, engine),
            Self::F64Load(offset, align) => memory::f64_load(stack, frame, *offset, *align, engine),
            Self::I32Load8S(offset, align) => {
                memory::i32_load8_s(stack, frame, *offset, *align, engine)
            }
            Self::I32Load8U(offset, align) => {
                memory::i32_load8_u(stack, frame, *offset, *align, engine)
            }
            Self::I32Load16S(offset, align) => {
                memory::i32_load16_s(stack, frame, *offset, *align, engine)
            }
            Self::I32Load16U(offset, align) => {
                memory::i32_load16_u(stack, frame, *offset, *align, engine)
            }
            Self::I64Load8S(offset, align) => {
                memory::i64_load8_s(stack, frame, *offset, *align, engine)
            }
            Self::I64Load8U(offset, align) => {
                memory::i64_load8_u(stack, frame, *offset, *align, engine)
            }
            Self::I64Load16S(offset, align) => {
                memory::i64_load16_s(stack, frame, *offset, *align, engine)
            }
            Self::I64Load16U(offset, align) => {
                memory::i64_load16_u(stack, frame, *offset, *align, engine)
            }
            Self::I64Load32S(offset, align) => {
                memory::i64_load32_s(stack, frame, *offset, *align, engine)
            }
            Self::I64Load32U(offset, align) => {
                memory::i64_load32_u(stack, frame, *offset, *align, engine)
            }
            Self::I32Store(offset, align) => {
                memory::i32_store(stack, frame, *offset, *align, engine)
            }
            Self::I64Store(offset, align) => {
                memory::i64_store(stack, frame, *offset, *align, engine)
            }
            Self::F32Store(offset, align) => {
                memory::f32_store(stack, frame, *offset, *align, engine)
            }
            Self::F64Store(offset, align) => {
                memory::f64_store(stack, frame, *offset, *align, engine)
            }
            Self::I32Store8(offset, align) => {
                memory::i32_store8(stack, frame, *offset, *align, engine)
            }
            Self::I32Store16(offset, align) => {
                memory::i32_store16(stack, frame, *offset, *align, engine)
            }
            Self::I64Store8(offset, align) => {
                memory::i64_store8(stack, frame, *offset, *align, engine)
            }
            Self::I64Store16(offset, align) => {
                memory::i64_store16(stack, frame, *offset, *align, engine)
            }
            Self::I64Store32(offset, align) => {
                memory::i64_store32(stack, frame, *offset, *align, engine)
            }
            Self::MemorySize(mem_idx) => memory::memory_size(stack, frame, *mem_idx, engine),
            Self::MemoryGrow(mem_idx) => memory::memory_grow(stack, frame, *mem_idx, engine),
            Self::MemoryInit(data_idx, mem_idx) => {
                memory::memory_init(stack, frame, *data_idx, *mem_idx, engine)
            }
            Self::DataDrop(idx) => memory::data_drop(stack, frame, *idx, engine),
            Self::MemoryCopy(dst_mem, src_mem) => {
                memory::memory_copy(stack, frame, *dst_mem, *src_mem, engine)
            }
            Self::MemoryFill(mem_idx) => memory::memory_fill(stack, frame, *mem_idx, engine),

            // Numeric constant instructions
            Self::I32Const(value) => numeric::i32_const(stack, frame, *value, engine),
            Self::I64Const(value) => numeric::i64_const(stack, frame, *value, engine),
            Self::F32Const(value) => numeric::f32_const(stack, frame, *value, engine),
            Self::F64Const(value) => numeric::f64_const(stack, frame, *value, engine),

            // Comparison instructions
            Self::I32Eqz => comparison::i32_eqz(stack, frame, engine),
            Self::I32Eq => comparison::i32_eq(stack, frame, engine),
            Self::I32Ne => comparison::i32_ne(stack, frame, engine),
            Self::I32LtS => comparison::i32_lt_s(stack, frame, engine),
            Self::I32LtU => comparison::i32_lt_u(stack, frame, engine),
            Self::I32GtS => comparison::i32_gt_s(stack, frame, engine),
            Self::I32GtU => comparison::i32_gt_u(stack, frame, engine),
            Self::I32LeS => comparison::i32_le_s(stack, frame, engine),
            Self::I32LeU => comparison::i32_le_u(stack, frame, engine),
            Self::I32GeS => comparison::i32_ge_s(stack, frame, engine),
            Self::I32GeU => comparison::i32_ge_u(stack, frame, engine),
            Self::I64Eqz => comparison::i64_eqz(stack, frame, engine),
            Self::I64Eq => comparison::i64_eq(stack, frame, engine),
            Self::I64Ne => comparison::i64_ne(stack, frame, engine),
            Self::I64LtS => comparison::i64_lt_s(stack, frame, engine),
            Self::I64LtU => comparison::i64_lt_u(stack, frame, engine),
            Self::I64GtS => comparison::i64_gt_s(stack, frame, engine),
            Self::I64GtU => comparison::i64_gt_u(stack, frame, engine),
            Self::I64LeS => comparison::i64_le_s(stack, frame, engine),
            Self::I64LeU => comparison::i64_le_u(stack, frame, engine),
            Self::I64GeS => comparison::i64_ge_s(stack, frame, engine),
            Self::I64GeU => comparison::i64_ge_u(stack, frame, engine),
            Self::F32Eq => comparison::f32_eq(stack, frame, engine),
            Self::F32Ne => comparison::f32_ne(stack, frame, engine),
            Self::F32Lt => comparison::f32_lt(stack, frame, engine),
            Self::F32Gt => comparison::f32_gt(stack, frame, engine),
            Self::F32Le => comparison::f32_le(stack, frame, engine),
            Self::F32Ge => comparison::f32_ge(stack, frame, engine),
            Self::F64Eq => comparison::f64_eq(stack, frame, engine),
            Self::F64Ne => comparison::f64_ne(stack, frame, engine),
            Self::F64Lt => comparison::f64_lt(stack, frame, engine),
            Self::F64Gt => comparison::f64_gt(stack, frame, engine),
            Self::F64Le => comparison::f64_le(stack, frame, engine),
            Self::F64Ge => comparison::f64_ge(stack, frame, engine),
            Self::F64Ceil => numeric::f64_ceil(stack, frame, engine),
            Self::F64Floor => numeric::f64_floor(stack, frame, engine),
            Self::F64Trunc => arithmetic::f64_trunc(stack, frame, engine),
            Self::F64Nearest => arithmetic::f64_nearest(stack, frame, engine),

            // Arithmetic instructions
            Self::I32Clz => numeric::i32_clz(stack, frame, engine),
            Self::I32Ctz => numeric::i32_ctz(stack, frame, engine),
            Self::I32Popcnt => numeric::i32_popcnt(stack, frame, engine),
            Self::I32Add => arithmetic::i32_add(stack, frame, engine),
            Self::I32Sub => arithmetic::i32_sub(stack, frame, engine),
            Self::I32Mul => arithmetic::i32_mul(stack, frame, engine),
            Self::I32DivS => arithmetic::i32_div_s(stack, frame, engine),
            Self::I32DivU => arithmetic::i32_div_u(stack, frame, engine),
            Self::I32RemS => arithmetic::i32_rem_s(stack, frame, engine),
            Self::I32RemU => arithmetic::i32_rem_u(stack, frame, engine),
            Self::I32And => arithmetic::i32_and(stack, frame, engine),
            Self::I32Or => arithmetic::i32_or(stack, frame, engine),
            Self::I32Xor => arithmetic::i32_xor(stack, frame, engine),
            Self::I32Shl => arithmetic::i32_shl(stack, frame, engine),
            Self::I32ShrS => arithmetic::i32_shr_s(stack, frame, engine),
            Self::I32ShrU => arithmetic::i32_shr_u(stack, frame, engine),
            Self::I32Rotl => arithmetic::i32_rotl(stack, frame, engine),
            Self::I32Rotr => arithmetic::i32_rotr(stack, frame, engine),
            Self::I64Clz => numeric::i64_clz(stack, frame, engine),
            Self::I64Ctz => numeric::i64_ctz(stack, frame, engine),
            Self::I64Popcnt => numeric::i64_popcnt(stack, frame, engine),
            Self::I64Add => arithmetic::i64_add(stack, frame, engine),
            Self::I64Sub => arithmetic::i64_sub(stack, frame, engine),
            Self::I64Mul => arithmetic::i64_mul(stack, frame, engine),
            Self::I64DivS => arithmetic::i64_div_s(stack, frame, engine),
            Self::I64DivU => arithmetic::i64_div_u(stack, frame, engine),
            Self::I64RemS => arithmetic::i64_rem_s(stack, frame, engine),
            Self::I64RemU => arithmetic::i64_rem_u(stack, frame, engine),
            Self::I64And => arithmetic::i64_and(stack, frame, engine),
            Self::I64Or => arithmetic::i64_or(stack, frame, engine),
            Self::I64Xor => arithmetic::i64_xor(stack, frame, engine),
            Self::I64Shl => arithmetic::i64_shl(stack, frame, engine),
            Self::I64ShrS => arithmetic::i64_shr_s(stack, frame, engine),
            Self::I64ShrU => arithmetic::i64_shr_u(stack, frame, engine),
            Self::I64Rotl => arithmetic::i64_rotl(stack, frame, engine),
            Self::I64Rotr => arithmetic::i64_rotr(stack, frame, engine),
            Self::F32Abs => numeric::f32_abs(stack, frame, engine),
            Self::F32Neg => numeric::f32_neg(stack, frame, engine),
            Self::F32Ceil => numeric::f32_ceil(stack, frame, engine),
            Self::F32Floor => numeric::f32_floor(stack, frame, engine),
            Self::F32Trunc => numeric::f32_trunc(stack, frame, engine),
            Self::F32Nearest => arithmetic::f32_nearest(stack, frame, engine),
            Self::F32Sqrt => numeric::f32_sqrt(stack, frame, engine),
            Self::F32Add => arithmetic::f32_add(stack, frame, engine),
            Self::F32Sub => arithmetic::f32_sub(stack, frame, engine),
            Self::F32Mul => arithmetic::f32_mul(stack, frame, engine),
            Self::F32Div => arithmetic::f32_div(stack, frame, engine),
            Self::F32Min => numeric::f32_min(stack, frame, engine),
            Self::F32Max => numeric::f32_max(stack, frame, engine),
            Self::F32Copysign => numeric::f32_copysign(stack, frame, engine),
            Self::F64Abs => numeric::f64_abs(stack, frame, engine),
            Self::F64Neg => numeric::f64_neg(stack, frame, engine),
            Self::F64Sqrt => arithmetic::f64_sqrt(stack, frame, engine),
            Self::F64Add => arithmetic::f64_add(stack, frame, engine),
            Self::F64Sub => arithmetic::f64_sub(stack, frame, engine),
            Self::F64Mul => arithmetic::f64_mul(stack, frame, engine),
            Self::F64Div => arithmetic::f64_div(stack, frame, engine),
            Self::F64Min => arithmetic::f64_min(stack, frame, engine),
            Self::F64Max => arithmetic::f64_max(stack, frame, engine),
            Self::F64Copysign => numeric::f64_copysign(stack, frame, engine),

            // Conversion instructions
            Self::I32WrapI64 => arithmetic::i32_wrap_i64(stack, frame, engine),
            Self::I64ExtendI32S => arithmetic::i64_extend_i32_s(stack, frame, engine),
            Self::I64ExtendI32U => arithmetic::i64_extend_i32_u(stack, frame, engine),
            Self::I32TruncF32S => arithmetic::i32_trunc_f32_s(stack, frame, engine),
            Self::I32TruncF32U => arithmetic::i32_trunc_f32_u(stack, frame, engine),
            Self::I32TruncF64S => arithmetic::i32_trunc_f64_s(stack, frame, engine),
            Self::I32TruncF64U => arithmetic::i32_trunc_f64_u(stack, frame, engine),
            Self::I64TruncF32S => arithmetic::i64_trunc_f32_s(stack, frame, engine),
            Self::I64TruncF32U => arithmetic::i64_trunc_f32_u(stack, frame, engine),
            Self::I64TruncF64S => arithmetic::i64_trunc_f64_s(stack, frame, engine),
            Self::I64TruncF64U => arithmetic::i64_trunc_f64_u(stack, frame, engine),
            Self::F32ConvertI32S => arithmetic::f32_convert_i32_s(stack, frame, engine),
            Self::F32ConvertI32U => arithmetic::f32_convert_i32_u(stack, frame, engine),
            Self::F32ConvertI64S => arithmetic::f32_convert_i64_s(stack, frame, engine),
            Self::F32ConvertI64U => arithmetic::f32_convert_i64_u(stack, frame, engine),
            Self::F32DemoteF64 => arithmetic::f32_demote_f64(stack, frame, engine),
            Self::F64ConvertI32S => arithmetic::f64_convert_i32_s(stack, frame, engine),
            Self::F64ConvertI32U => arithmetic::f64_convert_i32_u(stack, frame, engine),
            Self::F64ConvertI64S => arithmetic::f64_convert_i64_s(stack, frame, engine),
            Self::F64ConvertI64U => arithmetic::f64_convert_i64_u(stack, frame, engine),
            Self::F64PromoteF32 => arithmetic::f64_promote_f32(stack, frame, engine),

            // Reinterpret instructions
            Self::I32ReinterpretF32 => arithmetic::i32_reinterpret_f32(stack, frame, engine),
            Self::I64ReinterpretF64 => arithmetic::i64_reinterpret_f64(stack, frame, engine),
            Self::F32ReinterpretI32 => arithmetic::f32_reinterpret_i32(stack, frame, engine),
            Self::F64ReinterpretI64 => arithmetic::f64_reinterpret_i64(stack, frame, engine),

            // Extend instructions
            Self::I32Extend8S => arithmetic::i32_extend8_s(stack, frame, engine),
            Self::I32Extend16S => arithmetic::i32_extend16_s(stack, frame, engine),
            Self::I64Extend8S => arithmetic::i64_extend8_s(stack, frame, engine),
            Self::I64Extend16S => arithmetic::i64_extend16_s(stack, frame, engine),
            Self::I64Extend32S => arithmetic::i64_extend32_s(stack, frame, engine),

            // Saturating conversion instructions
            Self::I32TruncSatF32S => arithmetic::i32_trunc_sat_f32_s(stack, frame, engine),
            Self::I32TruncSatF32U => arithmetic::i32_trunc_sat_f32_u(stack, frame, engine),
            Self::I32TruncSatF64S => arithmetic::i32_trunc_sat_f64_s(stack, frame, engine),
            Self::I32TruncSatF64U => arithmetic::i32_trunc_sat_f64_u(stack, frame, engine),
            Self::I64TruncSatF32S => arithmetic::i64_trunc_sat_f32_s(stack, frame, engine),
            Self::I64TruncSatF32U => arithmetic::i64_trunc_sat_f32_u(stack, frame, engine),
            Self::I64TruncSatF64S => arithmetic::i64_trunc_sat_f64_s(stack, frame, engine),
            Self::I64TruncSatF64U => arithmetic::i64_trunc_sat_f64_u(stack, frame, engine),

            // Reference type instructions
            // Self::RefNull(heap_type) => variable::ref_null(stack, frame, *heap_type, engine),
            // Self::RefIsNull => variable::ref_is_null(stack, frame, engine),
            // Self::RefFunc(func_idx) => variable::ref_func(stack, frame, *func_idx, engine),

            // Bulk memory operations
            // Already handled under Memory instructions

            // SIMD instructions - Add calls to SIMD handlers
            Self::V128Load(offset, align) => {
                memory::v128_load(stack, frame, *offset, *align, engine)
            }
            // Self::V128Load8x8S { offset, align } => memory::v128_load8x8_s(stack, frame, *offset, *align, engine),
            // Self::V128Load8x8U { offset, align } => memory::v128_load8x8_u(stack, frame, *offset, *align, engine),
            // Self::V128Load16x4S { offset, align } => memory::v128_load16x4_s(stack, frame, *offset, *align, engine),
            // Self::V128Load16x4U { offset, align } => memory::v128_load16x4_u(stack, frame, *offset, *align, engine),
            // Self::V128Load32x2S { offset, align } => memory::v128_load32x2_s(stack, frame, *offset, *align, engine),
            // Self::V128Load32x2U { offset, align } => memory::v128_load32x2_u(stack, frame, *offset, *align, engine),
            // Self::V128Load8Splat { offset, align } => memory::v128_load8_splat(stack, frame, *offset, *align, engine),
            // Self::V128Load16Splat { offset, align } => memory::v128_load16_splat(stack, frame, *offset, *align, engine),
            // Self::V128Load32Splat { offset, align } => memory::v128_load32_splat(stack, frame, *offset, *align, engine),
            // Self::V128Load64Splat { offset, align } => memory::v128_load64_splat(stack, frame, *offset, *align, engine),
            // Self::V128Load32Zero { offset, align } => memory::v128_load32_zero(stack, frame, *offset, *align, engine),
            // Self::V128Load64Zero { offset, align } => memory::v128_load64_zero(stack, frame, *offset, *align, engine),
            Self::V128Store(offset, align) => {
                memory::v128_store(stack, frame, *offset, *align, engine)
            }
            // Self::V128Load8Lane { offset, align, lane_idx } => memory::v128_load8_lane(stack, frame, *offset, *align, *lane_idx, engine),
            // Self::V128Load16Lane { offset, align, lane_idx } => memory::v128_load16_lane(stack, frame, *offset, *align, *lane_idx, engine),
            // Self::V128Load32Lane { offset, align, lane_idx } => memory::v128_load32_lane(stack, frame, *offset, *align, *lane_idx, engine),
            // Self::V128Load64Lane { offset, align, lane_idx } => memory::v128_load64_lane(stack, frame, *offset, *align, *lane_idx, engine),
            // Self::V128Store8Lane { offset, align, lane_idx } => memory::v128_store8_lane(stack, frame, *offset, *align, *lane_idx, engine),
            // Self::V128Store16Lane { offset, align, lane_idx } => memory::v128_store16_lane(stack, frame, *offset, *align, *lane_idx, engine),
            // Self::V128Store32Lane { offset, align, lane_idx } => memory::v128_store32_lane(stack, frame, *offset, *align, *lane_idx, engine),
            // Self::V128Store64Lane { offset, align, lane_idx } => memory::v128_store64_lane(stack, frame, *offset, *align, *lane_idx, engine),
            Self::V128Const(val) => simd::v128_const(stack, frame, *val, engine),
            Self::I8x16Shuffle(indices) => simd::i8x16_shuffle(stack, frame, *indices, engine),
            Self::I8x16ExtractLaneS(lane_idx) => {
                simd::i8x16_extract_lane_s(stack, frame, *lane_idx)
            }
            Self::I8x16ExtractLaneU(lane_idx) => {
                simd::i8x16_extract_lane_u(stack, frame, *lane_idx)
            }
            Self::I8x16ReplaceLane(lane_idx) => simd::i8x16_replace_lane(stack, frame, *lane_idx),
            Self::I16x8ExtractLaneS(lane_idx) => {
                simd::i16x8_extract_lane_s(stack, frame, *lane_idx)
            }
            Self::I16x8ExtractLaneU(lane_idx) => {
                simd::i16x8_extract_lane_u(stack, frame, *lane_idx)
            }
            Self::I16x8ReplaceLane(lane_idx) => simd::i16x8_replace_lane(stack, frame, *lane_idx),
            Self::I32x4ExtractLane(lane_idx) => simd::i32x4_extract_lane(stack, frame, *lane_idx),
            Self::I32x4ReplaceLane(lane_idx) => simd::i32x4_replace_lane(stack, frame, *lane_idx),
            Self::I64x2ExtractLane(lane_idx) => simd::i64x2_extract_lane(stack, frame, *lane_idx),
            Self::I64x2ReplaceLane(lane_idx) => simd::i64x2_replace_lane(stack, frame, *lane_idx),
            Self::F32x4ExtractLane(lane_idx) => simd::f32x4_extract_lane(stack, frame, *lane_idx),
            Self::F32x4ReplaceLane(lane_idx) => simd::f32x4_replace_lane(stack, frame, *lane_idx),
            Self::F64x2ExtractLane(lane_idx) => simd::f64x2_extract_lane(stack, frame, *lane_idx),
            Self::F64x2ReplaceLane(lane_idx) => simd::f64x2_replace_lane(stack, frame, *lane_idx),

            Self::I8x16Swizzle => simd::i8x16_swizzle(stack, frame, engine),
            Self::I8x16Splat => simd::i8x16_splat(stack, frame),
            Self::I16x8Splat => simd::i16x8_splat(stack, frame),
            Self::I32x4Splat => simd::i32x4_splat(stack, frame),
            Self::I64x2Splat => simd::i64x2_splat(stack, frame),
            Self::F32x4Splat => simd::f32x4_splat(stack, frame),
            Self::F64x2Splat => simd::f64x2_splat(stack, frame),

            // SIMD Comparison Ops (Corrected Paths)
            Self::I8x16Eq => simd::i8x16_eq(stack, frame),
            Self::I8x16Ne => simd::i8x16_ne(stack, frame),
            Self::I8x16LtS => simd::i8x16_lt_s(stack, frame),
            Self::I8x16LtU => simd::i8x16_lt_u(stack, frame),
            Self::I8x16GtS => simd::i8x16_gt_s(stack, frame),
            Self::I8x16GtU => simd::i8x16_gt_u(stack, frame),
            Self::I8x16LeS => simd::i8x16_le_s(stack, frame),
            Self::I8x16LeU => simd::i8x16_le_u(stack, frame),
            Self::I8x16GeS => simd::i8x16_ge_s(stack, frame),
            Self::I8x16GeU => simd::i8x16_ge_u(stack, frame),
            Self::I16x8Eq => simd::i16x8_eq(stack, frame),
            Self::I16x8Ne => simd::i16x8_ne(stack, frame),
            Self::I16x8LtS => simd::i16x8_lt_s(stack, frame),
            Self::I16x8LtU => simd::i16x8_lt_u(stack, frame),
            Self::I16x8GtS => simd::i16x8_gt_s(stack, frame),
            Self::I16x8GtU => simd::i16x8_gt_u(stack, frame),
            Self::I16x8LeS => simd::i16x8_le_s(stack, frame),
            Self::I16x8LeU => simd::i16x8_le_u(stack, frame),
            Self::I16x8GeS => simd::i16x8_ge_s(stack, frame),
            Self::I16x8GeU => simd::i16x8_ge_u(stack, frame),
            Self::I32x4Eq => simd::i32x4_eq(stack, frame),
            Self::I32x4Ne => simd::i32x4_ne(stack, frame),
            Self::I32x4LtS => simd::i32x4_lt_s(stack, frame),
            Self::I32x4LtU => simd::i32x4_lt_u(stack, frame),
            Self::I32x4GtS => simd::i32x4_gt_s(stack, frame),
            Self::I32x4GtU => simd::i32x4_gt_u(stack, frame),
            Self::I32x4LeS => simd::i32x4_le_s(stack, frame),
            Self::I32x4LeU => simd::i32x4_le_u(stack, frame),
            Self::I32x4GeS => simd::i32x4_ge_s(stack, frame),
            Self::I32x4GeU => simd::i32x4_ge_u(stack, frame),
            Self::I64x2Eq => simd::i64x2_eq(stack, frame, engine),
            Self::I64x2Ne => simd::i64x2_ne(stack, frame, engine),
            Self::I64x2LtS => simd::i64x2_lt_s(stack, frame, engine),
            Self::I64x2GtS => simd::i64x2_gt_s(stack, frame, engine),
            Self::I64x2LeS => simd::i64x2_le_s(stack, frame, engine),
            Self::I64x2GeS => simd::i64x2_ge_s(stack, frame, engine),
            Self::F32x4Eq => simd::f32x4_eq(stack, frame, engine),
            Self::F32x4Ne => simd::f32x4_ne(stack, frame, engine),
            Self::F32x4Lt => simd::f32x4_lt(stack, frame, engine),
            Self::F32x4Gt => simd::f32x4_gt(stack, frame, engine),
            Self::F32x4Le => simd::f32x4_le(stack, frame, engine),
            Self::F32x4Ge => simd::f32x4_ge(stack, frame, engine),
            Self::F64x2Eq => simd::f64x2_eq(stack, frame, engine),
            Self::F64x2Ne => simd::f64x2_ne(stack, frame, engine),
            Self::F64x2Lt => simd::f64x2_lt(stack, frame, engine),
            Self::F64x2Gt => simd::f64x2_gt(stack, frame, engine),
            Self::F64x2Le => simd::f64x2_le(stack, frame, engine),
            Self::F64x2Ge => simd::f64x2_ge(stack, frame, engine),

            // SIMD V128 Ops
            Self::V128Not => simd::v128_not(stack, frame, engine),
            Self::V128And => simd::v128_and(stack, frame, engine),
            Self::V128AndNot => simd::v128_andnot(stack, frame, engine),
            Self::V128Or => simd::v128_or(stack, frame, engine),
            Self::V128Xor => simd::v128_xor(stack, frame, engine),
            Self::V128Bitselect => simd::v128_bitselect(stack, frame, engine),
            Self::V128AnyTrue => simd::v128_any_true(stack, frame, engine),

            Self::I8x16Abs => simd::i8x16_abs(stack, frame),
            Self::I8x16Neg => simd::i8x16_neg(stack, frame),
            Self::I8x16Popcnt => simd::i8x16_popcnt(stack, frame),
            Self::I8x16AllTrue => simd::i8x16_all_true(stack, frame),
            Self::I8x16Bitmask => simd::i8x16_bitmask(stack, frame),
            Self::I8x16Shl => simd::i8x16_shl(stack, frame, engine),
            Self::I8x16ShrS => simd::i8x16_shr_s(stack, frame, engine),
            Self::I8x16ShrU => simd::i8x16_shr_u(stack, frame, engine),
            Self::I8x16Add => simd::i8x16_add(stack, frame),
            Self::I8x16AddSatS => simd::i8x16_add_sat_s(stack, frame, engine),
            Self::I8x16AddSatU => simd::i8x16_add_sat_u(stack, frame, engine),
            Self::I8x16Sub => simd::i8x16_sub(stack, frame),
            Self::I8x16SubSatS => simd::i8x16_sub_sat_s(stack, frame, engine),
            Self::I8x16SubSatU => simd::i8x16_sub_sat_u(stack, frame, engine),
            Self::I8x16MinS => simd::i8x16_min_s(stack, frame),
            Self::I8x16MinU => simd::i8x16_min_u(stack, frame),
            Self::I8x16MaxS => simd::i8x16_max_s(stack, frame),
            Self::I8x16MaxU => simd::i8x16_max_u(stack, frame),
            Self::I8x16AvgrU => simd::i8x16_avgr_u(stack, frame, engine),

            Self::I16x8Q15MulrSatS => simd::i16x8_q15mulr_sat_s(stack, frame, engine),
            Self::I16x8AllTrue => simd::i16x8_all_true(stack, frame, engine),
            Self::I16x8Bitmask => simd::i16x8_bitmask(stack, frame, engine),
            Self::I16x8Shl => simd::i16x8_shl(stack, frame, engine),
            Self::I16x8ShrS => simd::i16x8_shr_s(stack, frame, engine),
            Self::I16x8ShrU => simd::i16x8_shr_u(stack, frame, engine),
            Self::I16x8Add => simd::i16x8_add(stack, frame),
            Self::I16x8AddSatS => simd::i16x8_add_sat_s(stack, frame, engine),
            Self::I16x8AddSatU => simd::i16x8_add_sat_u(stack, frame, engine),
            Self::I16x8Sub => simd::i16x8_sub(stack, frame),
            Self::I16x8SubSatS => simd::i16x8_sub_sat_s(stack, frame, engine),
            Self::I16x8SubSatU => simd::i16x8_sub_sat_u(stack, frame, engine),
            Self::I16x8Mul => simd::i16x8_mul(stack, frame),
            Self::I16x8MinS => simd::i16x8_min_s(stack, frame),
            Self::I16x8MinU => simd::i16x8_min_u(stack, frame),
            Self::I16x8MaxS => simd::i16x8_max_s(stack, frame),
            Self::I16x8MaxU => simd::i16x8_max_u(stack, frame),
            Self::I16x8AvgrU => simd::i16x8_avgr_u(stack, frame, engine),
            Self::I16x8ExtMulLowI8x16S => simd::i16x8_extmul_low_i8x16_s(stack, frame, engine),
            Self::I16x8ExtMulHighI8x16S => simd::i16x8_extmul_high_i8x16_s(stack, frame, engine),
            Self::I16x8ExtMulLowI8x16U => simd::i16x8_extmul_low_i8x16_u(stack, frame, engine),
            Self::I16x8ExtMulHighI8x16U => simd::i16x8_extmul_high_i8x16_u(stack, frame, engine),

            Self::I32x4ExtaddPairwiseI16x8S => simd::i32x4_extadd_pairwise_i16x8_s(stack, frame),
            Self::I32x4ExtaddPairwiseI16x8U => simd::i32x4_extadd_pairwise_i16x8_u(stack, frame),
            Self::I32x4Abs => simd::i32x4_abs(stack, frame, engine),
            Self::I32x4Neg => simd::i32x4_neg(stack, frame),
            Self::I32x4AllTrue => simd::i32x4_all_true(stack, frame, engine),
            Self::I32x4Bitmask => simd::i32x4_bitmask(stack, frame, engine),
            Self::I32x4Shl => simd::i32x4_shl(stack, frame, engine),
            Self::I32x4ShrS => simd::i32x4_shr_s(stack, frame, engine),
            Self::I32x4ShrU => simd::i32x4_shr_u(stack, frame, engine),
            Self::I32x4Add => simd::i32x4_add(stack, frame),
            Self::I32x4Sub => simd::i32x4_sub(stack, frame),
            Self::I32x4Mul => simd::i32x4_mul(stack, frame),
            Self::I32x4MinS => simd::i32x4_min_s(stack, frame),
            Self::I32x4MinU => simd::i32x4_min_u(stack, frame),
            Self::I32x4MaxS => simd::i32x4_max_s(stack, frame),
            Self::I32x4MaxU => simd::i32x4_max_u(stack, frame),
            Self::I32x4DotI16x8S => simd::i32x4_dot_i16x8_s(stack, frame, engine),
            Self::I32x4ExtMulLowI16x8S => simd::i32x4_extmul_low_i16x8_s(stack, frame, engine),
            Self::I32x4ExtMulHighI16x8S => simd::i32x4_extmul_high_i16x8_s(stack, frame, engine),
            Self::I32x4ExtMulLowI16x8U => simd::i32x4_extmul_low_i16x8_u(stack, frame, engine),
            Self::I32x4ExtMulHighI16x8U => simd::i32x4_extmul_high_i16x8_u(stack, frame, engine),

            Self::I64x2Abs => simd::i64x2_abs(stack, frame, engine),
            Self::I64x2Neg => simd::i64x2_neg(stack, frame),
            Self::I64x2AllTrue => simd::i64x2_all_true(stack, frame, engine),
            Self::I64x2Bitmask => simd::i64x2_bitmask(stack, frame, engine),
            Self::I64x2Shl => simd::i64x2_shl(stack, frame, engine),
            Self::I64x2ShrS => simd::i64x2_shr_s(stack, frame, engine),
            Self::I64x2ShrU => simd::i64x2_shr_u(stack, frame, engine),
            Self::I64x2Add => simd::i64x2_add(stack, frame),
            Self::I64x2Sub => simd::i64x2_sub(stack, frame),
            Self::I64x2Mul => simd::i64x2_mul(stack, frame),
            Self::I64x2ExtMulLowI32x4S => simd::i64x2_extmul_low_i32x4_s(stack, frame, engine),
            Self::I64x2ExtMulHighI32x4S => simd::i64x2_extmul_high_i32x4_s(stack, frame, engine),
            Self::I64x2ExtMulLowI32x4U => simd::i64x2_extmul_low_i32x4_u(stack, frame, engine),
            Self::I64x2ExtMulHighI32x4U => simd::i64x2_extmul_high_i32x4_u(stack, frame, engine),

            Self::F32x4Ceil => simd::f32x4_ceil(stack, frame, engine),
            Self::F32x4Floor => simd::f32x4_floor(stack, frame, engine),
            Self::F32x4Trunc => simd::f32x4_trunc(stack, frame, engine),
            Self::F32x4Nearest => simd::f32x4_nearest(stack, frame, engine),
            Self::F32x4Abs => simd::f32x4_abs(stack, frame, engine),
            Self::F32x4Neg => simd::f32x4_neg(stack, frame),
            Self::F32x4Sqrt => simd::f32x4_sqrt(stack, frame, engine),
            Self::F32x4Add => simd::f32x4_add(stack, frame),
            Self::F32x4Sub => simd::f32x4_sub(stack, frame),
            Self::F32x4Mul => simd::f32x4_mul(stack, frame),
            Self::F32x4Div => simd::f32x4_div(stack, frame),
            Self::F32x4Min => simd::f32x4_min(stack, frame, engine),
            Self::F32x4Max => simd::f32x4_max(stack, frame, engine),
            Self::F32x4PMin => simd::f32x4_pmin(stack, frame, engine),
            Self::F32x4PMax => simd::f32x4_pmax(stack, frame, engine),

            Self::F32x4Sub => simd::f32x4_sub(stack, frame),
            Self::F32x4Mul => simd::f32x4_mul(stack, frame),
            Self::F32x4Div => simd::f32x4_div(stack, frame),
            Self::F32x4Min => simd::f32x4_min(stack, frame, engine),
            Self::F32x4Max => simd::f32x4_max(stack, frame, engine),
            Self::F32x4PMin => simd::f32x4_pmin(stack, frame, engine),
            Self::F32x4PMax => simd::f32x4_pmax(stack, frame, engine),

            Self::F64x2Ceil => simd::f64x2_ceil(stack, frame, engine),
            Self::F64x2Floor => simd::f64x2_floor(stack, frame, engine),
            Self::F64x2Trunc => simd::f64x2_trunc(stack, frame, engine),
            Self::F64x2Nearest => simd::f64x2_nearest(stack, frame, engine),
            Self::F64x2Abs => simd::f64x2_abs(stack, frame, engine),
            Self::F64x2Neg => simd::f64x2_neg(stack, frame),
            Self::F64x2Sqrt => simd::f64x2_sqrt(stack, frame, engine),
            Self::F64x2Add => simd::f64x2_add(stack, frame),
            Self::F64x2Sub => simd::f64x2_sub(stack, frame),
            Self::F64x2Mul => simd::f64x2_mul(stack, frame),
            Self::F64x2Div => simd::f64x2_div(stack, frame),
            Self::F64x2Min => simd::f64x2_min(stack, frame, engine),
            Self::F64x2Max => simd::f64x2_max(stack, frame, engine),
            Self::F64x2PMin => simd::f64x2_pmin(stack, frame, engine),
            Self::F64x2PMax => simd::f64x2_pmax(stack, frame, engine),

            Self::I32x4TruncSatF32x4S => simd::i32x4_trunc_sat_f32x4_s(stack, frame, engine),
            Self::I32x4TruncSatF32x4U => simd::i32x4_trunc_sat_f32x4_u(stack, frame, engine),
            Self::F32x4ConvertI32x4S => simd::f32x4_convert_i32x4_s(stack, frame, engine),
            Self::F32x4ConvertI32x4U => simd::f32x4_convert_i32x4_u(stack, frame, engine),
            Self::I32x4TruncSatF64x2SZero => {
                simd::i32x4_trunc_sat_f64x2_s_zero(stack, frame, engine)
            }
            Self::I32x4TruncSatF64x2UZero => {
                simd::i32x4_trunc_sat_f64x2_u_zero(stack, frame, engine)
            }
            Self::F64x2ConvertLowI32x4S => simd::f64x2_convert_low_i32x4_s(stack, frame, engine),
            Self::F64x2ConvertLowI32x4U => simd::f64x2_convert_low_i32x4_u(stack, frame, engine),
            Self::F32x4DemoteF64x2Zero => simd::f32x4_demote_f64x2_zero(stack, frame, engine),
            Self::F64x2PromoteLowF32x4 => simd::f64x2_promote_low_f32x4(stack, frame, engine),

            Self::I8x16NarrowI16x8S => simd::i8x16_narrow_i16x8_s(stack, frame, engine),
            Self::I8x16NarrowI16x8U => simd::i8x16_narrow_i16x8_u(stack, frame, engine),
            Self::I16x8NarrowI32x4S => simd::i16x8_narrow_i32x4_s(stack, frame, engine),
            Self::I16x8NarrowI32x4U => simd::i16x8_narrow_i32x4_u(stack, frame, engine),

            Self::I16x8ExtendLowI8x16S => simd::i16x8_extend_low_i8x16_s(stack, frame, engine),
            Self::I16x8ExtendHighI8x16S => simd::i16x8_extend_high_i8x16_s(stack, frame, engine),
            Self::I16x8ExtendLowI8x16U => simd::i16x8_extend_low_i8x16_u(stack, frame, engine),
            Self::I16x8ExtendHighI8x16U => simd::i16x8_extend_high_i8x16_u(stack, frame, engine),
            Self::I32x4ExtendLowI16x8S => simd::i32x4_extend_low_i16x8_s(stack, frame, engine),
            Self::I32x4ExtendHighI16x8S => simd::i32x4_extend_high_i16x8_s(stack, frame, engine),
            Self::I32x4ExtendLowI16x8U => simd::i32x4_extend_low_i16x8_u(stack, frame, engine),
            Self::I32x4ExtendHighI16x8U => simd::i32x4_extend_high_i16x8_u(stack, frame, engine),
            Self::I64x2ExtendLowI32x4S => simd::i64x2_extend_low_i32x4_s(stack, frame, engine),
            Self::I64x2ExtendHighI32x4S => simd::i64x2_extend_high_i32x4_s(stack, frame, engine),
            Self::I64x2ExtendLowI32x4U => simd::i64x2_extend_low_i32x4_u(stack, frame, engine),
            Self::I64x2ExtendHighI32x4U => simd::i64x2_extend_high_i32x4_u(stack, frame, engine),

            // Catch-all for unimplemented instructions
            _ => Err(Error::Unimplemented(format!("Instruction: {:?}", self))),
        }
    }
}
