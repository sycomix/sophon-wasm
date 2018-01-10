use std::io;
use super::{Deserialize, Serialize, Error, GlobalType, InitExpr};

/// Global entry in the module.
#[derive(Clone)]
pub struct GlobalEntry {
    global_type: GlobalType,
    init_expr: InitExpr,
}

impl GlobalEntry {
    /// New global entry
    pub fn new(global_type: GlobalType, init_expr: InitExpr) -> Self {
        GlobalEntry {
            global_type: global_type,
            init_expr: init_expr,
        }
    }
    /// Global type.
    pub fn global_type(&self) -> &GlobalType { &self.global_type }
    /// Initialization expression (opcodes) for global.
    pub fn init_expr(&self) -> &InitExpr { &self.init_expr }
    /// Global type (mutable)
    pub fn global_type_mut(&mut self) -> &mut GlobalType { &mut self.global_type }
    /// Initialization expression (opcodes) for global (mutable)
    pub fn init_expr_mut(&mut self) -> &mut InitExpr { &mut self.init_expr }
}

impl Deserialize for GlobalEntry {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let global_type = GlobalType::deserialize(reader)?;
        let init_expr = InitExpr::deserialize(reader)?;

        Ok(GlobalEntry {
            global_type: global_type,
            init_expr: init_expr,
        })
    }
}

impl Serialize for GlobalEntry {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        self.global_type.serialize(writer)?;
        self.init_expr.serialize(writer)
    }
}
