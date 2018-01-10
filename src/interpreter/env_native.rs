use std::sync::Arc;
use std::collections::HashMap;
use std::borrow::Cow;
use parking_lot::RwLock;
use elements::{Internal, ValueType};
use interpreter::{Error, UserError};
use interpreter::module::{ModuleInstanceInterface, ExecutionParams, ItemIndex,
	CallerContext, ExportEntryType, InternalFunctionReference, InternalFunction, FunctionSignature};
use interpreter::memory::MemoryInstance;
use interpreter::table::TableInstance;
use interpreter::value::RuntimeValue;
use interpreter::variable::{VariableInstance, VariableType};

/// Min index of native function.
pub const NATIVE_INDEX_FUNC_MIN: u32 = 10001;
/// Min index of native global.
pub const NATIVE_INDEX_GLOBAL_MIN: u32 = 20001;

/// User functions executor.
pub trait UserFunctionExecutor<E: UserError> {
	/// Execute function with given name.
	fn execute(&mut self, name: &str, context: CallerContext<E>) -> Result<Option<RuntimeValue>, Error<E>>;
}

/// User function descriptor
#[derive(Debug, Clone)]
pub enum UserFunctionDescriptor {
	/// Static function definition
	Static(&'static str, &'static [ValueType], Option<ValueType>),
	/// Dynamic heap function definition
	Heap(String, Vec<ValueType>, Option<ValueType>),
}

impl UserFunctionDescriptor {
	/// New function with statically known params
	pub fn statik(name: &'static str, params: &'static [ValueType], result: Option<ValueType>) -> Self {
		UserFunctionDescriptor::Static(name, params, result)
	}

	/// New function with statically unknown params
	pub fn heap(name: String, params: Vec<ValueType>, result: Option<ValueType>) -> Self {
		UserFunctionDescriptor::Heap(name, params, result)
	}

	/// Name of the function
	pub fn name(&self) -> &str {
		match self {
			&UserFunctionDescriptor::Static(name, _, _) => name,
			&UserFunctionDescriptor::Heap(ref name, _, _) => name,
		}
	}

	/// Arguments of the function
	pub fn params(&self) -> &[ValueType] {
		match self {
			&UserFunctionDescriptor::Static(_, params, _) => params,
			&UserFunctionDescriptor::Heap(_, ref params, _) => params,
		}
	}

	/// Return type of the function
	pub fn return_type(&self) -> Option<ValueType> {
		match self {
			&UserFunctionDescriptor::Static(_, _, result) => result,
			&UserFunctionDescriptor::Heap(_, _, result) => result,
		}
	}
}

/// Set of user-defined module elements.
pub struct UserDefinedElements<'a, E: 'a + UserError> {
	/// User globals list.
	pub globals: HashMap<String, Arc<VariableInstance<E>>>,
	/// User functions list.
	pub functions: Cow<'static, [UserFunctionDescriptor]>,
	/// Functions executor.
	pub executor: Option<&'a mut UserFunctionExecutor<E>>,
}

/// Native module instance.
pub struct NativeModuleInstance<'a, E: 'a + UserError> {
	/// Underllying module reference.
	env: Arc<ModuleInstanceInterface<E>>,
	/// User function executor.
	executor: RwLock<Option<&'a mut UserFunctionExecutor<E>>>,
	/// By-name functions index.
	functions_by_name: HashMap<String, u32>,
	/// User functions list.
	functions: Cow<'static, [UserFunctionDescriptor]>,
	/// By-name functions index.
	globals_by_name: HashMap<String, u32>,
	/// User globals list.
	globals: Vec<Arc<VariableInstance<E>>>,
}

impl<'a, E> NativeModuleInstance<'a, E> where E: UserError {
	/// Create new native module
	pub fn new(env: Arc<ModuleInstanceInterface<E>>, elements: UserDefinedElements<'a, E>) -> Result<Self, Error<E>> {
		if !elements.functions.is_empty() && elements.executor.is_none() {
			return Err(Error::Function("trying to construct native env module with functions, but without executor".into()));
		}

		Ok(NativeModuleInstance {
			env: env,
			executor: RwLock::new(elements.executor),
			functions_by_name: elements.functions.iter().enumerate().map(|(i, f)| (f.name().to_owned(), i as u32)).collect(),
			functions: elements.functions,
			globals_by_name: elements.globals.iter().enumerate().map(|(i, (g_name, _))| (g_name.to_owned(), i as u32)).collect(),
			globals: elements.globals.into_iter().map(|(_, g)| g).collect(),
		})
	}
}

impl<'a, E> ModuleInstanceInterface<E> for NativeModuleInstance<'a, E> where E: UserError {
	fn execute_index(&self, index: u32, params: ExecutionParams<E>) -> Result<Option<RuntimeValue>, Error<E>> {
		self.env.execute_index(index, params)
	}

	fn execute_export(&self, name: &str, params: ExecutionParams<E>) -> Result<Option<RuntimeValue>, Error<E>> {
		self.env.execute_export(name, params)
	}

	fn export_entry<'b>(&self, name: &str, required_type: &ExportEntryType) -> Result<Internal, Error<E>> {
		if let Some(index) = self.functions_by_name.get(name) {
			let composite_index = NATIVE_INDEX_FUNC_MIN + *index;
			match required_type {
				&ExportEntryType::Any => return Ok(Internal::Function(composite_index)),
				&ExportEntryType::Function(ref required_type)
					if self.function_type(ItemIndex::Internal(composite_index))
						.expect("by_name contains index; function_type succeeds for all functions from by_name; qed") == *required_type
					=> return Ok(Internal::Function(composite_index)),
				_ => (),
			}
		}
		if let Some(index) = self.globals_by_name.get(name) {
			match required_type {
				&ExportEntryType::Any => return Ok(Internal::Global(NATIVE_INDEX_GLOBAL_MIN + *index)),
				&ExportEntryType::Global(ref required_type)
					if self.globals.get(*index as usize)
						.expect("globals_by_name maps to indexes of globals; index read from globals_by_name; qed")
						.variable_type() == *required_type
					=> return Ok(Internal::Global(NATIVE_INDEX_GLOBAL_MIN + *index)),
				_ => (),
			}
		}

		self.env.export_entry(name, required_type)
	}

	fn table(&self, index: ItemIndex) -> Result<Arc<TableInstance<E>>, Error<E>> {
		self.env.table(index)
	}

	fn memory(&self, index: ItemIndex) -> Result<Arc<MemoryInstance<E>>, Error<E>> {
		self.env.memory(index)
	}

	fn global<'b>(&self, global_index: ItemIndex, variable_type: Option<VariableType>, externals: Option<&'b HashMap<String, Arc<ModuleInstanceInterface<E> + 'b>>>) -> Result<Arc<VariableInstance<E>>, Error<E>> {
		let index = match global_index {
			ItemIndex::IndexSpace(index) | ItemIndex::Internal(index) => index,
			ItemIndex::External(_) => unreachable!("trying to get global, exported by native env module"),
		};

		if index < NATIVE_INDEX_GLOBAL_MIN {
			return self.env.global(global_index, variable_type, externals);
		}

		self.globals
			.get((index - NATIVE_INDEX_GLOBAL_MIN) as usize)
			.cloned()
			.ok_or(Error::Native(format!("trying to get native global with index {}", index)))
	}

	fn function_type(&self, function_index: ItemIndex) -> Result<FunctionSignature, Error<E>> {
		let index = match function_index {
			ItemIndex::IndexSpace(index) | ItemIndex::Internal(index) => index,
			ItemIndex::External(_) => unreachable!("trying to call function, exported by native env module"),
		};

		if index < NATIVE_INDEX_FUNC_MIN || index >= NATIVE_INDEX_GLOBAL_MIN {
			return self.env.function_type(function_index);
		}

		Ok(FunctionSignature::User(self.functions
			.get((index - NATIVE_INDEX_FUNC_MIN) as usize)
			.ok_or(Error::Native(format!("missing native env function with index {}", index)))?))
	}

	fn function_type_by_index(&self, type_index: u32) -> Result<FunctionSignature, Error<E>> {
		self.function_type(ItemIndex::Internal(type_index))
	}

	fn function_reference<'b>(&self, index: ItemIndex, externals: Option<&'b HashMap<String, Arc<ModuleInstanceInterface<E> + 'b>>>) -> Result<InternalFunctionReference<'b, E>, Error<E>> {
		self.env.function_reference(index, externals)
	}

	fn function_reference_indirect<'b>(&self, table_idx: u32, type_idx: u32, func_idx: u32, externals: Option<&'b HashMap<String, Arc<ModuleInstanceInterface<E> + 'b>>>) -> Result<InternalFunctionReference<'b, E>, Error<E>> {
		self.env.function_reference_indirect(table_idx, type_idx, func_idx, externals)
	}

	fn function_body<'b>(&'b self, _internal_index: u32) -> Result<Option<InternalFunction<'b>>, Error<E>> {
		Ok(None)
	}

	fn call_internal_function(&self, outer: CallerContext<E>, index: u32) -> Result<Option<RuntimeValue>, Error<E>> {
		if index < NATIVE_INDEX_FUNC_MIN || index >= NATIVE_INDEX_GLOBAL_MIN {
			return self.env.call_internal_function(outer, index);
		}

		self.functions
			.get((index - NATIVE_INDEX_FUNC_MIN) as usize)
			.ok_or(Error::Native(format!("trying to call native function with index {}", index)).into())
			.and_then(|f| self.executor.write()
				.as_mut()
				.expect("function existss; if function exists, executor must also exists [checked in constructor]; qed")
				.execute(&f.name(), outer))
	}
}

/// Create wrapper for env module with given native user functions.
pub fn env_native_module<'a, E: UserError>(env: Arc<ModuleInstanceInterface<E>>, user_elements: UserDefinedElements<'a, E>) -> Result<NativeModuleInstance<E>, Error<E>> {
	NativeModuleInstance::new(env, user_elements)
}

impl<'a> PartialEq for UserFunctionDescriptor {
	fn eq(&self, other: &Self) -> bool {
		self.params() == other.params()
			&& self.return_type() == other.return_type()
	}
}