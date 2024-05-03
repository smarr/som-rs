use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
#[cfg(feature = "block-dbg-info")]
use som_core::ast::BlockDebugInfo;

use som_core::bytecode::Bytecode;

use crate::class::Class;
use crate::compiler::Literal;
use crate::frame::Frame;
// use crate::interner::Interned;
use crate::method::Method;
use crate::universe::Universe;
use crate::SOMRef;

#[derive(Clone)]
pub struct BlockInfo {
    // pub locals: Vec<Interned>,
    pub literals: Vec<Literal>,
    pub body: Vec<Bytecode>,
    pub nb_locals: usize,
    pub nb_params: usize,
    pub inline_cache: RefCell<Vec<Option<(*const Class, Rc<Method>)>>>,
    #[cfg(feature = "block-dbg-info")]
    pub block_debug_info: BlockDebugInfo,
}

/// Represents an executable block.
#[derive(Clone)]
pub struct Block {
    /// Reference to the captured stack frame.
    pub frame: Option<SOMRef<Frame>>,
    pub blk_info: Rc<BlockInfo>,
}

impl Block {
    /// Get the block's class.
    pub fn class(&self, universe: &Universe) -> SOMRef<Class> {
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

    pub fn make_equivalent_with_no_return(&self) -> Rc<Block> {
        Rc::from(Block {
            frame: self.frame.clone(),
            blk_info: Rc::new(BlockInfo {
                literals: self.blk_info.literals.clone(),
                body: {
                    let og_body = &self.blk_info.body;
                    let mut new_body = og_body.clone();

                    let non_local_rets_idx: Vec<usize> = new_body.iter().enumerate().filter_map(|(idx, bc)|
                        match bc {
                            Bytecode::ReturnNonLocal(_) => Some(idx),
                            _ => None
                        }
                    ).collect();

                    if !non_local_rets_idx.is_empty() {
                        for idx in &non_local_rets_idx {
                            new_body.insert(*idx, Bytecode::Pop);
                        }

                        for (bc_idx, bc) in &mut new_body.iter_mut().enumerate() {
                            match bc {
                                Bytecode::Jump(jump_idx)
                                | Bytecode::JumpOnTruePop(jump_idx) | Bytecode::JumpOnFalsePop(jump_idx)
                                | Bytecode::JumpOnTrueTopNil(jump_idx) | Bytecode::JumpOnFalseTopNil(jump_idx) => {
                                    let nbr_new_bc_in_range = non_local_rets_idx.iter()
                                        .filter(|new_bc_idx| (bc_idx < **new_bc_idx) && (**new_bc_idx <= *jump_idx + bc_idx))
                                        .count();
                                    
                                    if nbr_new_bc_in_range > 0 {
                                        *jump_idx += nbr_new_bc_in_range
                                    }
                                },
                                Bytecode::JumpBackward(jump_idx) => {
                                    let nbr_new_bc_in_range = non_local_rets_idx.iter()
                                        .filter(|new_bc_idx| (**new_bc_idx < bc_idx) && (**new_bc_idx >= bc_idx - *jump_idx))
                                        .count();

                                    if nbr_new_bc_in_range > 0 {
                                        *jump_idx += nbr_new_bc_in_range
                                    }
                                },
                                _ => {}
                            }
                        }
                    }

                    assert_eq!(*new_body.last().unwrap(), Bytecode::ReturnLocal); // can't be a return self cause it's never in a block

                    // right before the ReturnLocal, we pop whatever value we wanted to return
                    new_body.insert(new_body.len() - 1, Bytecode::Pop);
                    
                    new_body
                },
                nb_locals: self.blk_info.nb_locals,
                nb_params: self.blk_info.nb_params,
                inline_cache: self.blk_info.inline_cache.clone(),
            }),
        })
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct(&format!("Block{}", self.nb_parameters() + 1))
            .finish()
    }
}
