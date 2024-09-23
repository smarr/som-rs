use crate::ast::{AstBinaryDispatch, AstBlock, AstBody, AstDispatchNode, AstExpression, AstMethodDef, AstNAryDispatch, AstSuperMessage, AstTernaryDispatch, AstUnaryDispatch};
use crate::class::Class;
use crate::inliner::PrimMessageInliner;
use crate::method::{MethodKind, MethodKindSpecialized};
use crate::specialized::down_to_do_node::DownToDoNode;
use crate::specialized::if_node::IfNode;
use crate::specialized::if_true_if_false_node::IfTrueIfFalseNode;
use crate::specialized::to_by_do_node::ToByDoNode;
use crate::specialized::to_do_node::ToDoNode;
use crate::specialized::trivial_methods::{TrivialGetterMethod, TrivialGlobalMethod, TrivialLiteralMethod, TrivialSetterMethod};
use crate::specialized::while_node::WhileNode;
use som_core::ast;
use som_core::ast::{Expression, MethodBody};
use som_core::gc::{GCInterface, GCRef};

pub struct AstMethodCompilerCtxt<'a> {
    pub scopes: Vec<AstScopeCtxt>,
    pub super_class: Option<GCRef<Class>>,
    pub gc_interface: &'a mut GCInterface
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

impl<'a> AstMethodCompilerCtxt<'a> {
    pub fn get_method_kind(method: &ast::MethodDef, super_class: Option<GCRef<Class>>, gc_interface: &mut GCInterface) -> MethodKind {
        // NB: these If/IfTrueIfFalse/While are very rare cases, since we normally inline those functions.
        // But we don't do inlining when e.g. the condition for ifTrue: isn't a block.
        // so there is *some* occasional benefit in having those specialized method nodes around for those cases.
        match method.signature.as_str() {
            "ifTrue:" => MethodKind::Specialized(MethodKindSpecialized::If(IfNode { expected_bool: true })),
            "ifFalse:" => MethodKind::Specialized(MethodKindSpecialized::If(IfNode { expected_bool: false })),
            "ifTrue:ifFalse:" => MethodKind::Specialized(MethodKindSpecialized::IfTrueIfFalse(IfTrueIfFalseNode {})),
            "whileTrue:" => MethodKind::Specialized(MethodKindSpecialized::While(WhileNode { expected_bool: true })),
            "whileFalse:" => MethodKind::Specialized(MethodKindSpecialized::While(WhileNode { expected_bool: false })),
            "to:do:" => MethodKind::Specialized(MethodKindSpecialized::ToDo(ToDoNode {})),
            "to:by:do:" => MethodKind::Specialized(MethodKindSpecialized::ToByDo(ToByDoNode {})),
            "downTo:do:" => MethodKind::Specialized(MethodKindSpecialized::DownToDo(DownToDoNode {})),
            _ => {
                match method.body {
                    MethodBody::Primitive => MethodKind::NotImplemented(method.signature.clone()),
                    MethodBody::Body { .. } => {
                        let ast_method_def = AstMethodCompilerCtxt::parse_method_def(method, super_class, gc_interface);

                        if let Some(trivial_method_kind) = AstMethodCompilerCtxt::make_trivial_method_if_possible(&ast_method_def) {
                            trivial_method_kind
                        } else {
                            MethodKind::Defined(ast_method_def)
                        }
                    }
                }
            }
        }
    }

    fn make_trivial_method_if_possible(method_def: &AstMethodDef) -> Option<MethodKind> {
        if method_def.locals_nbr != 0 || method_def.body.exprs.len() != 1 {
            return None;
        }

        match method_def.body.exprs.first()? {
            AstExpression::LocalExit(expr) => {
                match expr.as_ref() {
                    AstExpression::Literal(lit) => {
                        Some(MethodKind::TrivialLiteral(TrivialLiteralMethod { literal: lit.clone() })) // todo avoid clone by moving code to previous function tbh
                    }
                    AstExpression::GlobalRead(global) => {
                        Some(MethodKind::TrivialGlobal(TrivialGlobalMethod { global_name: global.clone() }))
                    }
                    AstExpression::FieldRead(idx) => {
                        Some(MethodKind::TrivialGetter(TrivialGetterMethod { field_idx: *idx }))
                    }
                    _ => None
                }
            }
            AstExpression::FieldWrite(idx, expr) => {
                match expr.as_ref() {
                    AstExpression::ArgRead(0, 1) => {
                        Some(MethodKind::TrivialSetter(TrivialSetterMethod { field_idx: *idx }))
                    }
                    _ => None
                }
            }
            _ => None
        }
    }

    /// Transforms a generic MethodDef into an AST-specific one.
    /// Note: public since it's used in tests.
    pub fn parse_method_def(method_def: &ast::MethodDef, super_class: Option<GCRef<Class>>, gc_interface: &mut GCInterface) -> AstMethodDef {
        let (body, locals_nbr) = match &method_def.body {
            MethodBody::Primitive => { unreachable!("unimplemented primitive") }
            MethodBody::Body { locals_nbr, body, .. } => {
                let args_nbr = method_def.signature.chars().filter(|e| *e == ':').count(); // not sure if needed
                let mut ctxt = AstMethodCompilerCtxt { scopes: vec![AstScopeCtxt::init(args_nbr, *locals_nbr, false)], super_class, gc_interface: gc_interface };

                (ctxt.parse_body(body), ctxt.scopes.last().unwrap().get_nbr_locals())
            }
        };

        AstMethodDef {
            signature: method_def.signature.clone(),
            locals_nbr,
            body,
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
            Expression::GlobalWrite(_global_name, _expr) => todo!("handle field writes"),
            Expression::FieldWrite(a, b) => AstExpression::FieldWrite(a, Box::new(self.parse_expression(b.as_ref()))),
            Expression::Message(msg) => self.parse_message(msg.as_ref()),
            Expression::Exit(a, b) => {
                match b {
                    0 => AstExpression::LocalExit(Box::new(self.parse_expression(a.as_ref()))),
                    _ => AstExpression::NonLocalExit(Box::new(self.parse_expression(a.as_ref())), b)
                }
            }
            Expression::Literal(a) => AstExpression::Literal(a),
            Expression::Block(a) => AstExpression::Block(GCRef::<AstBlock>::alloc(self.parse_block(&a), self.gc_interface))
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

    pub fn parse_message(&mut self, msg: &ast::Message) -> AstExpression {
        self.parse_message_with_func(msg, Self::parse_expression)
    }

    pub fn parse_message_with_inlining(&mut self, msg: &ast::Message) -> AstExpression {
        self.parse_message_with_func(msg, Self::parse_expression_with_inlining)
    }
    
    pub fn parse_message_with_func(&mut self, msg: &ast::Message, expr_parsing_func: fn(&mut AstMethodCompilerCtxt<'a>, &Expression) -> AstExpression) -> AstExpression {
        let maybe_inlined = self.inline_if_possible(msg);
        if let Some(inlined_node) = maybe_inlined {
            return AstExpression::InlinedCall(Box::new(inlined_node));
        }

        let receiver = expr_parsing_func(self, &msg.receiver);
        match receiver {
            _super if _super == AstExpression::GlobalRead(String::from("super")) => {
                AstExpression::SuperMessage(Box::new(
                AstSuperMessage {
                    super_class: self.super_class.clone().unwrap_or_else(|| panic!("no super class set, even though the method has a super call?")),
                    signature: msg.signature.clone(),
                    values: msg.values.iter().map(|e| expr_parsing_func(self, e)).collect(),
                }))
            },
            _ => {
                match msg.values.len() {
                    0 => {
                        AstExpression::UnaryDispatch(Box::new(
                            AstUnaryDispatch {
                                dispatch_node: AstDispatchNode {
                                    receiver,
                                    signature: msg.signature.clone(),
                                    inline_cache: None
                                }
                            }))
                    },
                    1 => {
                        AstExpression::BinaryDispatch(Box::new(
                            AstBinaryDispatch {
                                dispatch_node: AstDispatchNode {
                                    receiver,
                                    signature: msg.signature.clone(),
                                    inline_cache: None
                                },
                                arg: expr_parsing_func(self, msg.values.first().unwrap()),
                            }))
                    },
                    2 => {
                        AstExpression::TernaryDispatch(Box::new(
                            AstTernaryDispatch {
                                dispatch_node: AstDispatchNode {
                                    receiver,
                                    signature: msg.signature.clone(),
                                    inline_cache: None
                                },
                                arg1: expr_parsing_func(self, msg.values.first().unwrap()),
                                arg2: expr_parsing_func(self, msg.values.get(1).unwrap()),
                            }))
                    },
                    _ => {
                        AstExpression::NAryDispatch(Box::new(
                            AstNAryDispatch {
                                dispatch_node: AstDispatchNode {
                                    receiver,
                                    signature: msg.signature.clone(),
                                    inline_cache: None
                                },
                                values: msg.values.iter().map(|e| expr_parsing_func(self, e)).collect(),
                            }))
                    }
                }
            }
        }
    }
}