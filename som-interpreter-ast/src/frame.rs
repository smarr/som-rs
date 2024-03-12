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

    // todo remove this legacy func, override it with next ones
    /// Search for a local binding.
    pub fn lookup_local(&self, name: impl AsRef<str>) -> Option<Value> {
        let name = name.as_ref();
        if let Some(value) = self.bindings.get(name).cloned() {
            return Some(value);
        }
        match &self.kind {
            FrameKind::Method {
                self_value, holder, ..
            } => {
                if holder.borrow().is_static {
                    holder.borrow().lookup_local(name)
                } else {
                    self_value.lookup_local(name)
                }
            }
            FrameKind::Block { block, .. } => block.frame.borrow().lookup_local(name),
        }
    }

    #[inline] // not sure if necessary
    pub fn lookup_local_1(&self, name: impl AsRef<str>) -> Option<Value> {
        self.bindings.get(name.as_ref()).cloned()
    }

    pub fn lookup_non_local_1(&self, name: impl AsRef<str>, scope: usize) -> Option<Value> {
        let mut current_frame: Option<Rc<RefCell<Frame>>> = match &self.kind {
            FrameKind::Block { block, .. } => {
                Some(Rc::clone(&block.frame))
            }
            _ => panic!("attempting to read a non local var from a method instead of a block.")
        };

        for _ in 0..scope - 1 {
            current_frame = match &Rc::clone(&current_frame.unwrap()).borrow().kind {
                FrameKind::Block { block, .. } => {
                    Some(Rc::clone(&block.frame))
                }
                _ => panic!("attempting to read a non local var from a method instead of a block.")
            };
        }

        match current_frame {
            Some(v) => v.borrow().lookup_local_1(name.as_ref()),
            None => panic!("non local read error: scope was equal to 0, so it should have been a local read.")
        }
    }

    /// Assign to a local binding.
    pub fn assign_local(&mut self, name: impl AsRef<str>, value: Value) -> Option<()> {
        let name = name.as_ref();
        if let Some(local) = self.bindings.get_mut(name) {
            *local = value;
            return Some(());
        }
        match &mut self.kind {
            FrameKind::Method {
                self_value, holder, ..
            } => {
                if holder.borrow().is_static {
                    holder.borrow_mut().assign_local(name, value)
                } else {
                    self_value.assign_local(name, value)
                }
            }
            FrameKind::Block { block, .. } => block.frame.borrow_mut().assign_local(name, value),
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
