use std::rc::Rc;

use som_core::ast;
use som_core::ast::{Expression, MethodBody};

use crate::ast::{AstBinaryOp, AstBlock, AstBody, AstExpression, AstMessage, AstMethodDef, AstSuperMessage};
use crate::inliner::PrimMessageInliner;
use crate::method::MethodKind;
use crate::specialized::down_to_do_node::DownToDoNode;
use crate::specialized::if_node::IfNode;
use crate::specialized::if_true_if_false_node::IfTrueIfFalseNode;
use crate::specialized::to_by_do_node::ToByDoNode;
use crate::specialized::to_do_node::ToDoNode;
use crate::specialized::while_node::WhileNode;

pub struct AstMethodCompilerCtxt {
    pub scopes: Vec<AstScopeCtxt>,
}

#[derive(Debug, Default)]
pub struct AstScopeCtxt {
    nbr_args: usize,
    nbr_locals: usize,
    pub is_getting_inlined: bool,
}

impl AstScopeCtxt {
    pub fn init(nbr_args: usize,
                nbr_locals: usize,
                is_getting_inlined: bool) -> Self {
        Self {
            nbr_args,
            nbr_locals,
            is_getting_inlined,
        }
    }

    pub fn get_nbr_locals(&self) -> usize {
        self.nbr_locals
    }

    pub fn add_nbr_locals(&mut self, nbr_to_add: usize) {
        self.nbr_locals += nbr_to_add;
    }
    pub fn get_nbr_args(&self) -> usize {
        self.nbr_args
    }

    pub fn add_nbr_args(&mut self, nbr_to_add: usize) {
        self.nbr_args += nbr_to_add;
    }
}

impl AstMethodCompilerCtxt {
    pub fn get_method_kind(method: &ast::MethodDef) -> MethodKind {
        // NB: these If/IfTrueIfFalse/While are very rare cases, since we normally inline those functions.
        // But we don't do inlining when e.g. the condition for ifTrue: isn't a block.
        // so there is *some* occasional benefit in having those specialized method nodes around for those cases.
        match method.signature.as_str() {
            "ifTrue:" => MethodKind::If(IfNode { expected_bool: true }),
            "ifFalse:" => MethodKind::If(IfNode { expected_bool: false }),
            "ifTrue:ifFalse:" => MethodKind::IfTrueIfFalse(IfTrueIfFalseNode {}),
            "whileTrue:" => MethodKind::While(WhileNode { expected_bool: true }),
            "whileFalse:" => MethodKind::While(WhileNode { expected_bool: false }),
            "to:do:" => MethodKind::ToDo(ToDoNode{}),
            "to:by:do:" => MethodKind::ToByDo(ToByDoNode{}),
            "downTo:do:" => MethodKind::DownToDo(DownToDoNode{}),
            _ => {
                match method.body {
                    MethodBody::Primitive => MethodKind::NotImplemented(method.signature.clone()),
                    MethodBody::Body { .. } => MethodKind::Defined(AstMethodCompilerCtxt::parse_method_def(method))
                }
            }
        }
    }
    
    /// Transforms a generic MethodDef into an AST-specific one.
    /// Note: public since it's used in tests.
    pub fn parse_method_def(method_def: &ast::MethodDef) -> AstMethodDef {
        let (body, locals_nbr) = match &method_def.body {
            MethodBody::Primitive => { unreachable!("unimplemented primitive") }
            MethodBody::Body { locals_nbr, body, .. } => {
                let args_nbr = method_def.signature.chars().filter(|e| *e == ':').count(); // not sure if needed
                let mut ctxt = AstMethodCompilerCtxt { scopes: vec![AstScopeCtxt::init(args_nbr, *locals_nbr, false)] };

                (ctxt.parse_body(body), ctxt.scopes.last().unwrap().get_nbr_locals())
            }
        };
        
        /*if locals_nbr == 0 && body.exprs.len() == 1 {
            match body.exprs.first().unwrap() {
                // AstExpression::FieldWrite(..) => {dbg!(&body);},
                AstExpression::LocalExit(expr) => {
                    match expr.as_ref() {
                        AstExpression::Literal(lit) => {
                            dbg!(&body);
                            return LiteralTrivialMethod(lit);
                        },
                        _ => {}
                    };
                    // dbg!(&body);
                },
                _ => {}
            }
        }*/
        
        AstMethodDef {
            signature: method_def.signature.clone(),
            locals_nbr,
            body
        }
    }

    pub fn parse_expression(&mut self, expr: &Expression) -> AstExpression {
        match expr.clone() {
            Expression::GlobalRead(global_name) => AstExpression::GlobalRead(global_name.clone()),
            Expression::LocalVarRead(idx) => AstExpression::LocalVarRead(idx),
            Expression::NonLocalVarRead(scope, idx) => AstExpression::NonLocalVarRead(scope, idx),
            Expression::ArgRead(scope, idx) => AstExpression::ArgRead(scope, idx),
            Expression::FieldRead(idx) => AstExpression::FieldRead(idx),
            Expression::LocalVarWrite(a, b) => AstExpression::LocalVarWrite(a, Box::new(self.parse_expression(b.as_ref()))),
            Expression::NonLocalVarWrite(a, b, c) => AstExpression::NonLocalVarWrite(a, b, Box::new(self.parse_expression(c.as_ref()))),
            Expression::ArgWrite(a, b, c) => AstExpression::ArgWrite(a, b, Box::new(self.parse_expression(c.as_ref()))),
            Expression::FieldWrite(a, b) => AstExpression::FieldWrite(a, Box::new(self.parse_expression(b.as_ref()))),
            Expression::Message(msg) => self.parse_message_maybe_inline(msg.as_ref()),
            Expression::SuperMessage(a) => AstExpression::SuperMessage(Box::new(self.parse_super_message(a.as_ref()))),
            Expression::BinaryOp(a) => AstExpression::BinaryOp(Box::new(self.parse_binary_op(a.as_ref()))),
            Expression::Exit(a, b) => {
                match b {
                    0 => AstExpression::LocalExit(Box::new(self.parse_expression(a.as_ref()))),
                    _ => AstExpression::NonLocalExit(Box::new(self.parse_expression(a.as_ref())), b)
                }
            },
            Expression::Literal(a) => AstExpression::Literal(a),
            Expression::Block(a) => AstExpression::Block(Rc::new(self.parse_block(&a)))
        }
    }

    pub fn parse_body(&mut self, body: &ast::Body) -> AstBody {
        AstBody {
            exprs: body.exprs.iter().map(|expr| self.parse_expression(expr)).collect()
        }
    }

    pub fn parse_block(&mut self, blk: &ast::Block) -> AstBlock {
        self.scopes.push(AstScopeCtxt::init(blk.nbr_params, blk.nbr_locals, false));

        let body = self.parse_body(&blk.body);
        let bl = self.scopes.last().unwrap();
        let output_blk = AstBlock {
            nbr_params: bl.get_nbr_args(),
            nbr_locals: bl.get_nbr_locals(),
            body,
        };

        self.scopes.pop();
        output_blk
    }

    pub fn parse_binary_op(&mut self, binary_op: &ast::BinaryOp) -> AstBinaryOp {
        AstBinaryOp {
            op: binary_op.op.clone(),
            lhs: self.parse_expression(&binary_op.lhs),
            rhs: self.parse_expression(&binary_op.rhs),
        }
    }

    pub fn parse_message_maybe_inline(&mut self, msg: &ast::Message) -> AstExpression {
        let maybe_inlined = self.inline_if_possible(msg);
        if let Some(inlined_node) = maybe_inlined {
            return AstExpression::InlinedCall(Box::new(inlined_node));
        }

        AstExpression::Message(Box::new(
            AstMessage {
                receiver: self.parse_expression(&msg.receiver),
                signature: msg.signature.clone(),
                values: msg.values.iter().map(|e| self.parse_expression(e)).collect(),
            })
        )
    }

    pub fn parse_super_message(&mut self, super_msg: &ast::SuperMessage) -> AstSuperMessage {
        AstSuperMessage {
            receiver_name: super_msg.receiver_name.clone(),
            is_static_class_call: super_msg.is_static_class_call,
            signature: super_msg.signature.clone(),
            values: super_msg.values.iter().map(|e| self.parse_expression(e)).collect(),
        }
    }
}