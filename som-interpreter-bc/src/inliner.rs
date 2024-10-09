use crate::block::{Block, BlockInfo};
use crate::compiler::MethodCodegen;
use crate::compiler::{InnerGenCtxt, Literal};
use crate::inliner::JumpType::{JumpOnFalse, JumpOnTrue};
use crate::inliner::OrAndChoice::{And, Or};
use som_core::ast;
use som_core::bytecode::Bytecode;
use som_core::gc::{GCInterface, GCRef};

pub enum JumpType {
    JumpOnFalse,
    JumpOnTrue,
}

pub enum OrAndChoice {
    Or,
    And,
}

// TODO some of those should return Result types and throw errors instead, most likely.
pub trait PrimMessageInliner {
    /// Starts inlining a function if it's on the list of inlinable functions.
    fn inline_if_possible(&self, ctxt: &mut dyn InnerGenCtxt, mutator: &mut GCInterface) -> Option<()>;
    /// Inlines an expression. If this results in a PushBlock, calls `inline_last_push_block_bc(...)` to inline the block.
    fn inline_expression(&self, ctxt: &mut dyn InnerGenCtxt, expression: &ast::Expression, mutator: &mut GCInterface) -> Option<()>;
    /// Gets the last bytecode, assumes it to be a PushBlock, removes it and inlines the block - a set of operations for which there is a redundant need.
    fn inline_last_push_block_bc(&self, ctxt: &mut dyn InnerGenCtxt, mutator: &mut GCInterface) -> Option<()>;
    /// Inlines a compiled block into the current scope.
    fn inline_compiled_block(&self, ctxt: &mut dyn InnerGenCtxt, block: &BlockInfo, mutator: &mut GCInterface) -> Option<()>;
    /// When inlining a block, adapt its potential children blocks to account for the inlining changes.
    fn adapt_block_after_outer_inlined(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        block_body: GCRef<Block>,
        adjust_scope_by: usize,
        mutator: &mut GCInterface
    ) -> Block;
    /// Inlines `ifTrue:` and `ifFalse:`.
    fn inline_if_true_or_if_false(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        jump_type: JumpType,
        mutator: &mut GCInterface
    ) -> Option<()>;
    /// Inlines `ifTrue:ifFalse:`.
    fn inline_if_true_if_false(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        jump_type: JumpType,
        mutator: &mut GCInterface
    ) -> Option<()>;
    /// Inlines `whileTrue:` and `whileFalse:`.
    fn inline_while(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        jump_type: JumpType,
        mutator: &mut GCInterface
    ) -> Option<()>;
    /// Inlines `and:` and `or:`.
    fn inline_or_and(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        or_and_choice: OrAndChoice,
        mutator: &mut GCInterface
    ) -> Option<()>;
    /// Inlines `to:do`.
    fn inline_to_do(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        mutator: &mut GCInterface
    ) -> Option<()>;
}

impl PrimMessageInliner for ast::Message {
    fn inline_if_possible(&self, ctxt: &mut dyn InnerGenCtxt, mutator: &mut GCInterface) -> Option<()> {
        match self.signature.as_str() {
            "ifTrue:" => self.inline_if_true_or_if_false(ctxt, JumpOnFalse, mutator),
            "ifFalse:" => self.inline_if_true_or_if_false(ctxt, JumpOnTrue, mutator),
            "ifTrue:ifFalse:" => self.inline_if_true_if_false(ctxt, JumpOnFalse, mutator),
            "ifFalse:ifTrue:" => self.inline_if_true_if_false(ctxt, JumpOnTrue, mutator),
            "whileTrue:" => self.inline_while(ctxt, JumpOnFalse, mutator),
            "whileFalse:" => self.inline_while(ctxt, JumpOnTrue, mutator),
            "or:" | "||" => self.inline_or_and(ctxt, Or, mutator),
            "and:" | "&&" => self.inline_or_and(ctxt, And, mutator),
            // "to:do:" => self.inline_to_do(ctxt, mutator),
            // to:by:do, downTo:do:, perhaps?
            _ => None,
        }
    }

    fn inline_expression(&self, ctxt: &mut dyn InnerGenCtxt, expression: &ast::Expression, mutator: &mut GCInterface) -> Option<()> {
        expression.codegen(ctxt, mutator)?;
        match ctxt.get_instructions().last()? {
            Bytecode::PushBlock(_) => self.inline_last_push_block_bc(ctxt, mutator),
            _ => Some(())
        }
    }

    fn inline_last_push_block_bc(&self, ctxt: &mut dyn InnerGenCtxt, mutator: &mut GCInterface) -> Option<()> {
        let block_idx = match ctxt.get_instructions().last()? {
            Bytecode::PushBlock(val) => *val,
            bc => panic!("inlining function expects last bytecode to be a PUSH_BLOCK, instead it was {}.", bc),
        };
        ctxt.pop_instr(); // removing the PUSH_BLOCK

        let cond_block_ref = match ctxt.get_literal(block_idx as usize)? {
            Literal::Block(val) => val.clone(),
            _ => return None,
        };
        ctxt.remove_literal(block_idx as usize);
        

        match self.inline_compiled_block(ctxt, cond_block_ref.to_obj().blk_info.to_obj(), mutator) {
            None => panic!("Inlining a compiled block failed!"),
            _ => Some(()),
        }
    }

    fn inline_compiled_block(&self, ctxt: &mut dyn InnerGenCtxt, block: &BlockInfo, mutator: &mut GCInterface) -> Option<()> {
        let nbr_locals_pre_inlining = ctxt.get_nbr_locals();
        let _nbr_args_pre_inlining = ctxt.get_nbr_args();

        ctxt.set_nbr_locals(nbr_locals_pre_inlining + block.nb_locals + block.nb_params);

        // last is always ReturnLocal, so it gets ignored
        if let Some((_, body)) = block.body.split_last() {
            for block_bc in body {
                match block_bc {
                    Bytecode::PushLocal(idx) => ctxt.push_instr(Bytecode::PushLocal(nbr_locals_pre_inlining as u8 + *idx)),
                    Bytecode::PushNonLocal(up_idx, idx) => {
                        match *up_idx - 1 {
                            0 => ctxt.push_instr(Bytecode::PushLocal(*idx)),
                            _ => ctxt.push_instr(Bytecode::PushNonLocal(*up_idx - 1, *idx))
                        }
                    }
                    Bytecode::PopLocal(up_idx, idx) => match up_idx {
                        0 => ctxt.push_instr(Bytecode::PopLocal(
                            *up_idx,
                            nbr_locals_pre_inlining as u8 + *idx,
                        )),
                        1.. => ctxt.push_instr(Bytecode::PopLocal(*up_idx - 1, *idx)),
                    },
                    Bytecode::NilLocal(idx) => {
                        ctxt.push_instr(Bytecode::NilLocal(nbr_locals_pre_inlining as u8 + *idx))
                    }
                    Bytecode::PushArg(idx) => {
                        ctxt.push_instr(Bytecode::PushLocal(*idx + nbr_locals_pre_inlining as u8 - 1))
                    }
                    Bytecode::PushNonLocalArg(up_idx, idx) => {
                        match *up_idx - 1 {
                            0 => {
                                match *idx {
                                    0 => ctxt.push_instr(Bytecode::PushSelf),
                                    _ => ctxt.push_instr(Bytecode::PushArg(*idx))
                                }
                            },
                            _ => ctxt.push_instr(Bytecode::PushNonLocalArg(*up_idx - 1, *idx))
                        }
                    }
                    Bytecode::PopArg(up_idx, idx) => {
                        ctxt.push_instr(Bytecode::PopArg(*up_idx - 1, *idx))
                    }
                    Bytecode::Send1(lit_idx)
                    | Bytecode::Send2(lit_idx)
                    | Bytecode::Send3(lit_idx)
                    | Bytecode::SendN(lit_idx) => {
                        match block.literals.get(*lit_idx as usize)? {
                            Literal::Symbol(interned) => {
                                // I'm 99% sure this doesn't push duplicate literals. But it miiiight?
                                let idx = ctxt.push_literal(Literal::Symbol(*interned));

                                match block_bc {
                                    Bytecode::Send1(_) => {
                                        ctxt.push_instr(Bytecode::Send1(idx as u8))
                                    }
                                    Bytecode::Send2(_) => {
                                        ctxt.push_instr(Bytecode::Send2(idx as u8))
                                    }
                                    Bytecode::Send3(_) => {
                                        ctxt.push_instr(Bytecode::Send3(idx as u8))
                                    }
                                    Bytecode::SendN(_) => {
                                        ctxt.push_instr(Bytecode::SendN(idx as u8))
                                    }
                                    _ => unreachable!(),
                                }
                            }
                            _ => panic!("Unexpected block literal type, not yet implemented"),
                        }
                    }
                    Bytecode::PushBlock(block_idx) => {
                        match block.literals.get(*block_idx as usize)? {
                            Literal::Block(inner_block) => {
                                let new_block = self.adapt_block_after_outer_inlined(ctxt, *inner_block, 1, mutator);
                                let idx = ctxt.push_literal(Literal::Block(GCRef::<Block>::alloc(new_block, mutator)));
                                ctxt.push_instr(Bytecode::PushBlock(idx as u8));
                            }
                            _ => panic!("PushBlock not actually pushing a block somehow"),
                        };
                    }
                    Bytecode::PushGlobal(global_idx) => {
                        match block.literals.get(*global_idx as usize)? {
                            lit => {
                                let lit_idx = ctxt.push_literal(lit.clone());
                                ctxt.push_instr(Bytecode::PushGlobal(lit_idx as u8));
                            }
                        };
                    }
                    Bytecode::PushConstant(_)
                    | Bytecode::PushConstant0
                    | Bytecode::PushConstant1
                    | Bytecode::PushConstant2 => {
                        let constant_idx = match block_bc {
                            Bytecode::PushConstant(idx) => *idx,
                            Bytecode::PushConstant0 => 0,
                            Bytecode::PushConstant1 => 1,
                            Bytecode::PushConstant2 => 2,
                            _ => unreachable!(),
                        };

                        match block.literals.get(constant_idx as usize)? {
                            lit => {
                                let lit_idx = ctxt.push_literal(lit.clone());
                                match lit_idx {
                                    // maybe create a function just for translating "constant_id (usize) <-> Bytecode" that to avoid duplication
                                    0 => ctxt.push_instr(Bytecode::PushConstant0),
                                    1 => ctxt.push_instr(Bytecode::PushConstant1),
                                    2 => ctxt.push_instr(Bytecode::PushConstant2),
                                    _ => ctxt.push_instr(Bytecode::PushConstant(lit_idx as u8)),
                                }
                            }
                        };
                    }
                    Bytecode::ReturnNonLocal(scope) => {
                        match scope - 1 {
                            0 => ctxt.push_instr(Bytecode::ReturnLocal),
                            new_scope => ctxt.push_instr(Bytecode::ReturnNonLocal(new_scope))
                        }
                    }
                    Bytecode::ReturnLocal => {}
                    Bytecode::ReturnSelf => {
                        panic!("Inlining found a ReturnSelf in a block, which should be impossible.");
                    }
                    Bytecode::Jump(idx) => ctxt.push_instr(Bytecode::Jump(*idx)),
                    Bytecode::JumpBackward(idx) => ctxt.push_instr(Bytecode::JumpBackward(*idx)),
                    Bytecode::JumpOnTruePop(idx) => ctxt.push_instr(Bytecode::JumpOnTruePop(*idx)),
                    Bytecode::JumpOnFalsePop(idx) => {
                        ctxt.push_instr(Bytecode::JumpOnFalsePop(*idx))
                    }
                    Bytecode::JumpOnTrueTopNil(idx) => {
                        ctxt.push_instr(Bytecode::JumpOnTrueTopNil(*idx))
                    }
                    Bytecode::JumpOnFalseTopNil(idx) => {
                        ctxt.push_instr(Bytecode::JumpOnFalseTopNil(*idx))
                    }
                    Bytecode::JumpIfGreater(idx) => {
                        ctxt.push_instr(Bytecode::JumpIfGreater(*idx))
                    }
                    Bytecode::Halt
                    | Bytecode::Dup
                    | Bytecode::Dup2
                    | Bytecode::Inc
                    | Bytecode::Dec
                    | Bytecode::Push0
                    | Bytecode::Push1
                    | Bytecode::PushNil
                    | Bytecode::PushSelf
                    | Bytecode::Pop
                    | Bytecode::Pop2
                    | Bytecode::PushField(_)
                    | Bytecode::PopField(_)
                    | Bytecode::SuperSend1(_)
                    | Bytecode::SuperSend2(_)
                    | Bytecode::SuperSend3(_)
                    | Bytecode::SuperSendN(_) => {
                        ctxt.push_instr(*block_bc)
                    }
                }
            }
        }

        Some(())
    }

    fn adapt_block_after_outer_inlined(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        orig_block: GCRef<Block>,
        adjust_scope_by: usize,
        mutator: &mut GCInterface
    ) -> Block {
        let orig_block = orig_block.to_obj();
        
        let mut block_literals_to_patch = vec![];
        let new_body = orig_block
            .blk_info
            .to_obj()
            .body
            .iter()
            .map(|b| match b {
                Bytecode::PushNonLocal(up_idx, _)
                | Bytecode::PopLocal(up_idx, _)
                | Bytecode::PushNonLocalArg(up_idx, _)
                | Bytecode::PopArg(up_idx, _) => {
                    let new_up_idx = match *up_idx {
                        0 => 0, // local var/arg, not affected by inlining, stays the same
                        d if d > adjust_scope_by as u8 => *up_idx - 1,
                        _ => *up_idx,
                    };

                    // TODO ACTUALLY shouldn't the idx be adjusted depending on the amount of inlined variables in the block? make a test for that!
                    // and for the args case too!
                    // (present me): that's correct. see AST for correct (AFAIK) implem for inlining
                    
                    match b {
                        Bytecode::PushNonLocal(_, idx) => {
                            match new_up_idx {
                                0 => Bytecode::PushLocal(*idx),
                                _ => Bytecode::PushNonLocal(new_up_idx, *idx),
                            }
                        },
                        Bytecode::PopLocal(_, idx) => Bytecode::PopLocal(new_up_idx, *idx),
                        Bytecode::PushNonLocalArg(_, idx) => { 
                            match new_up_idx {
                                0 => {
                                    match *idx {
                                        0 => Bytecode::PushSelf,
                                        _ => Bytecode::PushArg(*idx)
                                    }
                                },
                                _ => Bytecode::PushNonLocalArg(new_up_idx, *idx),
                            }
                        },
                        Bytecode::PopArg(_, idx) => Bytecode::PopArg(new_up_idx, *idx),
                        _ => unreachable!(),
                    }
                }
                Bytecode::ReturnNonLocal(scope) => {
                    match scope - 1 {
                        0 => Bytecode::ReturnLocal,
                        new_scope => Bytecode::ReturnNonLocal(new_scope)
                    }
                }
                Bytecode::PushBlock(block_idx) => {
                    let inner_lit = orig_block
                        .blk_info
                        .to_obj()
                        .literals
                        .get(*block_idx as usize)
                        .unwrap_or_else(|| {
                            panic!("PushBlock is associated with no literal whatsoever?")
                        });
                    let inner_block = match inner_lit {
                        Literal::Block(inner_blk) => inner_blk,
                        _ => panic!("PushBlock is not actually pushing a block somehow"),
                    };

                    let new_block = self.adapt_block_after_outer_inlined(
                        ctxt,
                        *inner_block,
                        adjust_scope_by,
                        mutator
                    );

                    block_literals_to_patch.push((block_idx, GCRef::<Block>::alloc(new_block, mutator)));

                    Bytecode::PushBlock(*block_idx)
                }
                _ => b.clone(),
            })
            .collect();

        // can't just clone the inner_block then modify the body/literals because the body is behind an Rc (not Rc<RefCell<>>), so immutable
        // though if we ever want to do some runtime bytecode rewriting, it'll have to be an Rc<RefCell<>> and this code will be refactorable (not so many individual calls to .clone())
        // TODO: we now pass a mutable pointer to a Block actually, so this is all avoidable
        Block {
            frame: orig_block.frame.clone(),
            blk_info: GCRef::<BlockInfo>::alloc(BlockInfo {
                nb_locals: orig_block.blk_info.to_obj().nb_locals,
                literals: orig_block
                    .blk_info
                    .to_obj()
                    .literals
                    .iter()
                    .enumerate()
                    .map(|(idx, l)| {
                        let block_ptr = block_literals_to_patch
                            .iter()
                            .find_map(|(block_idx, blk)| (**block_idx == idx as u8).then(|| blk));

                        if block_ptr.is_some() {
                            Literal::Block(*block_ptr.unwrap())
                        } else {
                            l.clone()
                        }
                    })
                    .collect(),
                body: new_body,
                nb_params: orig_block.blk_info.to_obj().nb_params,
                inline_cache: orig_block.blk_info.to_obj().inline_cache.clone(),
                #[cfg(feature = "frame-debug-info")]
                block_debug_info: orig_block.blk_info.to_obj().block_debug_info.clone()
            }, mutator),
        }
    }

    fn inline_if_true_or_if_false(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        jump_type: JumpType,
        mutator: &mut GCInterface
    ) -> Option<()> {
        if self.values.len() != 1 { // || !matches!(message.values.get(0)?, ast::Expression::Block(_)) {
            return None;
        }

        let jump_idx = ctxt.get_cur_instr_idx();
        match jump_type {
            JumpOnFalse => ctxt.push_instr(Bytecode::JumpOnFalseTopNil(0)),
            JumpOnTrue => ctxt.push_instr(Bytecode::JumpOnTrueTopNil(0)),
        }

        self.inline_expression(ctxt, self.values.get(0)?, mutator);

        ctxt.backpatch_jump_to_current(jump_idx);

        Some(())
    }

    fn inline_if_true_if_false(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        jump_type: JumpType,
        mutator: &mut GCInterface
    ) -> Option<()> {
        if self.values.len() != 2 {
             // || !matches!(message.values.get(0)?, ast::Expression::Block(_))
             // || !matches!(message.values.get(1)?, ast::Expression::Block(_)) {
            return None;
        }

        let start_jump_idx = ctxt.get_cur_instr_idx();
        match jump_type {
            JumpOnFalse => ctxt.push_instr(Bytecode::JumpOnFalsePop(0)),
            JumpOnTrue => ctxt.push_instr(Bytecode::JumpOnTruePop(0)),
        }

        self.inline_expression(ctxt, self.values.get(0)?, mutator);

        let middle_jump_idx = ctxt.get_cur_instr_idx();
        ctxt.push_instr(Bytecode::Jump(0));

        ctxt.backpatch_jump_to_current(start_jump_idx);

        self.inline_expression(ctxt, self.values.get(1)?, mutator);

        ctxt.backpatch_jump_to_current(middle_jump_idx);

        Some(())
    }

    fn inline_while(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        jump_type: JumpType,
        mutator: &mut GCInterface
    ) -> Option<()> {
        if self.values.len() != 1 || !matches!(self.values.get(0)?, ast::Expression::Block(_)) { // I guess it doesn't have to be a block, but really, it is in all our benchmarks
            return None;
        }

        let idx_pre_condition = ctxt.get_cur_instr_idx();

        // by the time we see it's a "whileTrue:" or a "whileFalse:", there's already been a PushBlock, since they're methods defined on Block
        self.inline_last_push_block_bc(ctxt, mutator);

        let cond_jump_idx = ctxt.get_cur_instr_idx();
        match jump_type {
            JumpOnFalse => ctxt.push_instr(Bytecode::JumpOnFalsePop(0)),
            JumpOnTrue => ctxt.push_instr(Bytecode::JumpOnTruePop(0)),
        }

        self.inline_expression(ctxt, self.values.get(0)?, mutator);

        ctxt.push_instr(Bytecode::Pop);
        
        ctxt.push_instr(Bytecode::JumpBackward((ctxt.get_cur_instr_idx() - idx_pre_condition + 1) as u16));
        ctxt.backpatch_jump_to_current(cond_jump_idx);

        ctxt.push_instr(Bytecode::PushNil);

        Some(())
    }

    fn inline_or_and(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        or_and_choice: OrAndChoice,
        mutator: &mut GCInterface
    ) -> Option<()> {
        if self.values.len() != 1 || !matches!(self.values.get(0)?, ast::Expression::Block(_)) {
            return None;
        }

        let skip_cond_jump_idx = ctxt.get_cur_instr_idx();

        match or_and_choice {
            Or => ctxt.push_instr(Bytecode::JumpOnTruePop(0)),
            And => ctxt.push_instr(Bytecode::JumpOnFalsePop(0)),
        }

        self.inline_expression(ctxt, self.values.get(0)?, mutator);

        let skip_return_true_idx = ctxt.get_cur_instr_idx();
        ctxt.push_instr(Bytecode::Jump(0));

        ctxt.backpatch_jump_to_current(skip_cond_jump_idx);

        let name = match or_and_choice {
            Or => ctxt.intern_symbol("true"),
            And => ctxt.intern_symbol("false"),
        };
        let idx = ctxt.push_literal(Literal::Symbol(name));
        ctxt.push_instr(Bytecode::PushGlobal(idx as u8));

        ctxt.backpatch_jump_to_current(skip_return_true_idx);

        Some(())
    }

    fn inline_to_do(
        &self,
        ctxt: &mut dyn InnerGenCtxt,
        mutator: &mut GCInterface
    ) -> Option<()> {
        if self.values.len() != 2 || !matches!(self.values.get(1)?, ast::Expression::Block(_)) {
            return None;
        }

        self.inline_expression(ctxt, self.values.first()?, mutator);

        let idx_loop_accumulator = ctxt.get_nbr_locals() as u8; // not sure that's correct

        ctxt.push_instr(Bytecode::Dup2);
        // ctxt.push_instr(Bytecode::NilLocal(idx_loop_accumulator));

        let jump_if_greater_idx = ctxt.get_cur_instr_idx();
        ctxt.push_instr(Bytecode::JumpIfGreater(0));

        ctxt.push_instr(Bytecode::Dup);
        ctxt.push_instr(Bytecode::PopLocal(0, idx_loop_accumulator));

        self.inline_expression(ctxt, self.values.get(1)?, mutator); // inline the block

        ctxt.push_instr(Bytecode::Pop);
        ctxt.push_instr(Bytecode::Inc);
        // ctxt.push_instr(Bytecode::NilLocal(idx_loop_accumulator));
        ctxt.push_instr(Bytecode::JumpBackward((ctxt.get_cur_instr_idx() - jump_if_greater_idx) as u16));

        ctxt.backpatch_jump_to_current(jump_if_greater_idx);

        // println!("--- Inlined to:do:.");

        Some(())
    }
}
