use super::invoke::{Invoke, Identity};
use elements;

pub struct ImportBuilder<F=Identity> {
    callback: F,
    module: String,
    field: String,
    binding: elements::External,
}

impl ImportBuilder {
    pub fn new() -> Self {
        ImportBuilder::with_callback(Identity)
    }
}

impl<F> ImportBuilder<F> {

    pub fn with_callback(callback: F) -> Self {
        ImportBuilder {
            callback: callback,
            module: String::new(),
            field: String::new(),
            binding: elements::External::Function(0),
        }
    }

    pub fn module(mut self, name: &str) -> Self {
        self.module = name.to_owned();
        self
    }

    pub fn field(mut self, name: &str) -> Self {
        self.field = name.to_owned();
        self
    }

    pub fn path(self, module: &str, field: &str) -> Self {
        self.module(module).field(field)
    }

    pub fn with_external(mut self, external: elements::External) -> Self {
        self.binding = external;
        self
    }

    pub fn external(self) -> ImportExternalBuilder<Self> {
        ImportExternalBuilder::with_callback(self)
    }
}

impl<F> ImportBuilder<F> where F: Invoke<elements::ImportEntry> {
    pub fn build(self) -> F::Result {
        self.callback.invoke(elements::ImportEntry::new(self.module, self.field, self.binding))
    }
}

impl<F> Invoke<elements::External> for ImportBuilder<F> {
    type Result = Self;
    fn invoke(self, val: elements::External) -> Self {
        self.with_external(val)
    }
}

pub struct ImportExternalBuilder<F=Identity> {
    callback: F,
    binding: elements::External,
}

impl<F> ImportExternalBuilder<F> where F: Invoke<elements::External> {
    pub fn with_callback(callback: F) -> Self {
        ImportExternalBuilder{
            callback: callback,
            binding: elements::External::Function(0),
        }
    }

    pub fn func(mut self, index: u32) -> F::Result {
        self.binding = elements::External::Function(index);
        self.callback.invoke(self.binding)
    }

    pub fn memory(mut self, min: u32, max: Option<u32>) -> F::Result {
        self.binding = elements::External::Memory(elements::MemoryType::new(min, max));
        self.callback.invoke(self.binding)
    }

    pub fn table(mut self, min: u32, max: Option<u32>) -> F::Result {
        self.binding = elements::External::Table(elements::TableType::new(min, max));
        self.callback.invoke(self.binding)
    }

    pub fn global(mut self, value_type: elements::ValueType, is_mut: bool) -> F::Result {
        self.binding = elements::External::Global(elements::GlobalType::new(value_type, is_mut));
        self.callback.invoke(self.binding)
    }
}

/// New builder for import entry
pub fn import() -> ImportBuilder {
    ImportBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::import;

    #[test]
    fn example() {
        let entry = import().module("env").field("memory").external().memory(256, Some(256)).build();

        assert_eq!(entry.module(), "env");
        assert_eq!(entry.field(), "memory");
    }
}