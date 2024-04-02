use std::cell::{RefCell};
use std::rc::Rc;

use crate::block::Block;
use crate::class::Class;
use crate::interner::Interned;
use crate::value::Value;
use crate::SOMRef;

/// The kind of a given frame.
// #[cfg(feature = "frame-debug-info")]
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
    // #[cfg(feature = "frame-debug-info")]
    pub kind: FrameKind,
    /// Local variables that get defined within this frame.
    pub locals: Vec<Value>,
    /// Parameters for this frame.
    pub params: Vec<Value>,
}

impl Frame {
    /// Construct a new empty frame from its kind.
    pub fn from_kind(kind: FrameKind, nbr_locals: usize, self_value: Value) -> Self {
        let mut frame = Self {
            kind,
            locals: vec![Value::Nil; nbr_locals],
            params: vec![], // can we statically determine the length to not have to init it later? it's not straightforward as it turns out, but *should* be doable...
        };
        frame.params.push(self_value);
        frame
    }

    /// Get the frame's kind.
    pub fn kind(&self) -> &FrameKind {
        &self.kind
    }

    /// Get the self value for this frame.
    pub fn get_self(&self) -> Value {
        let mut self_or_block_self = self.params.get(0).unwrap().clone(); // todo should that really be a clone?

        // let oracle = match &self.kind {
        //     FrameKind::Method { self_value, .. } => self_value.clone(),
        //     FrameKind::Block { block, .. } => block.frame.borrow().get_self(),
        // };
        //
        // assert_eq!(new, oracle);
        self_or_block_self = match self_or_block_self {
            Value::BlockSelf(b) => b.frame.borrow().get_self(),
            _ => self_or_block_self
        };

        self_or_block_self
    }

    /// Get the holder for this current method.
    pub fn get_method_holder(&self) -> SOMRef<Class> {
        let ours = match self.get_self() {
            Value::Class(c) => c,
            v => todo!("{:?}", v)
        };

        let res = ours.clone().borrow().class();

        res
        // dbg!(&ours.borrow().class());

        // let oracle = match &self.kind {
        //     FrameKind::Method { holder, .. } => holder.clone(),
        //     FrameKind::Block { block, .. } => block.frame.borrow().get_method_holder(),
        // };

        // dbg!(&ours, &oracle);
        // oracle
    }

    /// Get the signature of the current method.
    #[cfg(feature = "frame-debug-info")]
    pub fn get_method_signature(&self) -> Interned {
        match &self.kind {
            FrameKind::Method { signature, .. } => *signature,
            FrameKind::Block { block, .. } => block.frame.borrow().get_method_signature(),
        }
    }

    #[inline] // not sure if necessary
    pub fn lookup_local(&self, idx: usize) -> Option<Value> {
        let res = self.locals.get(idx).cloned();

        res
    }

    pub fn lookup_non_local(&self, idx: usize, scope: usize) -> Option<Value> {
        let res = self.nth_frame_back(scope).borrow().lookup_local(idx);
        res
    }
    pub fn assign_local(&mut self, idx: usize, value: &Value) -> Option<()> {
        let local = self.locals.get_mut(idx).unwrap();
        *local = value.clone();
        Some(())
    }

    pub fn assign_non_local(&mut self, idx: usize, scope: usize, value: &Value) -> Option<()> {
        self.nth_frame_back(scope).borrow_mut().assign_local(idx, value)
    }

    pub fn lookup_arg(&self, idx: usize, scope: usize) -> Option<Value> {
        if idx == 0 && scope == 0 {
            return Some(self.get_self());
        }

        let res = match scope {
            0 => self.lookup_local_arg(idx),
            _ => self.lookup_non_local_arg(idx, scope),
        };

        res
    }

    pub fn lookup_local_arg(&self, idx: usize) -> Option<Value> {
        self.params.get(idx).cloned()
    }

    pub fn lookup_non_local_arg(&self, idx: usize, scope: usize) -> Option<Value> {
        self.nth_frame_back(scope).borrow().lookup_local_arg(idx)
    }

    pub fn assign_arg(&mut self, idx: usize, scope: usize, value: &Value) -> Option<()> {
        // todo make this one rely on the same logic as the others. but if reading args work, this is likely trivial
        if let Some(val) = self.params.get_mut(idx) {
            *val = value.clone();
            return Some(());
        } else {
            return match &mut self.kind {
                FrameKind::Method { ref mut self_value, .. } => {
                    self_value.assign_local(idx, value)
                }
                FrameKind::Block { block, .. } => block.frame.borrow_mut().assign_arg(idx, scope, value),
            };
        }
    }

    pub fn lookup_field(&self, idx: usize, kind: bool) -> Option<Value> {
        let self_val = self.get_self();

        // todo i am growing more and more confident the "kind" arg is useless. remove it when done with the variable arrays refactorings + frame refactorings
        let res = match kind {
            true => {
                match self_val {
                    Value::Instance(i) => { i.borrow_mut().lookup_local(idx) }
                    // Value::Class(c) => { c.borrow_mut().lookup_local(idx) }
                    v => { panic!("{:?}", &v) }
                }
            }
            false => {
                match self_val {
                    // Value::Instance(i) => { i.borrow().class.borrow().class().borrow_mut().lookup_local(idx) }
                    Value::Class(c) => { c.borrow().class().borrow_mut().lookup_local(idx) }
                    v => { panic!("{:?}", &v) }
                }
            }
        };

        res
    }

    pub fn assign_field(&self, idx: usize, _kind: bool, value: &Value) -> Option<()> {
        let self_val = self.get_self();

        // dbg!(&_kind);

        match _kind {
            true => {
                match self_val {
                    Value::Instance(i) => { i.borrow_mut().assign_local(idx, value.clone()) }
                    // Value::Class(c) => { c.borrow_mut().assign_local(idx, &value) }
                    v => { panic!("{:?}", &v) }
                }
            }
            false => {
                match self_val {
                    // Value::Instance(i) => { i.borrow().class.borrow().class().borrow_mut().assign_local(idx, &value) }
                    Value::Class(c) => { c.borrow().class().borrow_mut().assign_local(idx, &value) }
                    v => { panic!("{:?}", &v) }
                }
            }
        };

        Some(())
    }

    pub fn nth_frame_back(&self, n: usize) -> SOMRef<Frame> {
        let mut target_frame: Rc<RefCell<Frame>> = match &self.kind {
            FrameKind::Block { block, .. } => {
                Rc::clone(&block.frame)
            }
            _ => panic!("attempting to access a non local var/arg from a method instead of a block.")
        };

        for _ in 1..n {
            target_frame = match &Rc::clone(&target_frame).borrow().kind {
                FrameKind::Block { block, .. } => {
                    Rc::clone(&block.frame)
                }
                _ => panic!("attempting to access a non local var/arg from a method instead of a block.")
            };
        }

        target_frame
    }

    /// Get the method invocation frame for that frame.
    pub fn method_frame(frame: &SOMRef<Frame>) -> SOMRef<Frame> {
        match frame.borrow().kind() {
            FrameKind::Block { block, .. } => Frame::method_frame(&block.frame),
            FrameKind::Method { .. } => frame.clone(),
        }
    }
}