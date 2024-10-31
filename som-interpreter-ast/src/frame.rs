use crate::gc::FRAME_ARGS_PTR;
use crate::value::Value;
use core::mem::size_of;
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::{CustomAlloc, GCRef};
use std::marker::PhantomData;

/// The kind of a given frame.
// #[cfg(feature = "frame-debug-info")]
// #[derive(Debug, Clone)]
// pub enum FrameKind {
//     /// A frame created from a block evaluation.
//     Block {
//         /// The block instance for the current frame.
//         block: Rc<Block>,
//     },
//     /// A frame created from a method invocation.
//     Method {
//         /// The holder of the current method (used for lexical self/super).
//         holder: SOMRef<Class>,
//         /// The current method.
//         signature: Interned,
//         /// The self value.
//         self_value: Value,
//     },
// }

/// Represents a stack frame.
#[derive(Debug)]
pub struct Frame {
    pub prev_frame: GCRef<Frame>,
    /// This frame's kind.
    // #[cfg(feature = "frame-debug-info")]
    // pub kind: FrameKind,
    pub nbr_args: u8,
    pub nbr_locals: u8,
    /// Parameters for this frame.
    pub params_marker: PhantomData<Vec<Value>>,
    /// Local variables that get defined within this frame.
    pub locals_marker: PhantomData<Vec<Value>>,
}

impl Frame {
    /// Construct a new empty frame from its kind.
    // pub fn from_kind(kind: FrameKind, nbr_locals: usize, self_value: Value) -> Self {
    //     let mut frame = Self {
    //         kind,
    //         locals: vec![Value::Nil; nbr_locals],
    //         params: vec![], // can we statically determine the length to not have to init it later? it's not straightforward as it turns out, but *should* be doable...
    //     };
    //     frame.params.push(self_value);
    //     frame
    // }

    pub fn alloc_new_frame(
        nbr_locals: u8,
        mut params: Vec<Value>,
        prev_frame: GCRef<Frame>,
        gc_interface: &mut GCInterface,
    ) -> GCRef<Self> {
        let frame = Self {
            prev_frame,
            nbr_locals,
            nbr_args: params.len() as u8,
            params_marker: PhantomData,
            locals_marker: PhantomData,
        };

        unsafe {
            FRAME_ARGS_PTR = Some(&params);
        }
        let mut frame_ptr = Frame::alloc(frame, gc_interface);
        unsafe {
            FRAME_ARGS_PTR = None;
        }

        for i in (0..params.len()).rev() {
            frame_ptr.assign_arg(i as u8, params.pop().unwrap())
        }

        frame_ptr
    }

    /// Get the frame's kind.
    // pub fn kind(&self) -> &FrameKind {
    //     &self.kind
    // }

    /// Get the signature of the current method.
    // #[cfg(feature = "frame-debug-info")]
    // pub fn get_method_signature(&self) -> Interned {
    //     match &self.kind {
    //         FrameKind::Method { signature, .. } => *signature,
    //         FrameKind::Block { block, .. } => block.frame.borrow().get_method_signature(),
    //     }
    // }

    pub fn nth_frame_back(current_frame: &GCRef<Frame>, n: u8) -> GCRef<Frame> {
        if n == 0 {
            return *current_frame;
        }

        let mut target_frame: GCRef<Frame> = match current_frame.lookup_argument(0).as_block() {
            Some(block) => {
                block.frame
            }
            v => panic!("attempting to access a non local var/arg from a method instead of a block: self wasn't blockself but {:?}.", v)
        };
        for _ in 1..n {
            target_frame = match target_frame.lookup_argument(0).as_block() {
                Some(block) => {
                    block.frame
                }
                v => panic!("attempting to access a non local var/arg from a method instead of a block (but the original frame we were in was a block): self wasn't blockself but {:?}.", v)
            };
        }
        target_frame
    }

    pub fn nth_frame_back_through_frame_list(current_frame: &GCRef<Frame>, n: u8) -> GCRef<Frame> {
        debug_assert_ne!(n, 0);
        let mut target_frame = *current_frame;
        for _ in 1..n {
            target_frame = target_frame.prev_frame;
            if target_frame.is_empty() {
                panic!("empty target frame");
            }
        }
        target_frame
    }

    /// Get the method invocation frame for that frame.
    pub fn method_frame(frame: &GCRef<Frame>) -> GCRef<Frame> {
        if let Some(blk) = frame.lookup_argument(0).as_block() {
            Frame::method_frame(&blk.frame)
        } else {
            *frame
        }
    }
}

// exact same as BC... but I'm not positive this isn't useful duplication in the long run? since we may want them to have different implems still
pub trait FrameAccess {
    const ARG_OFFSET: usize = size_of::<Frame>();
    fn get_self(&self) -> Value;
    fn lookup_argument(&self, idx: u8) -> Value;
    fn assign_arg(&mut self, idx: u8, value: Value);
    fn lookup_local(&self, idx: u8) -> Value;
    fn assign_local(&mut self, idx: u8, value: Value);
    fn lookup_field(&self, idx: u8) -> Value;
    fn assign_field(&self, idx: u8, value: &Value);
}

impl FrameAccess for GCRef<Frame> {
    /// Get the self value for this frame.
    fn get_self(&self) -> Value {
        let maybe_self_arg = self.lookup_argument(0);
        match maybe_self_arg.as_block() {
            Some(blk) => blk.frame.get_self(),
            None => maybe_self_arg.clone(), // it is self, we've reached the root
        }
    }

    fn lookup_argument(&self, idx: u8) -> Value {
        unsafe {
            let arg_ptr: &Value = self
                .ptr
                .add(Self::ARG_OFFSET)
                .add(idx as usize * size_of::<Value>())
                .as_ref();
            arg_ptr.clone()
        }
    }

    fn assign_arg(&mut self, idx: u8, value: Value) {
        // TODO: shouldn't assignments take refs?
        unsafe {
            *self
                .ptr
                .add(Self::ARG_OFFSET)
                .add(idx as usize * size_of::<Value>())
                .as_mut_ref() = value
        }
    }

    #[inline] // not sure if necessary
    fn lookup_local(&self, idx: u8) -> Value {
        let nbr_args = self.nbr_args;
        unsafe {
            let value_ptr: &Value = self
                .ptr
                .add(Self::ARG_OFFSET)
                .add((nbr_args + idx) as usize * size_of::<Value>())
                .as_ref();
            value_ptr.clone()
        }
    }

    fn assign_local(&mut self, idx: u8, value: Value) {
        let nbr_args = self.nbr_args;
        unsafe {
            *self
                .ptr
                .add(Self::ARG_OFFSET)
                .add((nbr_args + idx) as usize * size_of::<Value>())
                .as_mut_ref() = value
        }
    }

    fn lookup_field(&self, idx: u8) -> Value {
        let self_ = self.get_self();
        if let Some(instance) = self_.as_instance() {
            instance.lookup_local(idx)
        } else if let Some(cls) = self_.as_class() {
            cls.class().lookup_field(idx)
        } else {
            panic!("{:?}", &self_)
        }
    }

    fn assign_field(&self, idx: u8, value: &Value) {
        let self_ = self.get_self();
        if let Some(mut instance) = self_.as_instance() {
            instance.assign_local(idx, value.clone())
        } else if let Some(cls) = self_.as_class() {
            cls.class().assign_field(idx, value.clone())
        } else {
            panic!("{:?}", &self_)
        }
    }
}

// this is a duplicate of the BC logic. they need unifying somehow, though it's easier said than done
impl CustomAlloc<Frame> for Frame {
    fn alloc(frame: Frame, gc_interface: &mut GCInterface) -> GCRef<Frame> {
        let nbr_locals = frame.nbr_locals;
        let nbr_args = frame.nbr_args;
        let size = size_of::<Frame>() + ((nbr_args + nbr_locals) as usize * size_of::<Value>());

        let frame_ptr = GCRef::<Frame>::alloc_with_size(frame, gc_interface, size);

        unsafe {
            let mut locals_addr = frame_ptr
                .ptr
                .add(size_of::<Frame>())
                .add(nbr_args as usize * size_of::<Value>());
            for _ in 0..nbr_locals {
                *locals_addr.as_mut_ref() = Value::NIL;
                locals_addr = locals_addr.add(size_of::<Value>());
            }
        };

        // println!("frame allocation ok");

        frame_ptr
    }
}
