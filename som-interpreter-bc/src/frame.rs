use std::cell::RefCell;
use std::rc::Rc;

use som_core::bytecode::Bytecode;

use crate::block::Block;
use crate::class::Class;
use crate::compiler::Literal;
use crate::method::{Method, MethodKind};
use crate::value::Value;
use crate::SOMRef;

/// The kind of a given frame.
#[derive(Clone)]
pub enum FrameKind {
    /// A frame created from a block evaluation.
    Block {
        /// The block instance for the current frame.
        block: Rc<Block>,
    },
    /// A frame created from a method invocation.
    Method {
        /// The holder of the current method (used for lexical self/super).
        holder: SOMRef<Class>,
        /// The current method.
        method: Rc<Method>,
        /// The self value.
        self_value: Value,
    },
}

/// Represents a stack frame.
pub struct Frame {
    /// This frame's kind.
    // #[cfg(feature = "frame-debug-info")]
    pub kind: FrameKind,
    /// The bytecodes associated with the frame.
    pub bytecodes: Vec<Bytecode>,
    /// The arguments within this frame.
    pub args: Vec<Value>,
    /// The bindings within this frame.
    pub locals: Vec<Value>,
    /// Literals/constants associated with the frame.
    pub literals: Vec<Literal>,
    /// Bytecode index.
    pub bytecode_idx: usize,
}

impl Frame {
    /// Construct a new empty frame from its kind.
    pub fn from_kind(kind: FrameKind) -> Self {
        match &kind {
            FrameKind::Block { block } => {
                // let locals = block.blk_info.locals.iter().map(|_| Value::Nil).collect();
                let locals =  (0..block.blk_info.nb_locals).map(|_| Value::Nil).collect();
                let frame = Self {
                    locals,
                    args: vec![Value::BlockSelf(Rc::clone(&block))],
                    literals: block.blk_info.literals.clone(),
                    bytecodes: block.blk_info.body.clone(),
                    bytecode_idx: 0,
                    kind
                };
                frame
            }
            FrameKind::Method { method, .. } => {
                if let MethodKind::Defined(env) = method.kind() {
                    // let locals = env.locals.iter().map(|_| Value::Nil).collect();
                    let locals =  (0..env.nbr_locals).map(|_| Value::Nil).collect();
                    Self {
                        locals,
                        args: vec![], //todo might as well initialize it with self_value there right?
                        literals: env.literals.clone(),
                        bytecodes: env.body.clone(),
                        bytecode_idx: 0,
                        kind
                    }
                } else {
                    Self {
                        locals: vec![],
                        args: vec![],
                        literals: vec![],
                        bytecodes: vec![],
                        bytecode_idx: 0,
                        kind
                    }
                }
            }
        }
    }

    // #[cfg(feature = "frame-debug-info")]
    /// Get the frame's kind.
    pub fn kind(&self) -> &FrameKind {
        &self.kind
    }

    /// Get the self value for this frame.
    pub fn get_self(&self) -> Value {
        // match self.args.get(0).unwrap() {
        //     Value::BlockSelf(b) => Rc::clone(&b.frame.unwrap()).borrow().get_self(),
        //     s => s.clone()
        // }

        match self.args.get(0).unwrap() {
            Value::BlockSelf(b) => {
                let block_frame = b.frame.as_ref().unwrap().clone();
                let x = block_frame.borrow().get_self();
                x
            },
            s => s.clone()
        }
    }

    /// Get the holder for this current method.
    pub fn get_method_holder(&self) -> SOMRef<Class> {
        // todo!()
        match &self.kind {
            FrameKind::Method { holder, .. } => holder.clone(),
            FrameKind::Block { block, .. } => {
                block.frame.as_ref().unwrap().borrow().get_method_holder()
            }
        }
    }

    /// Get the current method itself.
    pub fn get_method(&self) -> Rc<Method> {
        todo!()
        // match &self.kind {
        //     FrameKind::Method { method, .. } => method.clone(),
        //     FrameKind::Block { block, .. } => block.frame.as_ref().unwrap().borrow().get_method(),
        // }
    }
    
    pub fn get_bytecode(&self, idx: usize) -> Option<Bytecode> {
        self.bytecodes.get(idx).cloned()
    }

    pub fn lookup_constant(&self, idx: usize) -> Option<Literal> {
        self.literals.get(idx).cloned()
    }

    pub fn lookup_argument(&self, idx: usize) -> Option<Value> {
        self.args.get(idx).cloned()
    }

    /// Search for a local binding.
    pub fn lookup_local(&self, idx: usize) -> Option<Value> {
        self.locals.get(idx).cloned()
        // if let Some(value) = self.locals.get(idx).cloned() {
        //     return Some(value);
        // }
        // match &self.kind {
        //     FrameKind::Method {
        //         self_value, holder, ..
        //     } => {
        //         if holder.borrow().is_static {
        //             holder.borrow().lookup_local(idx)
        //         } else {
        //             self_value.lookup_local(idx)
        //         }
        //     }
        //     FrameKind::Block { block, .. } => {
        //         block.frame.as_ref().unwrap().borrow().lookup_local(idx)
        //     }
        // }
    }

    /// Assign to a local binding.
    pub fn assign_local(&mut self, idx: usize, value: Value) -> Option<()> {
        // if let Some(local) = self.locals.get_mut(idx) {
        //     *local = value;
        //     return Some(());
        // }
        // match &mut self.kind {
        //     FrameKind::Method {
        //         self_value, holder, ..
        //     } => {
        //         if holder.borrow().is_static {
        //             holder.borrow_mut().assign_local(idx, value)
        //         } else {
        //             self_value.assign_local(idx, value)
        //         }
        //     }
        //     FrameKind::Block { block, .. } => block
        //         .frame
        //         .as_ref()
        //         .unwrap()
        //         .borrow_mut()
        //         .assign_local(idx, value),
        // }
        self.locals.get_mut(idx).map(|local| *local = value)
    }

//    #[cfg(feature = "frame-debug-info")]
    /// Get the method invocation frame for that frame.
    pub fn method_frame(frame: &SOMRef<Frame>) -> SOMRef<Frame> {
        match frame.borrow().kind() {
            FrameKind::Block { block, .. } => Frame::method_frame(block.frame.as_ref().unwrap()),
            FrameKind::Method { .. } => frame.clone(),
        }
    }

    pub fn nth_frame_back(current_frame: SOMRef<Frame>, n: u8) -> SOMRef<Frame> {
        if n == 0 {
            return current_frame;
        }

        let mut target_frame: Rc<RefCell<Frame>> = match current_frame.borrow().args.get(0).unwrap() {
            Value::BlockSelf(block) => {
                Rc::clone(&block.frame.as_ref().unwrap())
            }
            v => panic!("attempting to access a non local var/arg from a method instead of a block: self wasn't blockself but {:?}.", v)
        };
        for _ in 1..n {
            target_frame = match Rc::clone(&target_frame).borrow().args.get(0).unwrap() {
                Value::BlockSelf(block) => {
                    Rc::clone(&block.frame.as_ref().unwrap())
                }
                v => panic!("attempting to access a non local var/arg from a method instead of a block (but the original frame we were in was a block): self wasn't blockself but {:?}.", v)
            };
        }
        target_frame
    }
}
