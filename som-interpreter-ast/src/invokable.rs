use std::rc::Rc;

use som_core::ast;
use som_core::ast::MethodBody;

use crate::block::Block;
use crate::evaluate::Evaluate;
use crate::frame::Frame;
use crate::method::{Method, MethodKind};
use crate::universe::UniverseAST;
use crate::value::Value;
use crate::SOMRef;

/// Represents the kinds of possible returns from an invocation.
#[derive(Debug)]
pub enum Return {
    /// A local return, the value is for the immediate caller.
    Local(Value),
    /// A non-local return, the value is for the parent of the referenced stack frame.
    NonLocal(Value, SOMRef<Frame>),
    /// An exception, expected to bubble all the way up.
    Exception(String),
    /// A request to restart execution from the top of the closest body.
    Restart,
}

/// The trait for invoking methods and primitives.
pub trait Invoke {
    /// Invoke within the given universe and with the given arguments.
    fn invoke(&self, universe: &mut UniverseAST, args: Vec<Value>) -> Return;
}

impl Invoke for Method {
    fn invoke(&self, universe: &mut UniverseAST, args: Vec<Value>) -> Return {
        // println!("--- Invoking \"{:1}\" ({:2})", &self.signature, &self.holder.upgrade().unwrap().borrow().name);
        // println!("--- ...with args: {:?}", &args);

        match self.kind() {
            MethodKind::Defined(method) => {
                let nbr_locals = match &method.body {
                    MethodBody::Body { locals_nbr, .. } => *locals_nbr,
                    MethodBody::Primitive => unreachable!()
                };

                universe.with_frame(
                    nbr_locals,
                    args,
                    |universe| method.invoke(universe, vec![]),
                )
            }
            MethodKind::Primitive(func) => func(universe, args),
            MethodKind::WhileInlined(while_node) => { while_node.invoke(universe, args) }
            MethodKind::IfInlined(if_node) => { if_node.invoke(universe, args) }
            MethodKind::IfTrueIfFalseInlined(if_true_if_false_node) => { if_true_if_false_node.invoke(universe, args) },
            MethodKind::NotImplemented(name) => { Return::Exception(format!("unimplemented primitive: {}", name)) }
        }
    }
}

impl Invoke for ast::GenericMethodDef {
    fn invoke(&self, universe: &mut UniverseAST, _: Vec<Value>) -> Return {
        let current_frame = universe.current_frame().clone();

        match &self.body {
            ast::MethodBody::Body { body, .. } => {
                loop {
                    match body.evaluate(universe) {
                        Return::NonLocal(value, frame) => {
                            if Rc::ptr_eq(&current_frame, &frame) {
                                break Return::Local(value);
                            } else {
                                break Return::NonLocal(value, frame);
                            }
                        }
                        Return::Local(_) => break Return::Local(current_frame.borrow().get_self()),
                        Return::Exception(msg) => break Return::Exception(msg),
                        Return::Restart => continue,
                    }
                }
            }
            ast::MethodBody::Primitive => Return::Exception(format!(
                "unimplemented primitive: {}>>#{}",
                current_frame
                    .borrow()
                    .get_self()
                    .class(universe)
                    .borrow()
                    .name(),
                self.signature,
            )),
        }
    }
}

impl Invoke for Block {
    fn invoke(&self, universe: &mut UniverseAST, _: Vec<Value>) -> Return {
        self.block.body.evaluate(universe)
    }
}
