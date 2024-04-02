use std::rc::Rc;

use som_core::ast;
use som_core::ast::MethodBody;

use crate::block::Block;
use crate::evaluate::Evaluate;
use crate::frame::Frame;
use crate::frame::FrameKind;
use crate::method::{Method, MethodKind};
use crate::universe::Universe;
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
    fn invoke(&self, universe: &mut Universe, args: Vec<Value>) -> Return;
}

impl Invoke for Method {
    fn invoke(&self, universe: &mut Universe, args: Vec<Value>) -> Return {
        // println!("--- Invoking \"{:1}\" ({:2})", &self.signature, &self.holder.upgrade().unwrap().borrow().name);
        // println!("--- ...with args: {:?}", &args);
        //
        // if self.signature == "at:" {
        //     dbg!("wow");
        // }
        //
        // if !universe.frames.is_empty() {
        //     match &universe.current_method_frame().as_ref().borrow().kind {
        //         FrameKind::Block { .. } => {}
        //         FrameKind::Method { signature, holder, .. } => {
        //             println!("We're in {:?} ({:?})", universe.lookup_symbol(signature.clone()),
        //                      holder.borrow().name)
        //         }
        //     }
        // }

        let output = match self.kind() {
            MethodKind::Defined(method) => {
                let (self_value, params) = {
                    let mut iter = args.into_iter();
                    let receiver = match iter.next() {
                        Some(receiver) => receiver,
                        None => {
                            return Return::Exception("missing receiver for invocation".to_string());
                        }
                    };
                    (receiver, iter.collect::<Vec<_>>())
                };
                // dbg!(&self_value);
                let holder = match self.holder().upgrade() {
                    Some(holder) => holder,
                    None => {
                        return Return::Exception(
                            "cannot invoke this method because its holder has been collected"
                                .to_string(),
                        );
                    }
                };

                let nbr_locals = match &method.body {
                    MethodBody::Body { locals, .. } => {locals.len()}
                    MethodBody::Primitive => unreachable!()
                };

                let signature = universe.intern_symbol(&self.signature);
                universe.with_frame(
                    FrameKind::Method {
                        holder,
                        signature,
                        self_value: self_value.clone(),
                    },
                    self_value,
                    nbr_locals,
                    |universe| method.invoke(universe, params),
                )
            }
            MethodKind::Primitive(func) => func(universe, args),
            MethodKind::NotImplemented(name) => {
                Return::Exception(format!("unimplemented primitive: {}", name))
            }
        };
        // println!("...exiting {:}.", self.signature);
        match output {
            // Return::Exception(msg) => Return::Exception(format!(
            //     "from {}>>#{}\n{}",
            //     self.holder().borrow().name(),
            //     self.signature(),
            //     msg,
            // )),
            output => output,
        }
    }
}

impl Invoke for ast::MethodDef {
    fn invoke(&self, universe: &mut Universe, args: Vec<Value>) -> Return {
        let current_frame = universe.current_frame().clone();
        // if &self.signature == "initialize:" {
        //     dbg!(&self.body);
        // std::process::exit(1);
        // }


        match &self.kind {
            ast::MethodKind::Unary => {}
            ast::MethodKind::Positional { .. } => current_frame
                .borrow_mut()
                .params
                .extend(args),
            ast::MethodKind::Operator { .. } => {
                let rhs_value = match args.into_iter().next() {
                    Some(value) => value,
                    None => {
                        // This should never happen in theory (the parser would have caught the missing rhs).
                        return Return::Exception(format!(
                            "no right-hand side for operator call ?"
                        ));
                    }
                };
                current_frame
                    .borrow_mut()
                    .params
                    .push(rhs_value);
            }
        }
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
    fn invoke(&self, universe: &mut Universe, args: Vec<Value>) -> Return {
        // println!("Invoking a block.");
        // println!("--- ...with args: {:?}", &args);

        // dbg!(&self.block.body);

        let current_frame = universe.current_frame();
        current_frame.borrow_mut().params.extend(args.into_iter().skip(1));

        // dbg!(&current_frame.borrow_mut().params);
        // dbg!(&self.block.parameters);
        // dbg!("--");

        let l = self.block.body.evaluate(universe);
        // println!("...exiting a block.");
        l
    }
}
