use elements;
use super::invoke::{Invoke, Identity};

#[derive(Debug)]
pub struct TableDefinition {
    pub min: u32,
    pub max: Option<u32>,
    pub elements: Vec<TableEntryDefinition>,
}

#[derive(Debug)]
pub struct TableEntryDefinition {
    pub offset: elements::InitExpr,
    pub values: Vec<u32>,
}

pub struct TableBuilder<F=Identity> {
    callback: F,
    table: TableDefinition,
}

impl TableBuilder {
    pub fn new() -> Self {
        TableBuilder::with_callback(Identity)
    }
}

impl<F> TableBuilder<F> where F: Invoke<TableDefinition> {
    pub fn with_callback(callback: F) -> Self {
        TableBuilder {
            callback: callback,
            table: Default::default(),
        }
    }

    pub fn with_min(mut self, min: u32) -> Self {
        self.table.min = min;
        self
    }

    pub fn with_max(mut self, max: Option<u32>) -> Self {
        self.table.max = max;
        self
    }

    pub fn with_element(mut self, index: u32, values: Vec<u32>) -> Self {
        self.table.elements.push(TableEntryDefinition {
            offset: elements::InitExpr::new(vec![
                elements::Opcode::I32Const(index as i32),
                elements::Opcode::End,
            ]),
            values: values,
        });
        self
    }

    pub fn build(self) -> F::Result {
        self.callback.invoke(self.table)
    }
}

impl Default for TableDefinition {
    fn default() -> Self {
        TableDefinition {
            min: 0,
            max: None,
            elements: Vec::new(),
        }
    }
}
