//! This crate provides some of the simplest exports
//! from the Emscripten runtime, such as `STACKTOP` or `abort`.

extern crate sophon_wasm;

use std::sync::{Arc, Weak};
use sophon_wasm::builder::module;
use sophon_wasm::elements::{ExportEntry, Internal, ValueType};
use sophon_wasm::interpreter::Error;
use sophon_wasm::interpreter::{native_module, UserDefinedElements, UserFunctionDescriptor, UserFunctionExecutor};
use sophon_wasm::interpreter::{CallerContext, ModuleInstance, ModuleInstanceInterface};
use sophon_wasm::interpreter::RuntimeValue;
use sophon_wasm::interpreter::ProgramInstance;
use sophon_wasm::interpreter::{VariableInstance, VariableType};

/// Memory address, at which stack begins.
const DEFAULT_STACK_TOP: u32 = 256 * 1024;
/// Memory, allocated for stack.
const DEFAULT_TOTAL_STACK: u32 = 5 * 1024 * 1024;
/// Total memory, allocated by default.
const DEFAULT_TOTAL_MEMORY: u32 = 16 * 1024 * 1024;
/// Whether memory can be enlarged, or not.
const DEFAULT_ALLOW_MEMORY_GROWTH: bool = true;
/// Default tableBase variable value.
const DEFAULT_TABLE_BASE: u32 = 0;
/// Default tableBase variable value.
const DEFAULT_MEMORY_BASE: u32 = 0;

/// Default table size.
const DEFAULT_TABLE_SIZE: u32 = 64;

/// Index of default memory.
const INDEX_MEMORY: u32 = 0;

/// Index of default table.
const INDEX_TABLE: u32 = 0;

const LINEAR_MEMORY_PAGE_SIZE: u32 = 64 * 1024;

/// Emscripten environment parameters.
pub struct EmscriptenParams {
	/// Stack size in bytes.
	pub total_stack: u32,
	/// Total memory size in bytes.
	pub total_memory: u32,
	/// Allow memory growth.
	pub allow_memory_growth: bool,
	/// Table size.
	pub table_size: u32,
	/// Static reserve, if any
	pub static_size: Option<u32>,
}

struct EmscriptenFunctionExecutor {
	total_mem_global: Arc<VariableInstance>,
}

impl<'a> UserFunctionExecutor for EmscriptenFunctionExecutor {
	fn execute(
		&mut self,
		name: &str,
		context: CallerContext,
	) -> Result<Option<RuntimeValue>, Error> {
		match name {
			"_abort" | "abort" => {
				Err(Error::Trap("abort".into()).into())
			},
			"assert" => {
				let condition = context.value_stack.pop_as::<i32>()?;
				if condition == 0 {
					Err(Error::Trap("assertion failed".into()))
				} else {
					Ok(None)
				}
			},
			"enlargeMemory" => {
				// TODO: support memory enlarge
				Ok(Some(RuntimeValue::I32(0)))
			},
			"getTotalMemory" => {
				let total_memory = self.total_mem_global.get();
				Ok(Some(total_memory))
			},
			_ => Err(Error::Trap("not implemented".into()).into()),
		}
	}
}

pub fn env_module(params: EmscriptenParams) -> Result<Arc<ModuleInstanceInterface>, Error> {
	debug_assert!(params.total_stack < params.total_memory);
	debug_assert!((params.total_stack % LINEAR_MEMORY_PAGE_SIZE) == 0);
	debug_assert!((params.total_memory % LINEAR_MEMORY_PAGE_SIZE) == 0);

	let stack_top = params.static_size.unwrap_or(DEFAULT_STACK_TOP);

	// Build module with defined memory and table,
	// instantiate it and wrap into an Arc.
	let instance = {
		let builder = module()
			// memory regions
			.memory()
				.with_min(params.total_memory / LINEAR_MEMORY_PAGE_SIZE)
				.with_max(params.max_memory().map(|m| m / LINEAR_MEMORY_PAGE_SIZE))
				.build()
				.with_export(ExportEntry::new("memory".into(), Internal::Memory(INDEX_MEMORY)))
			// tables
			.table()
				.with_min(params.table_size)
				.build()
				.with_export(ExportEntry::new("table".into(), Internal::Table(INDEX_TABLE)));
		let mut instance = ModuleInstance::new(Weak::default(), "env".into(), builder.build())?;
		instance.instantiate(None)?;
		Arc::new(instance)
	};
	let total_mem_global = Arc::new(
		VariableInstance::new(
			false,
			VariableType::I32,
			RuntimeValue::I32(params.total_memory as i32),
		).unwrap(),
	);

	let function_executor = EmscriptenFunctionExecutor {
		total_mem_global: Arc::clone(&total_mem_global),
	};

	const SIGNATURES: &'static [UserFunctionDescriptor] = &[
		UserFunctionDescriptor::Static("_abort", &[], None),
		UserFunctionDescriptor::Static("abort", &[], None),
		UserFunctionDescriptor::Static("assert", &[ValueType::I32], None),
		UserFunctionDescriptor::Static("enlargeMemory", &[], Some(ValueType::I32)),
		UserFunctionDescriptor::Static("getTotalMemory", &[], Some(ValueType::I32)),
	];

	let elements = UserDefinedElements {
		executor: Some(function_executor),
		globals: vec![
			(
				"STACK_BASE".into(),
				Arc::new(
					VariableInstance::new(
						false,
						VariableType::I32,
						RuntimeValue::I32(stack_top as i32),
					).unwrap(),
				),
			),
			(
				"STACKTOP".into(),
				Arc::new(
					VariableInstance::new(
						false,
						VariableType::I32,
						RuntimeValue::I32(stack_top as i32),
					).unwrap(),
				),
			),
			(
				"STACK_MAX".into(),
				Arc::new(
					VariableInstance::new(
						false,
						VariableType::I32,
						RuntimeValue::I32((stack_top + params.total_stack) as i32),
					).unwrap(),
				),
			),
			(
				"DYNAMIC_BASE".into(),
				Arc::new(
					VariableInstance::new(
						false,
						VariableType::I32,
						RuntimeValue::I32((stack_top + params.total_stack) as i32),
					).unwrap(),
				),
			),
			(
				"DYNAMICTOP_PTR".into(),
				Arc::new(
					VariableInstance::new(
						false,
						VariableType::I32,
						RuntimeValue::I32((stack_top + params.total_stack) as i32),
					).unwrap(),
				),
			),
			(
				"EXITSTATUS".into(),
				Arc::new(
					VariableInstance::new(false, VariableType::I32, RuntimeValue::I32(0)).unwrap(),
				),
			),
			(
				"tableBase".into(),
				Arc::new(
					VariableInstance::new(
						false,
						VariableType::I32,
						RuntimeValue::I32(DEFAULT_TABLE_BASE as i32),
					).unwrap(),
				),
			),
			(
				"memoryBase".into(),
				Arc::new(
					VariableInstance::new(
						false,
						VariableType::I32,
						RuntimeValue::I32(DEFAULT_MEMORY_BASE as i32),
					).unwrap(),
				),
			),
			("TOTAL_MEMORY".into(), total_mem_global),
		].into_iter()
			.collect(),
		functions: ::std::borrow::Cow::from(SIGNATURES),
	};

	Ok(native_module(instance, elements)?)
}

impl Default for EmscriptenParams {
	fn default() -> Self {
		EmscriptenParams {
			total_stack: DEFAULT_TOTAL_STACK,
			total_memory: DEFAULT_TOTAL_MEMORY,
			allow_memory_growth: DEFAULT_ALLOW_MEMORY_GROWTH,
			table_size: DEFAULT_TABLE_SIZE,
			static_size: None,
		}
	}
}

impl EmscriptenParams {
	fn max_memory(&self) -> Option<u32> {
		if self.allow_memory_growth { None } else { Some(self.total_memory) }
	}
}

pub fn program_with_emscripten_env(params: EmscriptenParams) -> Result<ProgramInstance, Error> {
	let program = ProgramInstance::new();
	program.insert_loaded_module("env", env_module(params)?)?;
	Ok(program)
}

#[cfg(test)]
mod tests {
	use super::program_with_emscripten_env;
	use sophon_wasm::builder::module;
	use sophon_wasm::elements::{ImportEntry, External, GlobalType, ValueType};

	#[test]
	fn import_env_mutable_global() {
		let program = program_with_emscripten_env(Default::default()).unwrap();

		let module = module()
			.with_import(ImportEntry::new("env".into(), "STACKTOP".into(), External::Global(GlobalType::new(ValueType::I32, false))))
			.build();

		program.add_module("main", module, None).unwrap();
	}
}
