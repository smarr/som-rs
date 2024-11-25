use crate::class::Class;
use crate::compiler::Literal;
use crate::frame::Frame;
// use crate::interner::Interned;
use crate::method::Method;
use crate::universe::Universe;
#[cfg(feature = "frame-debug-info")]
use som_core::ast::BlockDebugInfo;
use som_core::bytecode::Bytecode;
use som_gc::gcref::Gc;
use std::fmt;

// TODO - is that refcell still needed?
pub type BodyInlineCache = Vec<Option<(Gc<Class>, Gc<Method>)>>;

#[derive(Clone)]
pub struct BlockInfo {
    // pub locals: Vec<Interned>,
    pub literals: Vec<Literal>,
    pub body: Vec<Bytecode>,
    pub nb_locals: usize,
    pub nb_params: usize,
    pub inline_cache: BodyInlineCache,
    pub max_stack_size: u8,
    #[cfg(feature = "frame-debug-info")]
    pub block_debug_info: BlockDebugInfo,
}

/// Represents an executable block.
#[derive(Clone)]
pub struct Block {
    /// Reference to the captured stack frame.
    pub frame: Option<Gc<Frame>>,
    pub blk_info: Gc<BlockInfo>,
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
    pub fn nb_parameters(&self) -> usize {
        self.blk_info.nb_params
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct(&format!("Block{}", self.nb_parameters() + 1))
            .field("block", &self.blk_info)
            .field("frame", &self.frame.map(|f| f.ptr))
            .finish()
    }
}

impl fmt::Debug for BlockInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockInfo")
            .field("nbr_locals", &self.nb_locals)
            .field("nbr_params", &self.nb_params)
            .field("literals", &self.literals)
            .finish()
    }
}
