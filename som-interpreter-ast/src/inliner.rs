use crate::ast::{AstBlock, AstBody, AstExpression, InlinedNode};
use crate::compiler::{AstMethodCompilerCtxt, AstScopeCtxt};
use crate::specialized::inlined::and_inlined_node::AndInlinedNode;
use crate::specialized::inlined::if_inlined_node::IfInlinedNode;
use crate::specialized::inlined::if_true_if_false_inlined_node::IfTrueIfFalseInlinedNode;
use crate::specialized::inlined::or_inlined_node::OrInlinedNode;
use crate::specialized::inlined::while_inlined_node::WhileInlinedNode;
use som_core::ast;
use som_core::ast::{Block, Expression};

pub trait PrimMessageInliner {
    fn inline_if_possible(&mut self, msg: &ast::Message) -> Option<InlinedNode>;
    fn parse_expression_with_inlining(&mut self, expression: &Expression) -> AstExpression;
    fn inline_block(&mut self, expression: &Block) -> AstBody;
    fn adapt_block_after_outer_inlined(&mut self, blk: &Block) -> AstBlock;
    fn adapt_var_or_args_coords_from_inlining(&self, up_idx: usize, idx: usize, vars_or_args_fetcher_func: fn(&AstScopeCtxt) -> usize) -> (u8, u8);
    fn inline_if_true_or_if_false(&mut self, msg: &ast::Message, expected_bool: bool) -> Option<InlinedNode>;
    fn inline_if_true_if_false(&mut self, msg: &ast::Message, expected_bool: bool) -> Option<InlinedNode>;
    fn inline_while(&mut self, msg: &ast::Message, expected_bool: bool) -> Option<InlinedNode>;
    fn inline_or(&mut self, msg: &ast::Message) -> Option<InlinedNode>;
    fn inline_and(&mut self, msg: &ast::Message) -> Option<InlinedNode>;
}

impl PrimMessageInliner for AstMethodCompilerCtxt<'_> {
    fn inline_if_possible(&mut self, msg: &ast::Message) -> Option<InlinedNode> {
        match msg.signature.as_str() {
            "ifTrue:" => self.inline_if_true_or_if_false(msg, true),
            "ifFalse:" => self.inline_if_true_or_if_false(msg, false),
            "ifTrue:ifFalse:" => self.inline_if_true_if_false(msg, true),
            "ifFalse:ifTrue:" => self.inline_if_true_if_false(msg, false),
            "whileTrue:" => self.inline_while(msg, true),
            "whileFalse:" => self.inline_while(msg, false),
            "or:" | "||" => self.inline_or(msg),
            "and:" | "&&" => self.inline_and(msg),
            _ => None,
        }
    }

    /// Parses an expression while taking the possible effects of inlining into account.
    fn parse_expression_with_inlining(&mut self, expression: &Expression) -> AstExpression {
        let expr = match expression {
            Expression::Block(blk) => {
                let new_blk = self.adapt_block_after_outer_inlined(blk);
                let new_blk_ptr = self.gc_interface.alloc(new_blk); // could we just adapt the old block instead of allocating?
                AstExpression::Block(new_blk_ptr)
            }
            Expression::LocalVarRead(idx)
            | Expression::LocalVarWrite(idx, _)
            | Expression::NonLocalVarRead(_, idx)
            | Expression::NonLocalVarWrite(_, idx, _) => {
                let up_idx = match expression {
                    Expression::LocalVarRead(..) | Expression::LocalVarWrite(..) => 0,
                    Expression::NonLocalVarRead(up_idx, ..) | Expression::NonLocalVarWrite(up_idx, ..) => *up_idx,
                    _ => unreachable!(),
                };

                let (new_up_idx, new_idx) = self.adapt_var_or_args_coords_from_inlining(up_idx, *idx, AstScopeCtxt::get_nbr_locals);

                match new_up_idx {
                    0 => match expression {
                        Expression::LocalVarRead(..) | Expression::NonLocalVarRead(..) => AstExpression::LocalVarRead(new_idx),
                        Expression::LocalVarWrite(.., expr) | Expression::NonLocalVarWrite(.., expr) => {
                            AstExpression::LocalVarWrite(new_idx, Box::new(self.parse_expression_with_inlining(expr)))
                        }
                        _ => unreachable!(),
                    },
                    _ => match expression {
                        Expression::NonLocalVarRead(..) => AstExpression::NonLocalVarRead(new_up_idx, new_idx),
                        Expression::NonLocalVarWrite(.., expr) => {
                            AstExpression::NonLocalVarWrite(new_up_idx, new_idx, Box::new(self.parse_expression_with_inlining(expr)))
                        }
                        _ => unreachable!(),
                    },
                }
            }
            Expression::ArgRead(up_idx, idx) | Expression::ArgWrite(up_idx, idx, _) => {
                let (new_up_idx, new_idx) = self.adapt_var_or_args_coords_from_inlining(*up_idx, *idx, AstScopeCtxt::get_nbr_locals);

                match expression {
                    Expression::ArgRead(..) => AstExpression::ArgRead(new_up_idx, new_idx),
                    Expression::ArgWrite(.., expr) => {
                        AstExpression::ArgWrite(new_up_idx, new_idx, Box::new(self.parse_expression_with_inlining(expr)))
                    }
                    _ => unreachable!(),
                }
            }
            Expression::Exit(expr, scope) => {
                let inline_expr = self.parse_expression_with_inlining(expr);
                let adjust_scope_by = self.scopes.iter().rev().take(*scope).filter(|e| e.is_getting_inlined).count();
                let new_scope = scope - adjust_scope_by;
                match new_scope {
                    0 => AstExpression::LocalExit(Box::new(inline_expr)),
                    _ => AstExpression::NonLocalExit(Box::new(inline_expr), new_scope as u8),
                }
            }
            Expression::GlobalRead(a) => self.global_or_field_read_from_superclass(a.clone()),
            Expression::GlobalWrite(name, expr) => {
                if self.class.is_none() {
                    panic!(
                        "can't turn the GlobalWrite `{}` into a FieldWrite, and GlobalWrite shouldn't exist at runtime",
                        name
                    )
                }
                match self.class.unwrap().get_field_offset_by_name(&name) {
                    Some(offset) => AstExpression::FieldWrite(offset as u8, Box::new(self.parse_expression_with_inlining(expr))),
                    _ => panic!(
                        "can't turn the GlobalWrite `{}` into a FieldWrite, and GlobalWrite shouldn't exist at runtime",
                        name
                    ),
                }
            }
            Expression::Message(msg) => self.parse_message_with_inlining(msg),
            Expression::Literal(lit) => AstExpression::Literal(self.parse_literal(lit)),
        };

        expr
    }

    fn inline_block(&mut self, blk: &Block) -> AstBody {
        self.scopes.push(AstScopeCtxt::init(blk.nbr_params, blk.nbr_locals, true));

        let inlined_block = AstBody {
            exprs: blk.body.exprs.iter().map(|e| self.parse_expression_with_inlining(e)).collect(),
        };

        let (nbr_params_post_inlining, nbr_locals_post_inlining) = {
            let blk_scope = self.scopes.last().unwrap();
            (blk_scope.get_nbr_args(), blk_scope.get_nbr_locals())
        };

        self.scopes.pop();

        self.scopes.last_mut().unwrap().add_nbr_args(nbr_params_post_inlining);
        self.scopes.last_mut().unwrap().add_nbr_locals(nbr_locals_post_inlining);

        inlined_block
    }

    fn adapt_block_after_outer_inlined(&mut self, blk: &Block) -> AstBlock {
        self.scopes.push(AstScopeCtxt::init(blk.nbr_params, blk.nbr_locals, false));

        let exprs: Vec<AstExpression> = blk.body.exprs.iter().map(|og_expr| self.parse_expression_with_inlining(og_expr)).collect();

        let (nbr_params, nbr_locals) = {
            let outer_blk_scope = self.scopes.last().unwrap();
            (outer_blk_scope.get_nbr_args() as u8, outer_blk_scope.get_nbr_locals() as u8)
        };

        let adapted_inner_block = AstBlock {
            nbr_params,
            nbr_locals,
            body: AstBody { exprs },
        };

        self.scopes.pop();

        adapted_inner_block
    }

    fn adapt_var_or_args_coords_from_inlining(&self, up_idx: usize, idx: usize, vars_or_args_fetcher_func: fn(&AstScopeCtxt) -> usize) -> (u8, u8) {
        // new up index is the target var scope minus the number of inlined scopes in between the current scope and the target var scope
        // if you do a NonLocalVarRead(3, 0), and there's 1 inlined scope before that (3) target, then now that target scope is only (2) scopes away.
        let new_up_idx = match up_idx {
            0 => 0, // branch not strictly necessary, but faster, and for a very common case.
            _ => up_idx - self.scopes.iter().rev().take(up_idx).filter(|e| e.is_getting_inlined).count(),
        };

        // new index is more complicated, since some variables can have gotten inlined into other scopes.
        let new_idx = {
            let up_idx_of_scope_var_will_end_up_into =
                self.scopes.iter().rev().skip(up_idx).take_while(|scope| scope.is_getting_inlined).count() + up_idx; // if we're accessing a var into a scope that's getting inlined, then its idx may have changed due to the inlining.

            let nbr_vars_in_final_scope_to_offset_by = self
                .scopes
                .iter()
                .rev()
                .skip(up_idx + 1) // we go back right before the target scope...
                .take(up_idx_of_scope_var_will_end_up_into - up_idx) // ...and we get all the scopes that get inlined before the target scope...
                .map(vars_or_args_fetcher_func) // ...and we aggregate their args/vars. This number will be by how much to offset the variable index to account for inlining.
                .sum::<usize>();

            // Visual aide to understand why and how vars need to be modified:
            // _
            // | |a b|
            // |      // V -- THIS SCOPE GETS INLINED (prev scope vars become |a b c| )
            // |         _
            // |         | |c|
            // |         |    // V -- not inlined.
            // |         |       _
            // |         |      | |d|
            // |         |      |  VarRead(0, 0)... becomes: => VarRead(0, 0)
            // |         |      |  VarRead(1, 0) => VarRead(1, 2)
            // |         |      |  VarRead(2, 0) => VarRead(1, 0)
            // |         |      |  VarRead(2, 1) => VarRead(1, 1)
            // |         |    _
            // |         _
            // _

            nbr_vars_in_final_scope_to_offset_by + idx
        };

        (new_up_idx as u8, new_idx as u8)
    }

    fn inline_if_true_or_if_false(&mut self, msg: &ast::Message, expected_bool: bool) -> Option<InlinedNode> {
        let body_blk = match msg.values.first() {
            Some(Expression::Block(blk)) => blk,
            _ => return None,
        };

        let if_inlined_node = IfInlinedNode {
            expected_bool,
            cond_expr: self.parse_expression_with_inlining(&msg.receiver),
            body_instrs: self.inline_block(body_blk),
        };

        Some(InlinedNode::IfInlined(if_inlined_node))
    }

    fn inline_if_true_if_false(&mut self, msg: &ast::Message, expected_bool: bool) -> Option<InlinedNode> {
        let (body_blk_1, body_blk_2) = match (msg.values.first(), msg.values.get(1)) {
            (Some(Expression::Block(blk)), Some(Expression::Block(blk2))) => (blk, blk2),
            _ => return None,
        };

        let if_true_if_false_inlined_node = IfTrueIfFalseInlinedNode {
            expected_bool,
            cond_expr: self.parse_expression_with_inlining(&msg.receiver),
            body_1_instrs: self.inline_block(body_blk_1),
            body_2_instrs: self.inline_block(body_blk_2),
        };

        Some(InlinedNode::IfTrueIfFalseInlined(if_true_if_false_inlined_node))
    }

    fn inline_while(&mut self, msg: &ast::Message, expected_bool: bool) -> Option<InlinedNode> {
        let (cond_blk, body_blk) = match (&msg.receiver, msg.values.first()) {
            (Expression::Block(cond_blk), Some(Expression::Block(body_blk))) => (cond_blk, body_blk),
            _ => return None,
        };

        let while_inlined_node = WhileInlinedNode {
            expected_bool,
            cond_instrs: self.inline_block(cond_blk),
            body_instrs: self.inline_block(body_blk),
        };

        Some(InlinedNode::WhileInlined(while_inlined_node))
    }

    fn inline_or(&mut self, msg: &ast::Message) -> Option<InlinedNode> {
        let snd_blk = match msg.values.first() {
            Some(Expression::Block(blk)) => blk,
            _ => return None,
        };

        let or_inlined_node = OrInlinedNode {
            first: self.parse_expression_with_inlining(&msg.receiver),
            second: self.inline_block(snd_blk),
        };

        Some(InlinedNode::OrInlined(or_inlined_node))
    }

    fn inline_and(&mut self, msg: &ast::Message) -> Option<InlinedNode> {
        let snd_blk = match msg.values.first() {
            Some(Expression::Block(blk)) => blk,
            _ => return None,
        };

        let and_inlined_node = AndInlinedNode {
            first: self.parse_expression_with_inlining(&msg.receiver),
            second: self.inline_block(snd_blk),
        };

        Some(InlinedNode::AndInlined(and_inlined_node))
    }
}
