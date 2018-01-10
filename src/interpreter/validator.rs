use std::u32;
use std::sync::Arc;
use std::collections::HashMap;
use elements::{Opcode, BlockType, ValueType};
use interpreter::Error;
use common::{DEFAULT_MEMORY_INDEX, DEFAULT_TABLE_INDEX};
use interpreter::module::{ModuleInstance, ModuleInstanceInterface, ItemIndex, FunctionSignature};
use common::stack::StackWithLimit;
use interpreter::variable::VariableType;

/// Constant from wabt' validator.cc to skip alignment validation (not a part of spec).
const NATURAL_ALIGNMENT: u32 = 0xFFFFFFFF;

/// Function validation context.
pub struct FunctionValidationContext<'a> {
	/// Wasm module instance (in process of instantiation).
	module_instance: &'a ModuleInstance,
	/// Native externals.
	externals: Option<&'a HashMap<String, Arc<ModuleInstanceInterface + 'a>>>,
	/// Current instruction position.
	position: usize,
	/// Local variables.
	locals: &'a [ValueType],
	/// Value stack.
	value_stack: StackWithLimit<StackValueType>,
	/// Frame stack.
	frame_stack: StackWithLimit<BlockFrame>,
	/// Function return type. None if validating expression.
	return_type: Option<BlockType>,
	/// Labels positions.
	labels: HashMap<usize, usize>,
}

/// Value type on the stack.
#[derive(Debug, Clone, Copy)]
pub enum StackValueType {
	/// Any value type.
	Any,
	/// Any number of any values of any type.
	AnyUnlimited,
	/// Concrete value type.
	Specific(ValueType),
}

/// Control stack frame.
#[derive(Debug, Clone)]
pub struct BlockFrame {
	/// Frame type.
	pub frame_type: BlockFrameType,
	/// A signature, which is a block signature type indicating the number and types of result values of the region.
	pub block_type: BlockType,
	/// A label for reference to block instruction.
	pub begin_position: usize,
	/// A label for reference from branch instructions.
	pub branch_position: usize,
	/// A label for reference from end instructions.
	pub end_position: usize,
	/// A limit integer value, which is an index into the value stack indicating where to reset it to on a branch to that label.
	pub value_stack_len: usize,
}

/// Type of block frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockFrameType {
	/// Function frame.
	Function,
	/// Usual block frame.
	Block,
	/// Loop frame (branching to the beginning of block).
	Loop,
	/// True-subblock of if expression.
	IfTrue,
	/// False-subblock of if expression.
	IfFalse,
}

/// Function validator.
pub struct Validator;

/// Instruction outcome.
#[derive(Debug, Clone)]
pub enum InstructionOutcome {
	/// Continue with next instruction.
	ValidateNextInstruction,
	/// Unreachable instruction reached.
	Unreachable,
}

impl Validator {
	pub fn validate_function(context: &mut FunctionValidationContext, block_type: BlockType, body: &[Opcode]) -> Result<(), Error> {
		context.push_label(BlockFrameType::Function, block_type)?;
		Validator::validate_function_block(context, body)?;
		while !context.frame_stack.is_empty() {
			context.pop_label()?;
		}

		Ok(())
	}

	fn validate_function_block(context: &mut FunctionValidationContext, body: &[Opcode]) -> Result<(), Error> {
		let body_len = body.len();
		if body_len == 0 {
			return Err(Error::Validation("Non-empty function body expected".into()));
		}

		loop {
			let opcode = &body[context.position];
			match Validator::validate_instruction(context, opcode)? {
				InstructionOutcome::ValidateNextInstruction => (),
				InstructionOutcome::Unreachable => context.unreachable()?,
			}

			context.position += 1;
			if context.position >= body_len {
				return Ok(());
			}
		}
	}

	fn validate_instruction(context: &mut FunctionValidationContext, opcode: &Opcode) -> Result<InstructionOutcome, Error> {
		debug!(target: "validator", "validating {:?}", opcode);
		match opcode {
			&Opcode::Unreachable => Ok(InstructionOutcome::Unreachable),
			&Opcode::Nop => Ok(InstructionOutcome::ValidateNextInstruction),
			&Opcode::Block(block_type) => Validator::validate_block(context, block_type),
			&Opcode::Loop(block_type) => Validator::validate_loop(context, block_type),
			&Opcode::If(block_type) => Validator::validate_if(context, block_type),
			&Opcode::Else => Validator::validate_else(context),
			&Opcode::End => Validator::validate_end(context),
			&Opcode::Br(idx) => Validator::validate_br(context, idx),
			&Opcode::BrIf(idx) => Validator::validate_br_if(context, idx),
			&Opcode::BrTable(ref table, default) => Validator::validate_br_table(context, table, default),
			&Opcode::Return => Validator::validate_return(context),

			&Opcode::Call(index) => Validator::validate_call(context, index),
			&Opcode::CallIndirect(index, _reserved) => Validator::validate_call_indirect(context, index),

			&Opcode::Drop => Validator::validate_drop(context),
			&Opcode::Select => Validator::validate_select(context),

			&Opcode::GetLocal(index) => Validator::validate_get_local(context, index),
			&Opcode::SetLocal(index) => Validator::validate_set_local(context, index),
			&Opcode::TeeLocal(index) => Validator::validate_tee_local(context, index),
			&Opcode::GetGlobal(index) => Validator::validate_get_global(context, index),
			&Opcode::SetGlobal(index) => Validator::validate_set_global(context, index),

			&Opcode::I32Load(align, _) => Validator::validate_load(context, align, 4, ValueType::I32.into()),
			&Opcode::I64Load(align, _) => Validator::validate_load(context, align, 8, ValueType::I64.into()),
			&Opcode::F32Load(align, _) => Validator::validate_load(context, align, 4, ValueType::F32.into()),
			&Opcode::F64Load(align, _) => Validator::validate_load(context, align, 8, ValueType::F64.into()),
			&Opcode::I32Load8S(align, _) => Validator::validate_load(context, align, 1, ValueType::I32.into()),
			&Opcode::I32Load8U(align, _) => Validator::validate_load(context, align, 1, ValueType::I32.into()),
			&Opcode::I32Load16S(align, _) => Validator::validate_load(context, align, 2, ValueType::I32.into()),
			&Opcode::I32Load16U(align, _) => Validator::validate_load(context, align, 2, ValueType::I32.into()),
			&Opcode::I64Load8S(align, _) => Validator::validate_load(context, align, 1, ValueType::I64.into()),
			&Opcode::I64Load8U(align, _) => Validator::validate_load(context, align, 1, ValueType::I64.into()),
			&Opcode::I64Load16S(align, _) => Validator::validate_load(context, align, 2, ValueType::I64.into()),
			&Opcode::I64Load16U(align, _) => Validator::validate_load(context, align, 2, ValueType::I64.into()),
			&Opcode::I64Load32S(align, _) => Validator::validate_load(context, align, 4, ValueType::I64.into()),
			&Opcode::I64Load32U(align, _) => Validator::validate_load(context, align, 4, ValueType::I64.into()),

			&Opcode::I32Store(align, _) => Validator::validate_store(context, align, 4, ValueType::I32.into()),
			&Opcode::I64Store(align, _) => Validator::validate_store(context, align, 8, ValueType::I64.into()),
			&Opcode::F32Store(align, _) => Validator::validate_store(context, align, 4, ValueType::F32.into()),
			&Opcode::F64Store(align, _) => Validator::validate_store(context, align, 8, ValueType::F64.into()),
			&Opcode::I32Store8(align, _) => Validator::validate_store(context, align, 1, ValueType::I32.into()),
			&Opcode::I32Store16(align, _) => Validator::validate_store(context, align, 2, ValueType::I32.into()),
			&Opcode::I64Store8(align, _) => Validator::validate_store(context, align, 1, ValueType::I64.into()),
			&Opcode::I64Store16(align, _) => Validator::validate_store(context, align, 2, ValueType::I64.into()),
			&Opcode::I64Store32(align, _) => Validator::validate_store(context, align, 4, ValueType::I64.into()),

			&Opcode::CurrentMemory(_) => Validator::validate_current_memory(context),
			&Opcode::GrowMemory(_) => Validator::validate_grow_memory(context),

			&Opcode::I32Const(_) => Validator::validate_const(context, ValueType::I32.into()),
			&Opcode::I64Const(_) => Validator::validate_const(context, ValueType::I64.into()),
			&Opcode::F32Const(_) => Validator::validate_const(context, ValueType::F32.into()),
			&Opcode::F64Const(_) => Validator::validate_const(context, ValueType::F64.into()),

			&Opcode::I32Eqz => Validator::validate_testop(context, ValueType::I32.into()),
			&Opcode::I32Eq => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32Ne => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32LtS => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32LtU => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32GtS => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32GtU => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32LeS => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32LeU => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32GeS => Validator::validate_relop(context, ValueType::I32.into()),
			&Opcode::I32GeU => Validator::validate_relop(context, ValueType::I32.into()),

			&Opcode::I64Eqz => Validator::validate_testop(context, ValueType::I64.into()),
			&Opcode::I64Eq => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64Ne => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64LtS => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64LtU => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64GtS => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64GtU => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64LeS => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64LeU => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64GeS => Validator::validate_relop(context, ValueType::I64.into()),
			&Opcode::I64GeU => Validator::validate_relop(context, ValueType::I64.into()),

			&Opcode::F32Eq => Validator::validate_relop(context, ValueType::F32.into()),
			&Opcode::F32Ne => Validator::validate_relop(context, ValueType::F32.into()),
			&Opcode::F32Lt => Validator::validate_relop(context, ValueType::F32.into()),
			&Opcode::F32Gt => Validator::validate_relop(context, ValueType::F32.into()),
			&Opcode::F32Le => Validator::validate_relop(context, ValueType::F32.into()),
			&Opcode::F32Ge => Validator::validate_relop(context, ValueType::F32.into()),

			&Opcode::F64Eq => Validator::validate_relop(context, ValueType::F64.into()),
			&Opcode::F64Ne => Validator::validate_relop(context, ValueType::F64.into()),
			&Opcode::F64Lt => Validator::validate_relop(context, ValueType::F64.into()),
			&Opcode::F64Gt => Validator::validate_relop(context, ValueType::F64.into()),
			&Opcode::F64Le => Validator::validate_relop(context, ValueType::F64.into()),
			&Opcode::F64Ge => Validator::validate_relop(context, ValueType::F64.into()),

			&Opcode::I32Clz => Validator::validate_unop(context, ValueType::I32.into()),
			&Opcode::I32Ctz => Validator::validate_unop(context, ValueType::I32.into()),
			&Opcode::I32Popcnt => Validator::validate_unop(context, ValueType::I32.into()),
			&Opcode::I32Add => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32Sub => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32Mul => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32DivS => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32DivU => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32RemS => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32RemU => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32And => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32Or => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32Xor => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32Shl => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32ShrS => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32ShrU => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32Rotl => Validator::validate_binop(context, ValueType::I32.into()),
			&Opcode::I32Rotr => Validator::validate_binop(context, ValueType::I32.into()),

			&Opcode::I64Clz => Validator::validate_unop(context, ValueType::I64.into()),
			&Opcode::I64Ctz => Validator::validate_unop(context, ValueType::I64.into()),
			&Opcode::I64Popcnt => Validator::validate_unop(context, ValueType::I64.into()),
			&Opcode::I64Add => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64Sub => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64Mul => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64DivS => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64DivU => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64RemS => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64RemU => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64And => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64Or => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64Xor => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64Shl => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64ShrS => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64ShrU => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64Rotl => Validator::validate_binop(context, ValueType::I64.into()),
			&Opcode::I64Rotr => Validator::validate_binop(context, ValueType::I64.into()),

			&Opcode::F32Abs => Validator::validate_unop(context, ValueType::F32.into()),
			&Opcode::F32Neg => Validator::validate_unop(context, ValueType::F32.into()),
			&Opcode::F32Ceil => Validator::validate_unop(context, ValueType::F32.into()),
			&Opcode::F32Floor => Validator::validate_unop(context, ValueType::F32.into()),
			&Opcode::F32Trunc => Validator::validate_unop(context, ValueType::F32.into()),
			&Opcode::F32Nearest => Validator::validate_unop(context, ValueType::F32.into()),
			&Opcode::F32Sqrt => Validator::validate_unop(context, ValueType::F32.into()),
			&Opcode::F32Add => Validator::validate_binop(context, ValueType::F32.into()),
			&Opcode::F32Sub => Validator::validate_binop(context, ValueType::F32.into()),
			&Opcode::F32Mul => Validator::validate_binop(context, ValueType::F32.into()),
			&Opcode::F32Div => Validator::validate_binop(context, ValueType::F32.into()),
			&Opcode::F32Min => Validator::validate_binop(context, ValueType::F32.into()),
			&Opcode::F32Max => Validator::validate_binop(context, ValueType::F32.into()),
			&Opcode::F32Copysign => Validator::validate_binop(context, ValueType::F32.into()),

			&Opcode::F64Abs => Validator::validate_unop(context, ValueType::F64.into()),
			&Opcode::F64Neg => Validator::validate_unop(context, ValueType::F64.into()),
			&Opcode::F64Ceil => Validator::validate_unop(context, ValueType::F64.into()),
			&Opcode::F64Floor => Validator::validate_unop(context, ValueType::F64.into()),
			&Opcode::F64Trunc => Validator::validate_unop(context, ValueType::F64.into()),
			&Opcode::F64Nearest => Validator::validate_unop(context, ValueType::F64.into()),
			&Opcode::F64Sqrt => Validator::validate_unop(context, ValueType::F64.into()),
			&Opcode::F64Add => Validator::validate_binop(context, ValueType::F64.into()),
			&Opcode::F64Sub => Validator::validate_binop(context, ValueType::F64.into()),
			&Opcode::F64Mul => Validator::validate_binop(context, ValueType::F64.into()),
			&Opcode::F64Div => Validator::validate_binop(context, ValueType::F64.into()),
			&Opcode::F64Min => Validator::validate_binop(context, ValueType::F64.into()),
			&Opcode::F64Max => Validator::validate_binop(context, ValueType::F64.into()),
			&Opcode::F64Copysign => Validator::validate_binop(context, ValueType::F64.into()),

			&Opcode::I32WarpI64 => Validator::validate_cvtop(context, ValueType::I64.into(), ValueType::I32.into()),
			&Opcode::I32TruncSF32 => Validator::validate_cvtop(context, ValueType::F32.into(), ValueType::I32.into()),
			&Opcode::I32TruncUF32 => Validator::validate_cvtop(context, ValueType::F32.into(), ValueType::I32.into()),
			&Opcode::I32TruncSF64 => Validator::validate_cvtop(context, ValueType::F64.into(), ValueType::I32.into()),
			&Opcode::I32TruncUF64 => Validator::validate_cvtop(context, ValueType::F64.into(), ValueType::I32.into()),
			&Opcode::I64ExtendSI32 => Validator::validate_cvtop(context, ValueType::I32.into(), ValueType::I64.into()),
			&Opcode::I64ExtendUI32 => Validator::validate_cvtop(context, ValueType::I32.into(), ValueType::I64.into()),
			&Opcode::I64TruncSF32 => Validator::validate_cvtop(context, ValueType::F32.into(), ValueType::I64.into()),
			&Opcode::I64TruncUF32 => Validator::validate_cvtop(context, ValueType::F32.into(), ValueType::I64.into()),
			&Opcode::I64TruncSF64 => Validator::validate_cvtop(context, ValueType::F64.into(), ValueType::I64.into()),
			&Opcode::I64TruncUF64 => Validator::validate_cvtop(context, ValueType::F64.into(), ValueType::I64.into()),
			&Opcode::F32ConvertSI32 => Validator::validate_cvtop(context, ValueType::I32.into(), ValueType::F32.into()),
			&Opcode::F32ConvertUI32 => Validator::validate_cvtop(context, ValueType::I32.into(), ValueType::F32.into()),
			&Opcode::F32ConvertSI64 => Validator::validate_cvtop(context, ValueType::I64.into(), ValueType::F32.into()),
			&Opcode::F32ConvertUI64 => Validator::validate_cvtop(context, ValueType::I64.into(), ValueType::F32.into()),
			&Opcode::F32DemoteF64 => Validator::validate_cvtop(context, ValueType::F64.into(), ValueType::F32.into()),
			&Opcode::F64ConvertSI32 => Validator::validate_cvtop(context, ValueType::I32.into(), ValueType::F64.into()),
			&Opcode::F64ConvertUI32 => Validator::validate_cvtop(context, ValueType::I32.into(), ValueType::F64.into()),
			&Opcode::F64ConvertSI64 => Validator::validate_cvtop(context, ValueType::I64.into(), ValueType::F64.into()),
			&Opcode::F64ConvertUI64 => Validator::validate_cvtop(context, ValueType::I64.into(), ValueType::F64.into()),
			&Opcode::F64PromoteF32 => Validator::validate_cvtop(context, ValueType::F32.into(), ValueType::F64.into()),

			&Opcode::I32ReinterpretF32 => Validator::validate_cvtop(context, ValueType::F32.into(), ValueType::I32.into()),
			&Opcode::I64ReinterpretF64 => Validator::validate_cvtop(context, ValueType::F64.into(), ValueType::I64.into()),
			&Opcode::F32ReinterpretI32 => Validator::validate_cvtop(context, ValueType::I32.into(), ValueType::F32.into()),
			&Opcode::F64ReinterpretI64 => Validator::validate_cvtop(context, ValueType::I64.into(), ValueType::F64.into()),
		}
	}

	fn validate_const(context: &mut FunctionValidationContext, value_type: StackValueType) -> Result<InstructionOutcome, Error> {
		context.push_value(value_type)?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_unop(context: &mut FunctionValidationContext, value_type: StackValueType) -> Result<InstructionOutcome, Error> {
		context.pop_value(value_type)?;
		context.push_value(value_type)?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_binop(context: &mut FunctionValidationContext, value_type: StackValueType) -> Result<InstructionOutcome, Error> {
		context.pop_value(value_type)?;
		context.pop_value(value_type)?;
		context.push_value(value_type)?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_testop(context: &mut FunctionValidationContext, value_type: StackValueType) -> Result<InstructionOutcome, Error> {
		context.pop_value(value_type)?;
		context.push_value(ValueType::I32.into())?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_relop(context: &mut FunctionValidationContext, value_type: StackValueType) -> Result<InstructionOutcome, Error> {
		context.pop_value(value_type)?;
		context.pop_value(value_type)?;
		context.push_value(ValueType::I32.into())?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_cvtop(context: &mut FunctionValidationContext, value_type1: StackValueType, value_type2: StackValueType) -> Result<InstructionOutcome, Error> {
		context.pop_value(value_type1)?;
		context.push_value(value_type2)?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_drop(context: &mut FunctionValidationContext) -> Result<InstructionOutcome, Error> {
		context.pop_any_value().map(|_| ())?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_select(context: &mut FunctionValidationContext) -> Result<InstructionOutcome, Error> {
		context.pop_value(ValueType::I32.into())?;
		let select_type = context.pop_any_value()?;
		context.pop_value(select_type)?;
		context.push_value(select_type)?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_get_local(context: &mut FunctionValidationContext, index: u32) -> Result<InstructionOutcome, Error> {
		let local_type = context.require_local(index)?;
		context.push_value(local_type)?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_set_local(context: &mut FunctionValidationContext, index: u32) -> Result<InstructionOutcome, Error> {
		let local_type = context.require_local(index)?;
		let value_type = context.pop_any_value()?;
		if local_type != value_type {
			return Err(Error::Validation(format!("Trying to update local {} of type {:?} with value of type {:?}", index, local_type, value_type)));
		}
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_tee_local(context: &mut FunctionValidationContext, index: u32) -> Result<InstructionOutcome, Error> {
		let local_type = context.require_local(index)?;
		let value_type = context.tee_any_value()?;
		if local_type != value_type {
			return Err(Error::Validation(format!("Trying to update local {} of type {:?} with value of type {:?}", index, local_type, value_type)));
		}
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_get_global(context: &mut FunctionValidationContext, index: u32) -> Result<InstructionOutcome, Error> {
		let global_type = context.require_global(index, None)?;
		context.push_value(global_type)?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_set_global(context: &mut FunctionValidationContext, index: u32) -> Result<InstructionOutcome, Error> {
		let global_type = context.require_global(index, Some(true))?;
		let value_type = context.pop_any_value()?;
		if global_type != value_type {
			return Err(Error::Validation(format!("Trying to update global {} of type {:?} with value of type {:?}", index, global_type, value_type)));
		}
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_load(context: &mut FunctionValidationContext, align: u32, max_align: u32, value_type: StackValueType) -> Result<InstructionOutcome, Error> {
		if align != NATURAL_ALIGNMENT {
			if 1u32.checked_shl(align).unwrap_or(u32::MAX) > max_align {
				return Err(Error::Validation(format!("Too large memory alignment 2^{} (expected at most {})", align, max_align)));
			}
		}

		context.pop_value(ValueType::I32.into())?;
		context.require_memory(DEFAULT_MEMORY_INDEX)?;
		context.push_value(value_type)?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_store(context: &mut FunctionValidationContext, align: u32, max_align: u32, value_type: StackValueType) -> Result<InstructionOutcome, Error> {
		if align != NATURAL_ALIGNMENT {
			if 1u32.checked_shl(align).unwrap_or(u32::MAX) > max_align {
				return Err(Error::Validation(format!("Too large memory alignment 2^{} (expected at most {})", align, max_align)));
			}
		}

		context.require_memory(DEFAULT_MEMORY_INDEX)?;
		context.pop_value(value_type)?;
		context.pop_value(ValueType::I32.into())?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_block(context: &mut FunctionValidationContext, block_type: BlockType) -> Result<InstructionOutcome, Error> {
		context.push_label(BlockFrameType::Block, block_type).map(|_| InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_loop(context: &mut FunctionValidationContext, block_type: BlockType) -> Result<InstructionOutcome, Error> {
		context.push_label(BlockFrameType::Loop, block_type).map(|_| InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_if(context: &mut FunctionValidationContext, block_type: BlockType) -> Result<InstructionOutcome, Error> {
		context.pop_value(ValueType::I32.into())?;
		context.push_label(BlockFrameType::IfTrue, block_type).map(|_| InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_else(context: &mut FunctionValidationContext) -> Result<InstructionOutcome, Error> {
		let block_type = {
			let top_frame = context.top_label()?;
			if top_frame.frame_type != BlockFrameType::IfTrue {
				return Err(Error::Validation("Misplaced else instruction".into()));
			}
			top_frame.block_type
		};
		context.pop_label()?;

		if let BlockType::Value(value_type) = block_type {
			context.pop_value(value_type.into())?;
		}
		context.push_label(BlockFrameType::IfFalse, block_type).map(|_| InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_end(context: &mut FunctionValidationContext) -> Result<InstructionOutcome, Error> {
		{
			let top_frame = context.top_label()?;
			if top_frame.frame_type == BlockFrameType::IfTrue {
				if top_frame.block_type != BlockType::NoResult {
					return Err(Error::Validation(format!("If block without else required to have NoResult block type. But it have {:?} type", top_frame.block_type)));
				}
			}
		}

		context.pop_label().map(|_| InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_br(context: &mut FunctionValidationContext, idx: u32) -> Result<InstructionOutcome, Error> {
		let (frame_type, frame_block_type) = {
			let frame = context.require_label(idx)?;
			(frame.frame_type, frame.block_type)
		};
		if frame_type != BlockFrameType::Loop {
			if let BlockType::Value(value_type) = frame_block_type {
				context.tee_value(value_type.into())?;
			}
		}
		Ok(InstructionOutcome::Unreachable)
	}

	fn validate_br_if(context: &mut FunctionValidationContext, idx: u32) -> Result<InstructionOutcome, Error> {
		context.pop_value(ValueType::I32.into())?;

		let (frame_type, frame_block_type) = {
			let frame = context.require_label(idx)?;
			(frame.frame_type, frame.block_type)
		};
		if frame_type != BlockFrameType::Loop {
			if let BlockType::Value(value_type) = frame_block_type {
				context.tee_value(value_type.into())?;
			}
		}
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_br_table(context: &mut FunctionValidationContext, table: &Vec<u32>, default: u32) -> Result<InstructionOutcome, Error> {
		let mut required_block_type = None;

		{
			let default_block = context.require_label(default)?;
			if default_block.frame_type != BlockFrameType::Loop {
				required_block_type = Some(default_block.block_type);
			}

			for label in table {
				let label_block = context.require_label(*label)?;
				if label_block.frame_type != BlockFrameType::Loop {
					if let Some(required_block_type) = required_block_type {
						if required_block_type != label_block.block_type {
							return Err(Error::Validation(format!("Labels in br_table points to block of different types: {:?} and {:?}", required_block_type, label_block.block_type)));
						}
					}
					required_block_type = Some(label_block.block_type);
				}
			}
		}

		context.pop_value(ValueType::I32.into())?;
		if let Some(required_block_type) = required_block_type {
			if let BlockType::Value(value_type) = required_block_type {
				context.tee_value(value_type.into())?;
			}
		}

		Ok(InstructionOutcome::Unreachable)
	}

	fn validate_return(context: &mut FunctionValidationContext) -> Result<InstructionOutcome, Error> {
		if let BlockType::Value(value_type) = context.return_type()? {
			context.tee_value(value_type.into())?;
		}
		Ok(InstructionOutcome::Unreachable)
	}

	fn validate_call(context: &mut FunctionValidationContext, idx: u32) -> Result<InstructionOutcome, Error> {
		let (argument_types, return_type) = context.require_function(idx)?;
		for argument_type in argument_types.iter().rev() {
			context.pop_value((*argument_type).into())?;
		}
		if let BlockType::Value(value_type) = return_type {
			context.push_value(value_type.into())?;
		}
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_call_indirect(context: &mut FunctionValidationContext, idx: u32) -> Result<InstructionOutcome, Error> {
		context.require_table(DEFAULT_TABLE_INDEX, VariableType::AnyFunc)?;

		context.pop_value(ValueType::I32.into())?;
		let (argument_types, return_type) = context.require_function_type(idx)?;
		for argument_type in argument_types.iter().rev() {
			context.pop_value((*argument_type).into())?;
		}
		if let BlockType::Value(value_type) = return_type {
			context.push_value(value_type.into())?;
		}
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_current_memory(context: &mut FunctionValidationContext) -> Result<InstructionOutcome, Error> {
		context.require_memory(DEFAULT_MEMORY_INDEX)?;
		context.push_value(ValueType::I32.into())?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	fn validate_grow_memory(context: &mut FunctionValidationContext) -> Result<InstructionOutcome, Error> {
		context.require_memory(DEFAULT_MEMORY_INDEX)?;
		context.pop_value(ValueType::I32.into())?;
		context.push_value(ValueType::I32.into())?;
		Ok(InstructionOutcome::ValidateNextInstruction)
	}
}

impl<'a> FunctionValidationContext<'a> {
	pub fn new(
		module_instance: &'a ModuleInstance,
		externals: Option<&'a HashMap<String, Arc<ModuleInstanceInterface + 'a>>>,
		locals: &'a [ValueType],
		value_stack_limit: usize,
		frame_stack_limit: usize,
		function: FunctionSignature,
	) -> Self {
		FunctionValidationContext {
			module_instance: module_instance,
			externals: externals,
			position: 0,
			locals: locals,
			value_stack: StackWithLimit::with_limit(value_stack_limit),
			frame_stack: StackWithLimit::with_limit(frame_stack_limit),
			return_type: Some(function.return_type().map(BlockType::Value).unwrap_or(BlockType::NoResult)),
			labels: HashMap::new(),
		}
	}

	pub fn push_value(&mut self, value_type: StackValueType) -> Result<(), Error> {
		Ok(self.value_stack.push(value_type.into())?)
	}

	pub fn pop_value(&mut self, value_type: StackValueType) -> Result<(), Error> {
		self.check_stack_access()?;
		match self.value_stack.pop()? {
			StackValueType::Specific(stack_value_type) if stack_value_type == value_type => Ok(()),
			StackValueType::Any => Ok(()),
			StackValueType::AnyUnlimited => {
				self.value_stack.push(StackValueType::AnyUnlimited)?;
				Ok(())
			},
			stack_value_type @ _ => Err(Error::Validation(format!("Expected value of type {:?} on top of stack. Got {:?}", value_type, stack_value_type))),
		}
	}

	pub fn tee_value(&mut self, value_type: StackValueType) -> Result<(), Error> {
		self.check_stack_access()?;
		match *self.value_stack.top()? {
			StackValueType::Specific(stack_value_type) if stack_value_type == value_type => Ok(()),
			StackValueType::Any | StackValueType::AnyUnlimited => Ok(()),
			stack_value_type @ _ => Err(Error::Validation(format!("Expected value of type {:?} on top of stack. Got {:?}", value_type, stack_value_type))),
		}
	}

	pub fn pop_any_value(&mut self) -> Result<StackValueType, Error> {
		self.check_stack_access()?;
		match self.value_stack.pop()? {
			StackValueType::Specific(stack_value_type) => Ok(StackValueType::Specific(stack_value_type)),
			StackValueType::Any => Ok(StackValueType::Any),
			StackValueType::AnyUnlimited => {
				self.value_stack.push(StackValueType::AnyUnlimited)?;
				Ok(StackValueType::Any)
			},
		}
	}

	pub fn tee_any_value(&mut self) -> Result<StackValueType, Error> {
		self.check_stack_access()?;
		Ok(self.value_stack.top().map(Clone::clone)?)
	}

	pub fn unreachable(&mut self) -> Result<(), Error> {
		Ok(self.value_stack.push(StackValueType::AnyUnlimited)?)
	}

	pub fn top_label(&self) -> Result<&BlockFrame, Error> {
		Ok(self.frame_stack.top()?)
	}

	pub fn push_label(&mut self, frame_type: BlockFrameType, block_type: BlockType) -> Result<(), Error> {
		Ok(self.frame_stack.push(BlockFrame {
			frame_type: frame_type,
			block_type: block_type,
			begin_position: self.position,
			branch_position: self.position,
			end_position: self.position,
			value_stack_len: self.value_stack.len(),
		})?)
	}

	pub fn pop_label(&mut self) -> Result<InstructionOutcome, Error> {
		let frame = self.frame_stack.pop()?;
		let actual_value_type = if self.value_stack.len() > frame.value_stack_len {
			Some(self.value_stack.pop()?)
		} else {
			None
		};
		self.value_stack.resize(frame.value_stack_len, StackValueType::Any);

		match frame.block_type {
			BlockType::NoResult if actual_value_type.map(|vt| vt.is_any_unlimited()).unwrap_or(true) => (),
			BlockType::Value(required_value_type) if actual_value_type.map(|vt| vt == required_value_type).unwrap_or(false) => (),
			_ => return Err(Error::Validation(format!("Expected block to return {:?} while it has returned {:?}", frame.block_type, actual_value_type))),
		}
		if !self.frame_stack.is_empty() {
			self.labels.insert(frame.begin_position, self.position);
		}
		if let BlockType::Value(value_type) = frame.block_type {
			self.push_value(value_type.into())?;
		}

		Ok(InstructionOutcome::ValidateNextInstruction)
	}

	pub fn require_label(&self, idx: u32) -> Result<&BlockFrame, Error> {
		Ok(self.frame_stack.get(idx as usize)?)
	}

	pub fn return_type(&self) -> Result<BlockType, Error> {
		self.return_type.ok_or(Error::Validation("Trying to return from expression".into()))
	}

	pub fn require_local(&self, idx: u32) -> Result<StackValueType, Error> {
		self.locals.get(idx as usize)
			.cloned()
			.map(Into::into)
			.ok_or(Error::Validation(format!("Trying to access local with index {} when there are only {} locals", idx, self.locals.len())))
	}

	pub fn require_global(&self, idx: u32, mutability: Option<bool>) -> Result<StackValueType, Error> {
		self.module_instance
			.global(ItemIndex::IndexSpace(idx), None, self.externals.clone())
			.and_then(|g| match mutability {
				Some(true) if !g.is_mutable() => Err(Error::Validation(format!("Expected global {} to be mutable", idx))),
				Some(false) if g.is_mutable() => Err(Error::Validation(format!("Expected global {} to be immutable", idx))),
				_ => match g.variable_type() {
					VariableType::AnyFunc => Err(Error::Validation(format!("Expected global {} to have non-AnyFunc type", idx))),
					VariableType::I32 => Ok(StackValueType::Specific(ValueType::I32)),
					VariableType::I64 => Ok(StackValueType::Specific(ValueType::I64)),
					VariableType::F32 => Ok(StackValueType::Specific(ValueType::F32)),
					VariableType::F64 => Ok(StackValueType::Specific(ValueType::F64)),
				}
			})
	}

	pub fn require_memory(&self, idx: u32) -> Result<(), Error> {
		self.module_instance
			.memory(ItemIndex::IndexSpace(idx))
			.map(|_| ())
	}

	pub fn require_table(&self, idx: u32, variable_type: VariableType) -> Result<(), Error> {
		self.module_instance
			.table(ItemIndex::IndexSpace(idx))
			.and_then(|t| if t.variable_type() == variable_type {
				Ok(())
			} else {
				Err(Error::Validation(format!("Table {} has element type {:?} while {:?} expected", idx, t.variable_type(), variable_type)))
			})
	}

	pub fn require_function(&self, idx: u32) -> Result<(Vec<ValueType>, BlockType), Error> {
		self.module_instance.function_type(ItemIndex::IndexSpace(idx))
			.map(|ft| (ft.params().to_vec(), ft.return_type().map(BlockType::Value).unwrap_or(BlockType::NoResult)))
	}

	pub fn require_function_type(&self, idx: u32) -> Result<(Vec<ValueType>, BlockType), Error> {
		self.module_instance.function_type_by_index(idx)
			.map(|ft| (ft.params().to_vec(), ft.return_type().map(BlockType::Value).unwrap_or(BlockType::NoResult)))
	}

	pub fn function_labels(self) -> HashMap<usize, usize> {
		self.labels
	}

	fn check_stack_access(&self) -> Result<(), Error> {
		let value_stack_min = self.frame_stack.top().expect("at least 1 topmost block").value_stack_len;
		if self.value_stack.len() > value_stack_min {
			Ok(())
		} else {
			Err(Error::Validation("Trying to access parent frame stack values.".into()))
		}
	}
}

impl StackValueType {
	pub fn is_any(&self) -> bool {
		match self {
			&StackValueType::Any => true,
			_ => false,
		}
	}

	pub fn is_any_unlimited(&self) -> bool {
		match self {
			&StackValueType::AnyUnlimited => true,
			_ => false,
		}
	}

	pub fn value_type(&self) -> ValueType {
		match self {
			&StackValueType::Any | &StackValueType::AnyUnlimited => unreachable!("must be checked by caller"),
			&StackValueType::Specific(value_type) => value_type,
		}
	}
}

impl From<ValueType> for StackValueType {
	fn from(value_type: ValueType) -> Self {
		StackValueType::Specific(value_type)
	}
}

impl PartialEq<StackValueType> for StackValueType {
	fn eq(&self, other: &StackValueType) -> bool {
		if self.is_any() || other.is_any() || self.is_any_unlimited() || other.is_any_unlimited() {
			true
		} else {
			self.value_type() == other.value_type()
		}
	}
}

impl PartialEq<ValueType> for StackValueType {
	fn eq(&self, other: &ValueType) -> bool {
		if self.is_any() || self.is_any_unlimited() {
			true
		} else {
			self.value_type() == *other
		}
	}
}

impl PartialEq<StackValueType> for ValueType {
	fn eq(&self, other: &StackValueType) -> bool {
		other == self
	}
}
