use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use som_core::bytecode::Bytecode;

use crate::block::Block;
use crate::class::Class;
use crate::compiler::Literal;
use crate::frame::Frame;
use crate::interner::Interned;
use crate::method::{Method, MethodKind};
use crate::universe::UniverseBC;
use crate::value::Value;
use crate::SOMRef;

#[cfg(feature = "profiler")]
use crate::profiler::Profiler;

const INT_0: Value = Value::Integer(0);
const INT_1: Value = Value::Integer(1);

macro_rules! send {
    ($interp:expr, $universe:expr, $frame:expr, $lit_idx:expr, $nb_params:expr) => {{
        let literal = $frame.borrow().lookup_constant($lit_idx as usize).unwrap();
        let Literal::Symbol(symbol) = literal else {
            return None;
        };
        let nb_params = match $nb_params {
            Some(v) => v,
            None => {
                let signature = $universe.lookup_symbol(symbol);
                nb_params(signature)
            }
        };
        let method = {
            // dbg!($universe.lookup_symbol(symbol));
            let receiver = $interp.stack.iter().nth_back(nb_params)?;
            let receiver_class = receiver.class($universe);
            resolve_method($frame, &receiver_class, symbol, $interp.bytecode_idx)
        };
        do_send($interp, $universe, method, symbol, nb_params as usize);
    }};
}

macro_rules! super_send {
    ($interp:expr, $universe:expr, $frame_expr:expr, $lit_idx:expr, $nb_params:expr) => {{
        let literal = $frame_expr
            .borrow()
            .lookup_constant($lit_idx as usize)
            .unwrap();
        let Literal::Symbol(symbol) = literal else {
            return None;
        };
        let nb_params = match $nb_params {
            Some(v) => v,
            None => {
                let signature = $universe.lookup_symbol(symbol);
                nb_params(signature)
            }
        };
        let method = {
            // dbg!($universe.lookup_symbol(symbol));
            let holder = $frame_expr.borrow().get_method_holder();
            let super_class = holder.borrow().super_class().unwrap();
            resolve_method($frame_expr, &super_class, symbol, $interp.bytecode_idx)
        };
        do_send($interp, $universe, method, symbol, nb_params as usize);
    }};
}

pub struct Interpreter {
    /// The interpreter's stack frames.
    pub frames: Vec<SOMRef<Frame>>,
    /// The evaluation stack.
    pub stack: Vec<Value>,
    /// The time record of the interpreter's creation.
    pub start_time: Instant,
    /// The current bytecode index.
    pub bytecode_idx: usize,
    /// The current frame.
    pub current_frame: SOMRef<Frame>,
    /// Pointer to the frame's bytecodes, to not have to read them from the frame directly
    pub current_bytecodes: *const Vec<Bytecode>,
}

impl Interpreter {
    pub fn new(base_frame: SOMRef<Frame>) -> Self {
        Self {
            frames: vec![Rc::clone(&base_frame)],
            stack: vec![],
            start_time: Instant::now(),
            bytecode_idx: 0,
            current_frame: Rc::clone(&base_frame),
            current_bytecodes: base_frame.borrow_mut().bytecodes,
        }
    }

    pub fn push_method_frame(&mut self, method: Rc<Method>, args: Vec<Value>) -> SOMRef<Frame> {
        let frame = Rc::new(RefCell::new(Frame::from_method(method, args)));
        self.frames.push(frame.clone());
        self.bytecode_idx = 0;
        self.current_bytecodes = frame.borrow_mut().bytecodes;
        self.current_frame = Rc::clone(&frame);
        frame
    }

    pub fn push_block_frame(&mut self, block: Rc<Block>, args: Vec<Value>) -> SOMRef<Frame> {
        let frame = Rc::new(RefCell::new(Frame::from_block(block, args)));
        self.frames.push(frame.clone());
        self.bytecode_idx = 0;
        self.current_bytecodes = frame.borrow_mut().bytecodes;
        self.current_frame = Rc::clone(&frame);
        frame
    }

    pub fn pop_frame(&mut self) {
        self.frames.pop();

        match self.frames.last().cloned() {
            None => {}
            Some(f) => {
                self.bytecode_idx = f.borrow().bytecode_idx;
                self.current_frame = Rc::clone(&f);
                self.current_bytecodes = f.borrow_mut().bytecodes;
            }
        }
    }

    pub fn pop_n_frames(&mut self, n: usize) {
        (0..n).for_each(|_| { self.frames.pop(); });

        match self.frames.last().cloned() {
            None => {}
            Some(f) => {
                self.bytecode_idx = f.borrow().bytecode_idx;
                self.current_frame = Rc::clone(&f);
                self.current_bytecodes = f.borrow_mut().bytecodes;
            }
        }
    }

    pub fn run(&mut self, universe: &mut UniverseBC) -> Option<Value> {
        loop {
            let frame = Rc::clone(&self.current_frame);

            // Actually safe, there's always a reference to the current bytecodes. Need unsafe because we want to store a ref for quick access in perf-critical code
            let bytecode = *(unsafe { (*self.current_bytecodes).get_unchecked(self.bytecode_idx) });

            self.bytecode_idx += 1;

            match bytecode {
                Bytecode::Halt => {
                    return Some(Value::Nil);
                }
                Bytecode::Dup => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("DUP", "bytecodes");
                    let value = self.stack.last().cloned().unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Inc => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("INC", "bytecodes");
                    match self.stack.last_mut().unwrap() {
                        Value::Integer(v) => { *v += 1 }
                        Value::BigInteger(v) => { *v += 1 }
                        Value::Double(v) => { *v += 1.0 }
                        _ => panic!("Invalid type")
                    };
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Dec => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("DEC", "bytecodes");
                    match self.stack.last_mut().unwrap() {
                        Value::Integer(v) => { *v -= 1 }
                        Value::BigInteger(v) => { *v -= 1 }
                        Value::Double(v) => { *v -= 1.0 }
                        _ => panic!("Invalid type")
                    };
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushLocal(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_LOCAL", "bytecodes");
                    let value = frame.borrow().lookup_local(idx as usize).unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushNonLocal(up_idx, idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_NON_LOCAL", "bytecodes");
                    debug_assert_ne!(up_idx, 0);
                    let from = Frame::nth_frame_back(frame, up_idx);
                    let value = from.borrow().lookup_local(idx as usize).unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushArg(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_ARG", "bytecodes");
                    debug_assert_ne!(idx, 0); // that's a ReturnSelf case.
                    let value = frame.borrow().lookup_argument(idx as usize).unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushNonLocalArg(up_idx, idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSHNONLOCAL_ARG", "bytecodes");
                    debug_assert_ne!(up_idx, 0);
                    debug_assert_ne!((up_idx, idx), (0, 0)); // that's a ReturnSelf case.
                    let from = Frame::nth_frame_back(frame, up_idx);
                    let value = from.borrow().lookup_argument(idx as usize).unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushField(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_FIELD", "bytecodes");
                    let value = match frame.borrow().get_self() {
                        Value::Instance(i) => { i.borrow_mut().lookup_local(idx as usize) }
                        Value::Class(c) => { c.borrow().class().borrow_mut().lookup_local(idx as usize) }
                        v => { panic!("trying to read a field from a {:?}", &v) }
                    };
                    self.stack.push(value.unwrap());
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushBlock(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_BLOCK", "bytecodes");
                    let literal = frame.borrow().lookup_constant(idx as usize).unwrap();
                    let mut block = match literal {
                        Literal::Block(blk) => Block::clone(&blk),
                        _ => panic!("PushBlock expected a block, but got another invalid literal"),
                    };
                    block.frame.replace(Rc::clone(&frame));
                    self.stack.push(Value::Block(Rc::new(block)));
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushConstant(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_CONSTANT", "bytecodes");
                    let literal = frame.borrow().lookup_constant(idx as usize).unwrap();
                    let value = convert_literal(&frame, literal).unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushConstant0 => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_CONSTANT_0", "bytecodes");
                    let literal = frame.borrow().lookup_constant(0).unwrap();
                    let value = convert_literal(&frame, literal).unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushConstant1 => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_CONSTANT_1", "bytecodes");
                    let literal = frame.borrow().lookup_constant(1).unwrap();
                    let value = convert_literal(&frame, literal).unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushConstant2 => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_CONSTANT_2", "bytecodes");
                    let literal = frame.borrow().lookup_constant(2).unwrap();
                    let value = convert_literal(&frame, literal).unwrap();
                    self.stack.push(value);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushGlobal(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_GLOBAL", "bytecodes");
                    let literal = frame.borrow().lookup_constant(idx as usize).unwrap();
                    let symbol = match literal {
                        Literal::Symbol(sym) => sym,
                        _ => panic!("Global is not a symbol."),
                    };
                    if let Some(value) = universe.lookup_global(symbol) {
                        self.stack.push(value);
                    } else {
                        let self_value = frame.borrow().get_self();
                        universe.unknown_global(self, self_value, symbol).unwrap();
                    }
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Push0 => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_0", "bytecodes");
                    self.stack.push(INT_0);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Push1 => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_1", "bytecodes");
                    self.stack.push(INT_1);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushNil => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_NIL", "bytecodes");
                    self.stack.push(Value::Nil);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PushSelf => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("PUSH_SELF", "bytecodes");
                    self.stack.push(frame.borrow().lookup_argument(0).unwrap());
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Pop => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("POP", "bytecodes");
                    self.stack.pop();
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Pop2 => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("POP2", "bytecodes");
                    self.stack.remove(self.stack.len() - 2);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PopLocal(up_idx, idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("POP_LOCAL", "bytecodes");
                    let value = self.stack.pop().unwrap();
                    let from = Frame::nth_frame_back(frame, up_idx);
                    from.borrow_mut().assign_local(idx as usize, value).unwrap();
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PopArg(up_idx, idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("POP_ARG", "bytecodes");
                    let value = self.stack.pop().unwrap();
                    let from = Frame::nth_frame_back(frame, up_idx);
                    from.borrow_mut()
                        .args
                        .get_mut(idx as usize)
                        .map(|loc| *loc = value)
                        .unwrap();
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::PopField(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("POP_FIELD", "bytecodes");
                    let value = self.stack.pop().unwrap();
                    match frame.borrow_mut().get_self() {
                        Value::Instance(i) => { i.borrow_mut().assign_local(idx as usize, value) }
                        Value::Class(c) => { c.borrow().class().borrow_mut().assign_local(idx as usize, value) }
                        v => { panic!("{:?}", &v) }
                    };
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Send1(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("SEND_1", "bytecodes");
                    send! {self, universe, &frame, idx, Some(0)} // Send1 => receiver + 0 args, so we pass Some(0)
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Send2(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("SEND_2", "bytecodes");
                    send! {self, universe, &frame, idx, Some(1)}
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::Send3(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("SEND_3", "bytecodes");
                    send! {self, universe, &frame, idx, Some(2)}
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::SendN(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("SEND_N", "bytecodes");
                    send! {self, universe, &frame, idx, None}
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::SuperSend1(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("SUPER_SEND_1", "bytecodes");
                    super_send! {self, universe, &frame, idx, Some(0)}
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::SuperSend2(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("SUPER_SEND_2", "bytecodes");
                    super_send! {self, universe, &frame, idx, Some(1)}
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::SuperSend3(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("SUPER_SEND_3", "bytecodes");
                    super_send! {self, universe, &frame, idx, Some(2)}
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::SuperSendN(idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("SUPER_SEND_N", "bytecodes");
                    super_send! {self, universe, &frame, idx, None}
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::ReturnSelf => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("RETURN_SELF", "bytecodes");
                    let self_val = frame.borrow().args.get(0).unwrap().clone();
                    self.pop_frame();
                    if self.frames.is_empty() {
                        #[cfg(feature = "profiler")]
                        Profiler::global().finish_detached_event(timing);
                        return Some(self_val);
                    }
                    self.stack.push(self_val);
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::ReturnLocal => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("RETURN_LOCAL", "bytecodes");
                    self.pop_frame();
                    if self.frames.is_empty() {
                        #[cfg(feature = "profiler")]
                        Profiler::global().finish_detached_event(timing);
                        return Some(self.stack.pop().unwrap_or(Value::Nil));
                    }
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::ReturnNonLocal(up_idx) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("RETURN_NON_LOCAL", "bytecodes");
                    let method_frame = Frame::nth_frame_back(Rc::clone(&frame), up_idx);
                    let escaped_frames = self
                        .frames
                        .iter()
                        .rev()
                        .position(|live_frame| Rc::ptr_eq(&live_frame, &method_frame));

                    if let Some(count) = escaped_frames {
                        self.pop_n_frames(count + 1);
                        if self.frames.is_empty() {
                            #[cfg(feature = "profiler")]
                            Profiler::global().finish_detached_event(timing);
                            return Some(self.stack.pop().unwrap_or(Value::Nil));
                        }
                    } else {
                        // NB: I did some changes there with the blockself bits and i'm not positive it works the same as before, but it should.

                        // Block has escaped its method frame.
                        let instance = frame.borrow().get_self();
                        let block = match frame.borrow().args.first().unwrap() {
                            Value::Block(block) => block.clone(),
                            _ => {
                                // Should never happen, because `universe.current_frame()` would
                                // have been equal to `universe.current_method_frame()`.
                                panic!("A method frame has escaped itself ??");
                            }
                        };

                        universe.escaped_block(self, instance, block).expect(
                            "A block has escaped and `escapedBlock:` is not defined on receiver",
                        );
                    }
                }
                Bytecode::Jump(offset) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("JUMP", "bytecodes");
                    self.bytecode_idx += offset - 1; // minus one because it gets incremented by one already every loop
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::JumpBackward(offset) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("JUMP_BACKWARDS", "bytecodes");
                    self.bytecode_idx -= offset + 1;
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::JumpOnTrueTopNil(offset) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("JUMP_ON_TRUE_TOP_NIL", "bytecodes");
                    let condition_result = self.stack.last()?;

                    match condition_result {
                        Value::Boolean(true) => {
                            self.bytecode_idx += offset - 1;
                            *self.stack.last_mut()? = Value::Nil;
                        }
                        Value::Boolean(false) => {
                            self.stack.pop();
                        }
                        _ => panic!("JumpOnTrueTopNil condition did not evaluate to boolean (was {:?})", condition_result),
                    }
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::JumpOnFalseTopNil(offset) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("JUMP_ON_FALSE_TOP_NIL", "bytecodes");
                    let condition_result = self.stack.last()?;

                    match condition_result {
                        Value::Boolean(false) => {
                            self.bytecode_idx += offset - 1;
                            *self.stack.last_mut()? = Value::Nil;
                        }
                        Value::Boolean(true) => {
                            self.stack.pop();
                        }
                        _ => panic!("JumpOnFalseTopNil condition did not evaluate to boolean (was {:?})", condition_result),
                    }
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::JumpOnTruePop(offset) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("JUMP_ON_TRUE_POP", "bytecodes");
                    let condition_result = self.stack.pop()?;

                    match condition_result {
                        Value::Boolean(true) => {
                            self.bytecode_idx += offset - 1;
                        }
                        Value::Boolean(false) => {}
                        _ => panic!("JumpOnTruePop condition did not evaluate to boolean (was {:?})", condition_result),
                    }
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
                Bytecode::JumpOnFalsePop(offset) => {
                    #[cfg(feature = "profiler")]
                    let timing = Profiler::global().start_detached_event("JUMP_ON_FALSE_POP", "bytecodes");
                    let condition_result = self.stack.pop()?;

                    match condition_result {
                        Value::Boolean(false) => {
                            self.bytecode_idx += offset - 1;
                        }
                        Value::Boolean(true) => {}
                        _ => panic!("JumpOnFalsePop condition did not evaluate to boolean (was {:?})", condition_result),
                    }
                    #[cfg(feature = "profiler")]
                    Profiler::global().finish_detached_event(timing);
                }
            }
        }

        pub fn do_send(
            interpreter: &mut Interpreter,
            universe: &mut UniverseBC,
            method: Option<Rc<Method>>,
            symbol: Interned,
            nb_params: usize,
        ) {
            let Some(method) = method else {
                let args = interpreter.stack.split_off(interpreter.stack.len() - nb_params);
                let self_value = interpreter.stack.pop().unwrap();

                universe.does_not_understand(interpreter, self_value, symbol, args)
                    .expect(
                        "A message cannot be handled and `doesNotUnderstand:arguments:` is not defined on receiver"
                    );

                return;
            };

            // we store the current bytecode idx to be able to correctly restore the bytecode state when we pop frames
            interpreter.current_frame.borrow_mut().bytecode_idx = interpreter.bytecode_idx;

            match method.kind() {
                MethodKind::Defined(_) => {
                    // let name = &method.holder.upgrade().unwrap().borrow().name.clone();
                    // let filter_list = ["Integer", "Vector", "True", "Pair"];
                    // let filter_list = [];

                    // if !filter_list.contains(&name.as_str()) {
                    //     eprintln!("Invoking {:?} (in {:?})", &method.signature, &method.holder.upgrade().unwrap().borrow().name);
                    // }

                    let args = interpreter.stack.split_off(interpreter.stack.len() - nb_params - 1);
                    interpreter.push_method_frame(method, args);
                }
                MethodKind::Primitive(func) => {
                    // eprintln!("Invoking prim {:?} (in {:?})", &method.signature, &method.holder.upgrade().unwrap().borrow().name);
                    func(interpreter, universe);
                }
                MethodKind::NotImplemented(err) => {
                    let self_value = interpreter.stack.iter().nth_back(nb_params).unwrap();
                    println!(
                        "{}>>#{}",
                        self_value.class(&universe).borrow().name(),
                        method.signature(),
                    );
                    panic!("Primitive `#{}` not implemented", err)
                }
            }
        }

        fn resolve_method(
            frame: &SOMRef<Frame>,
            class: &SOMRef<Class>,
            signature: Interned,
            bytecode_idx: usize,
        ) -> Option<Rc<Method>> {
            let mut inline_cache = unsafe {
                (*frame.borrow_mut().inline_cache).borrow_mut()
            };

            // SAFETY: this access is actually safe because the bytecode compiler
            // makes sure the cache has as many entries as there are bytecode instructions,
            // therefore we can avoid doing any redundant bounds checks here.
            let maybe_found = unsafe { inline_cache.get_unchecked_mut(bytecode_idx) };

            match maybe_found {
                Some((receiver, method)) if *receiver == class.as_ptr() => {
                    Some(Rc::clone(method))
                }
                place @ None => {
                    let found = class.borrow().lookup_method(signature);
                    *place = found
                        .clone()
                        .map(|method| (class.as_ptr() as *const _, method));
                    found
                }
                _ => class.borrow().lookup_method(signature),
            }
        }

        fn convert_literal(frame: &SOMRef<Frame>, literal: Literal) -> Option<Value> {
            let value = match literal {
                Literal::Symbol(sym) => Value::Symbol(sym),
                Literal::String(val) => Value::String(val),
                Literal::Double(val) => Value::Double(val),
                Literal::Integer(val) => Value::Integer(val),
                Literal::BigInteger(val) => Value::BigInteger(val),
                Literal::Array(val) => {
                    let arr = val
                        .into_iter()
                        .map(|idx| {
                            frame
                                .borrow()
                                .lookup_constant(idx as usize)
                                .and_then(|lit| convert_literal(frame, lit))
                        })
                        .collect::<Option<Vec<_>>>()
                        .unwrap();
                    Value::Array(Rc::new(RefCell::new(arr)))
                }
                Literal::Block(val) => Value::Block(val),
            };
            Some(value)
        }

        fn nb_params(signature: &str) -> usize {
            match signature.chars().nth(0) {
                Some(ch) if !ch.is_alphabetic() => 1,
                _ => signature.chars().filter(|ch| *ch == ':').count(),
            }
        }
    }
}
