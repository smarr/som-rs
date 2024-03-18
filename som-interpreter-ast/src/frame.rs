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
    pub bindings: HashMap<String, Value>,
}

impl Frame {
    /// Construct a new empty frame from its kind.
    pub fn from_kind(kind: FrameKind) -> Self {
        Self {
            kind,
            bindings: HashMap::new(),
        }
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
    pub fn lookup_local(&self, name: impl AsRef<str>) -> Option<Value> {
        self.bindings.get(name.as_ref()).cloned()
    }

    pub fn lookup_non_local(&self, name: impl AsRef<str>, scope: usize) -> Option<Value> {
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

        let l = current_frame.borrow().lookup_local(name.as_ref());
        l
    }

    pub fn lookup_field(&self, name: impl AsRef<str>) -> Option<Value> {
        match &self.kind {
            FrameKind::Block { block } => block.frame.borrow().lookup_field(name),
            FrameKind::Method { holder, self_value, .. } => {
                if let Some(value) = self.bindings.get(name.as_ref()).cloned() {
                    return Some(value);
                } else if holder.borrow().is_static {
                    holder.borrow().lookup_local(name)
                } else {
                    self_value.lookup_local(name)
                }
            }
        }
    }

    pub fn lookup_arg(&self, name: impl AsRef<str>) -> Option<Value> {
        if let Some(value) = self.bindings.get(name.as_ref()).cloned() {
            return Some(value);
        } else {
            return match &self.kind {
                FrameKind::Method { self_value, .. } => {
                    self_value.lookup_local(name) // confused by this, frankly. args as stored as method locals?
                }
                FrameKind::Block { block, .. } => block.frame.borrow().lookup_arg(name),
            }
        }
    }

    /// Assign to a local binding.
    pub fn assign_local(&mut self, name: impl AsRef<str>, value: Value) -> Option<()> {
        let local = self.bindings.get_mut(name.as_ref()).unwrap();
        *local = value;
        Some(())
    }

    pub fn assign_non_local(&mut self, name: impl AsRef<str>, scope: usize, value: Value) -> Option<()> {
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

        let x= current_frame.borrow_mut().assign_local(name.as_ref(), value);
        x
    }

    pub fn assign_field(&mut self, name: impl AsRef<str>, value: Value) -> Option<()> {
        match &mut self.kind {
            FrameKind::Block { block } => block.frame.borrow_mut().assign_field(name, value),
            FrameKind::Method { holder, ref mut self_value, .. } => {
                if let Some(val) = self.bindings.get_mut(name.as_ref()) {
                    *val = value;
                    return Some(());
                } else if holder.borrow().is_static {
                    holder.borrow_mut().assign_local(name, value)
                } else {
                    self_value.assign_local(name, value)
                }
            }
        }
    }

    pub fn assign_arg(&mut self, name: impl AsRef<str>, value: Value) -> Option<()> {
        if let Some(val) = self.bindings.get_mut(name.as_ref()) {
            *val = value;
            return Some(());
        } else {
            return match &mut self.kind {
                FrameKind::Method { ref mut self_value, .. } => {
                    self_value.assign_local(name, value)
                }
                FrameKind::Block { block, .. } => block.frame.borrow_mut().assign_arg(name, value),
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
