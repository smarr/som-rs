use crate::value::Value;
use crate::FRAME_ARGS_PTR;
use core::mem::size_of;
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::Gc;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ptr::NonNull;

macro_rules! frame_args_ptr {
    ($base_ptr:expr) => {
        ($base_ptr.ptr + std::mem::size_of::<Frame>()) as *mut Value
    };
}

macro_rules! frame_locals_ptr {
    ($base_ptr:expr, $nbr_args:expr) => {
        frame_args_ptr!($base_ptr).add($nbr_args)
    };
}

/// Represents a stack frame.
pub struct Frame {
    pub prev_frame: Gc<Frame>,
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
    pub fn alloc_new_frame(nbr_locals: u8, mut params: Vec<Value>, prev_frame: &Gc<Frame>, gc_interface: &mut GCInterface) -> Gc<Self> {
        let frame = Self {
            prev_frame: Gc::default(),
            nbr_locals,
            nbr_args: params.len() as u8,
            params_marker: PhantomData,
            locals_marker: PhantomData,
        };

        unsafe {
            FRAME_ARGS_PTR = NonNull::new(&mut params);
        }

        let post_alloc_closure = |mut frame_ptr: Gc<Frame>| {
            unsafe {
                let mut locals_addr = (frame_ptr.ptr + size_of::<Frame>() + (params.len() * size_of::<Value>())) as *mut Value;
                for _ in 0..nbr_locals {
                    *locals_addr = Value::NIL;
                    locals_addr = locals_addr.wrapping_add(1);
                }

                std::slice::from_raw_parts_mut(frame_args_ptr!(frame_ptr), params.len()).copy_from_slice(params.as_slice());

                frame_ptr.prev_frame = *prev_frame;
            };
        };
        let size = size_of::<Frame>() + ((frame.nbr_args + frame.nbr_locals) as usize * size_of::<Value>());
        let frame_ptr = gc_interface.alloc_with_post_init(frame, size, post_alloc_closure);

        unsafe {
            FRAME_ARGS_PTR = None;
        }

        frame_ptr
    }

    pub fn nth_frame_back(current_frame: &Gc<Frame>, n: u8) -> Gc<Frame> {
        if n == 0 {
            return *current_frame;
        }

        let mut target_frame: Gc<Frame> = match current_frame.lookup_argument(0).as_block() {
            Some(block) => block.frame,
            v => panic!(
                "attempting to access a non local var/arg from a method instead of a block: self wasn't blockself but {:?}.",
                v
            ),
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

    pub fn nth_frame_back_through_frame_list(current_frame: &Gc<Frame>, n: u8) -> Gc<Frame> {
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
    pub fn method_frame(frame: &Gc<Frame>) -> Gc<Frame> {
        if let Some(blk) = frame.lookup_argument(0).as_block() {
            Frame::method_frame(&blk.frame)
        } else {
            *frame
        }
    }

    /// Returns the true size of a Frame, counting the heap stored right after it.
    pub fn get_true_size(nbr_args: u8, nbr_locals: u8) -> usize {
        size_of::<Frame>() + ((nbr_args + nbr_locals) as usize * size_of::<Value>())
    }
}

// exact same as BC... but I'm not positive this isn't useful duplication in the long run? since we may want them to have different implems still
pub trait FrameAccess {
    fn get_self(&self) -> Value;
    fn lookup_argument(&self, idx: u8) -> &Value;
    fn assign_arg(&mut self, idx: u8, value: Value);
    fn lookup_local(&self, idx: u8) -> &Value;
    fn assign_local(&mut self, idx: u8, value: Value);
    fn lookup_field(&self, idx: u8) -> Value;
    fn assign_field(&self, idx: u8, value: &Value);
}

impl FrameAccess for Gc<Frame> {
    /// Get the self value for this frame.
    fn get_self(&self) -> Value {
        let maybe_self_arg = *self.lookup_argument(0);
        match maybe_self_arg.as_block() {
            Some(blk) => blk.frame.get_self(),
            None => maybe_self_arg, // it is self, we've reached the root
        }
    }

    fn lookup_argument(&self, idx: u8) -> &Value {
        unsafe {
            let arg_ptr = frame_args_ptr!(self).add(idx as usize);
            &*arg_ptr
        }
    }

    fn assign_arg(&mut self, idx: u8, value: Value) {
        // TODO: shouldn't assignments take refs?
        unsafe {
            let arg_ptr = frame_args_ptr!(self).add(idx as usize);
            *arg_ptr = value
        }
    }

    #[inline] // not sure if necessary
    fn lookup_local(&self, idx: u8) -> &Value {
        unsafe {
            let value_ptr = frame_locals_ptr!(self, self.nbr_args as usize).add(idx as usize);
            &*value_ptr
        }
    }

    fn assign_local(&mut self, idx: u8, value: Value) {
        unsafe {
            let value_ptr = frame_locals_ptr!(self, self.nbr_args as usize).add(idx as usize);
            *value_ptr = value
        }
    }

    fn lookup_field(&self, idx: u8) -> Value {
        let self_ = self.get_self();
        if let Some(instance) = self_.as_instance() {
            *instance.lookup_field(idx)
        } else if let Some(cls) = self_.as_class() {
            cls.class().lookup_field(idx)
        } else {
            panic!("{:?}", &self_)
        }
    }

    fn assign_field(&self, idx: u8, value: &Value) {
        let self_ = self.get_self();
        if let Some(mut instance) = self_.as_instance() {
            instance.assign_field(idx, *value)
        } else if let Some(cls) = self_.as_class() {
            cls.class().assign_field(idx, *value)
        } else {
            panic!("{:?}", &self_)
        }
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame").field("nbr_args", &self.nbr_args).field("nbr_locals", &self.nbr_locals).finish()
    }
}
