use std::cell::{RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::block::Block;
use crate::class::Class;
use crate::interner::Interned;
use crate::value::Value;
use crate::SOMRef;

/// The kind of a given frame.
#[derive(Debug, Clone)]
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
        signature: Interned,
        /// The self value.
        self_value: Value,
    },
}

/// Represents a stack frame.
#[derive(Debug)]
pub struct Frame {
    /// This frame's kind.
    pub kind: FrameKind,
    /// The bindings within this frame.
    pub bindings: HashMap<String, Value>, // todo used for... binaryops?
    /// Local variables that get defined within this frame.
    pub locals: Vec<Value>,
    /// Parameters for this frame.
    pub params: Vec<Value>,
}

impl Frame {
    /// Construct a new empty frame from its kind.
    pub fn from_kind(kind: FrameKind) -> Self {
        let mut frame = Self {
            kind,
            locals: vec![], // TODO we can statically determine the length of the locals array here and not have to init it later. does it matter for perf, though? probably not
            params: vec![], // ditto for params
            bindings: HashMap::new()
        };
        frame.params.push(frame.get_self());
        frame
    }

    /// Get the frame's kind.
    pub fn kind(&self) -> &FrameKind {
        &self.kind
    }

    /// Get the self value for this frame.
    pub fn get_self(&self) -> Value {
        match &self.kind {
            FrameKind::Method { self_value, .. } => self_value.clone(),
            FrameKind::Block { block, .. } => block.frame.borrow().get_self(),
        }
    }

    /// Get the holder for this current method.
    pub fn get_method_holder(&self) -> SOMRef<Class> {
        match &self.kind {
            FrameKind::Method { holder, .. } => holder.clone(),
            FrameKind::Block { block, .. } => block.frame.borrow().get_method_holder(),
        }
    }

    /// Get the signature of the current method.
    pub fn get_method_signature(&self) -> Interned {
        match &self.kind {
            FrameKind::Method { signature, .. } => *signature,
            FrameKind::Block { block, .. } => block.frame.borrow().get_method_signature(),
        }
    }

    #[inline] // not sure if necessary
    pub fn lookup_local(&self, idx: usize) -> Option<Value> {
        self.locals.get(idx).cloned()
    }

    pub fn lookup_non_local(&self, idx: usize, scope: usize) -> Option<Value> {
        let mut current_frame: Rc<RefCell<Frame>> = match &self.kind {
            FrameKind::Block { block, .. } => {
                Rc::clone(&block.frame)
            }
            _ => panic!("attempting to read a non local var from a method instead of a block.")
        };

        for _ in 1..scope {
            current_frame = match &Rc::clone(&current_frame).borrow().kind {
                FrameKind::Block { block, .. } => {
                    Rc::clone(&block.frame)
                }
                _ => panic!("attempting to read a non local var from a method instead of a block.")
            };
        }

        let l = current_frame.borrow().lookup_local(idx);
        l
    }

    pub fn lookup_field(&self, idx: usize, kind: bool) -> Option<Value> {
        match &self.kind {
            FrameKind::Block { block } => block.frame.borrow().lookup_field(idx, kind),
            FrameKind::Method { holder, self_value, .. } => {
                match kind {
                    true => self_value.lookup_local(idx),
                    false => {
                        if holder.borrow().is_static {
                            holder.borrow().lookup_local(idx)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    pub fn lookup_arg(&self, idx: usize, scope: usize) -> Option<Value> {
        match scope {
            0 => self.lookup_local_arg(idx),
            _ => self.lookup_non_local_arg(idx, scope),
        }
    }

    pub fn lookup_local_arg(&self, idx: usize) -> Option<Value> {
        self.params.get(idx).cloned()
    }

    pub fn lookup_non_local_arg(&self, idx: usize, scope: usize) -> Option<Value> {
        let mut current_frame: Rc<RefCell<Frame>> = match &self.kind {
            FrameKind::Block { block, .. } => {
                Rc::clone(&block.frame)
            },
            _ => panic!("looking up a non local arg from the root of a method?")
        };

        for _ in 1..scope {
            current_frame = match &Rc::clone(&current_frame).borrow().kind {
                FrameKind::Block { block, .. } => {
                    Rc::clone(&block.frame)
                }
                _ => panic!("...why is this never reached in practice? because methods contain a framekind::block?")
            };
        }

        let l = current_frame.borrow().lookup_local_arg(idx);
        l
    }


    /// Assign to a local binding.
    pub fn assign_local(&mut self, idx: usize, value: &Value) -> Option<()> {
        let local = self.locals.get_mut(idx).unwrap();
        *local = value.clone();
        Some(())
    }

    pub fn assign_non_local(&mut self, idx: usize, scope: usize, value: &Value) -> Option<()> {
        let mut current_frame: Rc<RefCell<Frame>> = match &self.kind {
            FrameKind::Block { block, .. } => {
                Rc::clone(&block.frame)
            }
            _ => panic!("attempting to read a non local var from a method instead of a block.")
        };

        for _ in 1..scope {
            current_frame = match &Rc::clone(&current_frame).borrow_mut().kind {
                FrameKind::Block { block, .. } => {
                    Rc::clone(&block.frame)
                }
                _ => panic!("attempting to read a non local var from a method instead of a block.")
            };
        }

        let x= current_frame.borrow_mut().assign_local(idx, value);
        x
    }

    pub fn assign_field(&mut self, idx: usize, kind: bool, value: &Value) -> Option<()> {
        match &mut self.kind {
            FrameKind::Block { block } => block.frame.borrow_mut().assign_field(idx, kind, value),
            FrameKind::Method { holder, ref mut self_value, .. } => {
                match kind {
                    true => self_value.assign_local(idx, value),
                    false => {
                        if holder.borrow().is_static {
                            holder.borrow_mut().assign_local(idx, value)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    pub fn assign_arg(&mut self, idx: usize, scope: usize, value: &Value) -> Option<()> {
        if let Some(val) = self.params.get_mut(idx) {
            *val = value.clone();
            return Some(());
        } else {
            return match &mut self.kind {
                FrameKind::Method { ref mut self_value, .. } => {
                    self_value.assign_local(idx, value)
                }
                FrameKind::Block { block, .. } => block.frame.borrow_mut().assign_arg(idx, scope, value),
            }
        }
    }

    /// Get the method invocation frame for that frame.
    pub fn method_frame(frame: &SOMRef<Frame>) -> SOMRef<Frame> {
        match frame.borrow().kind() {
            FrameKind::Block { block, .. } => Frame::method_frame(&block.frame),
            FrameKind::Method { .. } => frame.clone(),
        }
    }
}
