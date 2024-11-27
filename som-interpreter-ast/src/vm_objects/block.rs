use crate::ast::AstBlock;
use som_gc::gcref::Gc;
use std::fmt;

use crate::universe::Universe;
use crate::vm_objects::class::Class;
use crate::vm_objects::frame::Frame;

/// Represents an executable block.
#[derive(Clone)]
pub struct Block {
    /// Reference to the captured stack frame.
    pub frame: Gc<Frame>,
    /// Block definition from the AST.
    pub block: Gc<AstBlock>,
}

impl Block {
    /// Get the block's class.
    pub fn class(&self, universe: &Universe) -> Gc<Class> {
        match self.nb_parameters() {
            0 => universe.block1_class(),
            1 => universe.block2_class(),
            2 => universe.block3_class(),
            _ => panic!("no support for blocks with more than 2 parameters"),
        }
    }

    /// Retrieve the number of parameters this block accepts.
    pub fn nb_parameters(&self) -> u8 {
        self.block.nbr_params
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct(&format!("Block{}", self.nb_parameters() + 1))
            .field("block", &self.block)
            .field("frame", &self.frame)
            .finish()
    }
}
