use crate::ast::{AstBinaryDispatch, AstBlock, AstBody, AstDispatchNode, AstExpression, AstMethodDef, AstNAryDispatch, AstSuperMessage, AstTerm, AstTernaryDispatch, AstUnaryDispatch, InlinedNode};
use crate::block::Block;
use crate::frame::{Frame, FrameAccess};
use crate::invokable::{Invoke, Return};
use crate::method::Method;
use crate::universe::UniverseAST;
use crate::value::Value;
use num_bigint::BigInt;
use som_core::ast;
use som_core::gc::GCRef;

/// The trait for evaluating AST nodes.
pub trait Evaluate {
    /// Evaluate the node within a given universe.
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return;
}

impl Evaluate for AstExpression {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        match self {
            Self::LocalVarWrite(idx, expr) => {
                // TODO: this doesn't call the fastest path for evaluate, still has to dispatch the right expr even though it's always a var write. potential minor speedup there
                let value = propagate!(expr.evaluate(universe));
                universe.assign_local(*idx, &value);
                Return::Local(value)
            }
            Self::NonLocalVarWrite(scope, idx, expr) => {
                let value = propagate!(expr.evaluate(universe));
                universe.assign_non_local(*idx, *scope, &value);
                Return::Local(value)
            }
            Self::FieldWrite(idx, expr) => {
                let value = propagate!(expr.evaluate(universe));
                universe.assign_field(*idx, &value);
                Return::Local(value)
            }
            Self::ArgWrite(scope, idx, expr) => {
                let value = propagate!(expr.evaluate(universe));
                universe.assign_arg(*idx, *scope, &value);
                Return::Local(value)
            }
            Self::Block(blk) => blk.evaluate(universe),
            Self::LocalExit(expr) => {
                let value = propagate!(expr.evaluate(universe));
                Return::NonLocal(value, universe.current_frame)  // not well named - Return::NonLocal means "exits the scope", so it can be a regular, local return. 
            }
            Self::NonLocalExit(expr, scope) => {
                debug_assert_ne!(*scope, 0);

                let value = propagate!(expr.evaluate(universe));
                let method_frame = Frame::nth_frame_back(&universe.current_frame, *scope);
                // let has_not_escaped = universe
                //     .frames
                //     .iter()
                //     .rev()
                //     .any(|live_frame| *live_frame == method_frame);

                let has_not_escaped = {
                    let mut current_frame = universe.current_frame;

                    loop {
                        if current_frame == method_frame {
                            break true
                        } else if current_frame.is_empty() {
                            break false
                        } else {
                            current_frame = current_frame.to_obj().prev_frame;
                        }
                    }
                };

                if has_not_escaped {
                    // the BC interp has to pop all the escaped frames here, we don't (because we chain return nonlocals, exception-style?).
                    Return::NonLocal(value, method_frame)
                } else {
                    // Block has escaped its method frame.
                    let instance = method_frame.get_self();
                    let frame = universe.current_frame;
                    let block = match frame.lookup_argument(0) {
                        Value::Block(b) => b,
                        _ => {
                            // Should never happen, because `universe.current_frame()` would
                            // have been equal to `universe.current_method_frame()`.
                            return Return::Exception("A method frame has escaped itself ??".to_string());
                        }
                    };
                    universe.escaped_block(instance, block).unwrap_or_else(|| {
                        // TODO: should we call `doesNotUnderstand:` here ?
                        Return::Exception(
                            "A block has escaped and `escapedBlock:` is not defined on receiver"
                                .to_string(),
                        )
                    })
                }
            }
            Self::Literal(literal) => literal.evaluate(universe),
            Self::LocalVarRead(idx) => {
                Return::Local(universe.lookup_local(*idx))
            }
            Self::NonLocalVarRead(scope, idx) => {
                Return::Local(universe.lookup_non_local(*idx, *scope))
            }
            Self::FieldRead(idx) => {
                Return::Local(universe.lookup_field(*idx))
            }
            Self::ArgRead(scope, idx) => {
                Return::Local(universe.lookup_arg(*idx, *scope))
            }
            Self::GlobalRead(name) =>
                match name.as_str() {
                    "super" => Return::Local(universe.current_frame.get_self()),
                    _ => universe.lookup_global(name.as_str())
                        .map(Return::Local)
                        .or_else(|| {
                            let frame = universe.current_frame;
                            let self_value = frame.get_self();
                            universe.unknown_global(self_value, name.as_str())
                        })
                        .unwrap_or_else(|| Return::Exception(format!("global variable '{}' not found", name)))
                },
            Self::UnaryDispatch(un_op) => un_op.evaluate(universe),
            Self::BinaryDispatch(bin_op) => bin_op.evaluate(universe),
            Self::TernaryDispatch(ter_op) => ter_op.evaluate(universe),
            Self::NAryDispatch(msg) => msg.evaluate(universe),
            Self::SuperMessage(msg) => msg.evaluate(universe),
            Self::InlinedCall(inlined_node) => {
                match inlined_node.as_mut() {
                    InlinedNode::IfInlined(if_inlined) => if_inlined.evaluate(universe),
                    InlinedNode::IfTrueIfFalseInlined(if_true_if_false_inlined) => if_true_if_false_inlined.evaluate(universe),
                    InlinedNode::WhileInlined(while_inlined) => while_inlined.evaluate(universe),
                    InlinedNode::OrInlined(or_inlined) => or_inlined.evaluate(universe),
                    InlinedNode::AndInlined(and_inlined) => and_inlined.evaluate(universe)
                }
            }
        }
    }
}

impl Evaluate for ast::Literal {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        match self {
            Self::Array(array) => {
                let mut output = Vec::with_capacity(array.len());
                for literal in array {
                    let value = propagate!(literal.evaluate(universe));
                    output.push(value);
                }
                Return::Local(Value::Array(GCRef::<Vec<Value>>::alloc(output, &mut universe.gc_interface)))
            }
            Self::Integer(int) => Return::Local(Value::Integer(*int)),
            Self::BigInteger(int) => match int.parse() {
                Ok(value) => Return::Local(Value::BigInteger(GCRef::<BigInt>::alloc(value, &mut universe.gc_interface))),
                Err(err) => Return::Exception(err.to_string()),
            },
            Self::Double(double) => Return::Local(Value::Double(*double)),
            Self::Symbol(sym) => Return::Local(Value::Symbol(universe.intern_symbol(sym))),
            Self::String(string) => Return::Local(Value::String(GCRef::<String>::alloc(string.clone(), &mut universe.gc_interface))),
        }
    }
}

impl Evaluate for AstTerm {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        self.body.evaluate(universe)
    }
}

impl Evaluate for GCRef<AstBlock> {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let block = Block {
            block: *self,
            frame: universe.current_frame,
        };
        let block_ptr = GCRef::<Block>::alloc(block, &mut universe.gc_interface);
        Return::Local(Value::Block(block_ptr))
    }
}

impl AstDispatchNode {
    #[inline(always)]
    fn lookup_invokable(&mut self, receiver: &Value, universe: &mut UniverseAST) -> (Option<GCRef<Method>>, bool) {
        let mut is_cache_hit = false;
        let invokable = match &self.inline_cache {
            Some((cached_rcvr_ptr, method)) => {
                if std::ptr::eq(*cached_rcvr_ptr, receiver.class(universe).as_ptr()) {
                    // dbg!("cache hit");
                    is_cache_hit = true;
                    Some(*method)
                } else {
                    // dbg!("cache miss");
                    receiver.lookup_method(universe, &self.signature)
                }
            }
            None => receiver.lookup_method(universe, &self.signature)
        };

        (invokable, is_cache_hit)
    }
    
    #[inline(always)]
    fn dispatch_or_dnu(&mut self, invokable: Option<GCRef<Method>>, args: Vec<Value>, is_cache_hit: bool, universe: &mut UniverseAST) -> Return {
        match invokable {
            Some(invokable) => {
                
                match is_cache_hit {
                    true => invokable.to_obj().invoke(universe, args),
                    false => {
                        let receiver = args.first().unwrap().clone();
                        let invoke_ret = invokable.to_obj().invoke(universe, args);

                        let class_ref = receiver.class(universe);
                        let rcvr_ptr = class_ref.as_ptr(); // first arg is the receiver
                        self.inline_cache = Some((rcvr_ptr, invokable));

                        invoke_ret
                    }
                }
            }
            None => {
                let mut args = args;
                let receiver = args.remove(0);
                universe
                    .does_not_understand(receiver.clone(), &self.signature, args)
                    .unwrap_or_else(|| {
                        Return::Exception(format!(
                            "could not find method '{}>>#{}'",
                            receiver.class(universe).borrow().name(),
                            self.signature
                        ))
                    })
            }
        }
    }
}

impl Evaluate for AstUnaryDispatch {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let receiver = propagate!(self.dispatch_node.receiver.evaluate(universe));
        let (invokable, is_cache_hit) = self.dispatch_node.lookup_invokable(&receiver, universe);
        self.dispatch_node.dispatch_or_dnu(invokable, vec![receiver], is_cache_hit, universe)
    }
}

impl Evaluate for AstBinaryDispatch {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let receiver = propagate!(self.dispatch_node.receiver.evaluate(universe));
        let (invokable, is_cache_hit) = self.dispatch_node.lookup_invokable(&receiver, universe);

        let arg = propagate!(self.arg.evaluate(universe));

        self.dispatch_node.dispatch_or_dnu(invokable, vec![receiver, arg], is_cache_hit, universe)
    }
}

impl Evaluate for AstTernaryDispatch {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let receiver = propagate!(self.dispatch_node.receiver.evaluate(universe));
        let (invokable, is_cache_hit) = self.dispatch_node.lookup_invokable(&receiver, universe);

        let arg1 = propagate!(self.arg1.evaluate(universe));
        let arg2 = propagate!(self.arg2.evaluate(universe));

        self.dispatch_node.dispatch_or_dnu(invokable, vec![receiver, arg1, arg2], is_cache_hit, universe)
    }
}

impl Evaluate for AstNAryDispatch {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let receiver = propagate!(self.dispatch_node.receiver.evaluate(universe));
        let (invokable, is_cache_hit) = self.dispatch_node.lookup_invokable(&receiver, universe);

        let args = {
            let mut output = Vec::with_capacity(self.values.len() + 1);
            output.push(receiver.clone());
            for expr in &mut self.values {
                let value = propagate!(expr.evaluate(universe));
                output.push(value);
            }
            output
        };

        debug_assert!(args.len() > 3, "should be a specialized unary/binary/ternary node, not a generic N-ary node");

        self.dispatch_node.dispatch_or_dnu(invokable, args, is_cache_hit, universe)
    }
}

impl Evaluate for AstSuperMessage {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let invokable = self.super_class.to_obj().lookup_method(&self.signature);
        let receiver = universe.current_frame.get_self();
        let args = {
            let mut output = Vec::with_capacity(self.values.len() + 1);
            output.push(receiver.clone());
            for expr in &mut self.values {
                let value = propagate!(expr.evaluate(universe));
                output.push(value);
            }
            output
        };

        let value = match invokable {
            Some(invokable) => invokable.to_obj().invoke(universe, args),
            None => {
                let mut args = args;
                args.remove(0);
                universe
                    .does_not_understand(receiver.clone(), &self.signature, args)
                    .unwrap_or_else(|| {
                        Return::Exception(format!(
                            "could not find method '{}>>#{}'",
                            receiver.class(universe).borrow().name(),
                            self.signature
                        ))
                        // Return::Local(Value::Nil)
                    })
            }
        };

        value
    }
}


impl Evaluate for AstBody {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let mut last_value = Value::Nil;
        for expr in &mut self.exprs {
            last_value = propagate!(expr.evaluate(universe));
        }
        Return::Local(last_value)
    }
}

impl Evaluate for AstMethodDef {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        let current_frame = universe.current_frame;

        loop {
            match self.body.evaluate(universe) {
                Return::NonLocal(value, frame) => {
                    if current_frame == frame {
                        break Return::Local(value);
                    } else {
                        break Return::NonLocal(value, frame);
                    }
                }
                Return::Local(_) => break Return::Local(current_frame.get_self()),
                Return::Exception(msg) => break Return::Exception(msg),
                Return::Restart => continue,
            }
        }
    }
}

impl Evaluate for GCRef<Block> {
    fn evaluate(&mut self, universe: &mut UniverseAST) -> Return {
        self.to_obj().block.to_obj().body.evaluate(universe)
    }
}