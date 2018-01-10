use super::invoke::{Invoke, Identity};
use super::misc::ValueTypeBuilder;
use elements;

pub struct GlobalBuilder<F=Identity> {
    callback: F,
    value_type: elements::ValueType,
    is_mutable: bool,
    init_expr: elements::InitExpr,
}

impl GlobalBuilder {
    pub fn new() -> Self {
        GlobalBuilder::with_callback(Identity)
    }
}

impl<F> GlobalBuilder<F> {
    pub fn with_callback(callback: F) -> Self {
        GlobalBuilder {
            callback: callback,
            value_type: elements::ValueType::I32,
            init_expr: elements::InitExpr::empty(),
            is_mutable: false,
        }
    }

    pub fn with_type(mut self, value_type: elements::ValueType) -> Self {
        self.value_type = value_type;
        self
    }

    pub fn mutable(mut self) -> Self {
        self.is_mutable = true;
        self
    }

    pub fn init_expr(mut self, opcode: elements::Opcode) -> Self {
        self.init_expr = elements::InitExpr::new(vec![opcode, elements::Opcode::End]);
        self
    }

    pub fn value_type(self) -> ValueTypeBuilder<Self> {
        ValueTypeBuilder::with_callback(self)
    }
}

impl<F> GlobalBuilder<F> where F: Invoke<elements::GlobalEntry> {
    pub fn build(self) -> F::Result {
        self.callback.invoke(
            elements::GlobalEntry::new(
                elements::GlobalType::new(self.value_type, self.is_mutable), 
                self.init_expr,
            )
        )
    }
}

impl<F> Invoke<elements::ValueType> for GlobalBuilder<F> {
    type Result = Self;
    fn invoke(self, the_type: elements::ValueType) -> Self {
        self.with_type(the_type)
    }
}

/// New builder for export entry
pub fn global() -> GlobalBuilder {
    GlobalBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::global;
    use elements;

    #[test]
    fn example() {
        let entry = global().value_type().i32().build();
        assert_eq!(entry.global_type().content_type(), elements::ValueType::I32);
        assert_eq!(entry.global_type().is_mutable(), false);
    }
}