use crate::block::Block;
use crate::class::Class;
use crate::compiler::Literal;
use crate::method::{Method, MethodKind};
use crate::value::Value;
use core::mem::size_of;
use som_core::bytecode::Bytecode;
use som_core::gc::{CustomAlloc, GCInterface, GCRef};
use std::cell::RefCell;
use std::marker::PhantomData;

const ARG_OFFSET: usize = size_of::<Frame>();

const MAX_STACK_SIZE: usize = 10;

/// Represents a stack frame.
pub struct Frame {
    /// The previous frame. Frames are handled as a linked list
    pub prev_frame: GCRef<Frame>,
    /// The method the execution context currently is in.
    pub current_method: *const Method,
    /// The bytecodes associated with the frame.
    pub bytecodes: *const Vec<Bytecode>,
    /// Literals/constants associated with the frame.
    pub literals: *const Vec<Literal>,
    /// Inline cache associated with the frame. TODO - refcell not worth it with the GC now, is it?
    pub inline_cache: *const RefCell<Vec<Option<(*const Class, GCRef<Method>)>>>, // todo class can also be a GC ref, that's basically a pointer
    /// Bytecode index.
    pub bytecode_idx: usize,

    pub nbr_args: usize, // todo u8 instead?
    pub nbr_locals: usize,

    pub stack_len: usize,

    /// markers. we don't use them directly. it's mostly a reminder that the struct looks different in memory... not the cleanest but not sure how else to go about it
    // pub stack_marker: PhantomData<[Value]>,
    pub args_marker: PhantomData<[Value]>,
    pub locals_marker: PhantomData<[Value]>,
    pub stack_marker: PhantomData<[Value]>,

    // /// The arguments within this frame.
    // pub args: Vec<Value>,
    // /// The bindings within this frame.
    // pub locals: Vec<Value>,
}

impl Frame {
    pub fn alloc_from_method(method: GCRef<Method>, mut args: Vec<Value>, prev_frame: GCRef<Frame>, mutator: &mut GCInterface) -> GCRef<Frame> {
        let mut frame_ptr = Frame::alloc(Frame::from_method(method, args.len(), prev_frame), mutator);

        // might be faster if we did that in the alloc method, but that means passing args as an argument to the trait method `alloc` somehow.
        for i in (0..args.len()).rev() {
            frame_ptr.assign_arg(i, args.pop().unwrap())
        }

        frame_ptr
    }

    pub fn alloc_from_block(block: GCRef<Block>,
                            mut args: Vec<Value>,
                            current_method: *const Method,
                            prev_frame: GCRef<Frame>,
                            mutator: &mut GCInterface) -> GCRef<Frame> {
        let mut frame_ptr = Frame::alloc(Frame::from_block(block, args.len(), current_method, prev_frame), mutator);

        for i in (0..args.len()).rev() {
            frame_ptr.assign_arg(i, args.pop().unwrap())
        }

        frame_ptr
    }

    // Creates a frame from a block. Meant to only be called by the alloc_from_block function
    fn from_block(block: GCRef<Block>, nbr_args: usize, current_method: *const Method, prev_frame: GCRef<Frame>) -> Self {
        let block_obj = block.to_obj();
        Self {
            prev_frame,
            current_method,
            nbr_locals: block_obj.blk_info.to_obj().nb_locals,
            nbr_args,
            literals: &block_obj.blk_info.to_obj().literals,
            bytecodes: &block_obj.blk_info.to_obj().body,
            bytecode_idx: 0,
            stack_len: 0,
            inline_cache: std::ptr::addr_of!(block_obj.blk_info.to_obj().inline_cache),
            args_marker: PhantomData,
            locals_marker: PhantomData,
            stack_marker: PhantomData,
        }
    }

    // Creates a frame from a block. Meant to only be called by the alloc_from_method function
    fn from_method(method: GCRef<Method>, nbr_args: usize, prev_frame: GCRef<Frame>) -> Self {
        match method.to_obj().kind() {
            MethodKind::Defined(env) => {
                Self {
                    prev_frame,
                    nbr_locals: env.nbr_locals,
                    nbr_args,
                    literals: &env.literals,
                    bytecodes: &env.body,
                    current_method: method.as_ref(),
                    bytecode_idx: 0,
                    stack_len: 0,
                    inline_cache: std::ptr::addr_of!(env.inline_cache),
                    args_marker: PhantomData,
                    locals_marker: PhantomData,
                    stack_marker: PhantomData,
                }
            }
            _ => unreachable!()
        }
    }

    // Don't even need this function. We store a pointer to the bytecode in the interpreter directly.
    // pub fn get_bytecode(&self, idx: usize) -> Option<Bytecode> {
    //     self.bytecodes.get(idx).cloned()
    // }

    #[inline(always)]
    pub fn lookup_constant(&self, idx: usize) -> Literal {
        match cfg!(debug_assertions) {
            true => unsafe { (*self.literals).get(idx).unwrap().clone() },
            false => unsafe { (*self.literals).get_unchecked(idx).clone() }
        }
    }

    pub fn nth_frame_back(current_frame: &GCRef<Frame>, n: u8) -> GCRef<Frame> {
        if n == 0 {
            return *current_frame;
        }

        let mut target_frame: GCRef<Frame> = match current_frame.lookup_argument(0).as_block() {
            Some(block) => {
                *block.to_obj().frame.as_ref().unwrap()
            }
            v => panic!("attempting to access a non local var/arg from a method instead of a block: self wasn't blockself but {:?}.", v)
        };
        for _ in 1..n {
            target_frame = match &target_frame.lookup_argument(0).as_block() {
                Some(block) => {
                    *block.to_obj().frame.as_ref().unwrap()
                }
                v => panic!("attempting to access a non local var/arg from a method instead of a block (but the original frame we were in was a block): self wasn't blockself but {:?}.", v)
            };
        }
        target_frame
    }

    /// nth_frame_back but through prev_frame ptr. TODO: clarify why different implems are needed
    pub fn nth_frame_back_through_frame_list(current_frame: &GCRef<Frame>, n: u8) -> GCRef<Frame> {
        debug_assert_ne!(n, 0);
        let mut target_frame = *current_frame;
        for _ in 1..n {
            target_frame = target_frame.to_obj().prev_frame;
            if target_frame.is_empty() {
                panic!("empty target frame");
            }
        }
        target_frame
    }
}

/// Operations to access inside a frame.
/// Currently defined on `GCRef<Frame>`, but those methods all used to be defined directly on `Frame`.
/// TODO: `Frame` should also implement it for debug purposes, since operating on raw memory through the GC pointer doesn't rely on the Rust type system and makes debugging harder
pub trait FrameAccess {
    fn get_self(&self) -> Value;
    fn get_method_holder(&self) -> GCRef<Class>;
    fn lookup_argument(&self, idx: usize) -> &Value;
    fn assign_arg(&mut self, idx: usize, value: Value);
    fn lookup_local(&self, idx: usize) -> &Value;
    fn assign_local(&mut self, idx: usize, value: Value);
    fn get_value_arr(&self, max_size: usize) -> &[Value];
    fn get_value_arr_mut(&self, max_size: usize) -> &mut [Value];

    /// Stack operations
    fn stack_push(&mut self, value: Value);
    fn stack_pop(&mut self) -> Value;
    fn stack_last(&self) -> &Value;
    fn stack_last_mut(&self) -> &mut Value;
    fn stack_nth_back(&self, n: usize) -> &Value;
    fn stack_len(&self) -> usize;
    fn get_stack(&self) -> &mut [Value];
    fn stack_n_last_elements(&self, at_idx: usize) -> Vec<Value>;
}

impl FrameAccess for GCRef<Frame> {
    /// Get the self value for this frame.
    fn get_self(&self) -> Value {
        let self_arg = self.lookup_argument(0);
        match self_arg.as_block() {
            Some(b) => {
                let block_frame = b.to_obj().frame.unwrap();
                block_frame.get_self()
            }
            None => self_arg.clone()
        }
    }

    /// Get the holder for this current method.
    fn get_method_holder(&self) -> GCRef<Class> {
        match self.lookup_argument(0).as_block() {
            Some(b) => {
                let block_frame = b.to_obj().frame.as_ref().unwrap();
                let x = block_frame.get_method_holder();
                x
            },
            None => {
                unsafe { (*self.to_obj().current_method).holder }
            }
        }
    }

    #[inline(always)]
    fn get_value_arr(&self, max_size: usize) -> &[Value] {
        unsafe {
            let ptr: *const Value = self.ptr.add(ARG_OFFSET).to_ptr();
            std::slice::from_raw_parts(ptr, max_size)
        }
    }

    #[inline(always)]
    fn get_value_arr_mut(&self, max_size: usize) -> &mut [Value] {
        unsafe {
            let ptr: *mut Value = self.ptr.add(ARG_OFFSET).to_mut_ptr();
            std::slice::from_raw_parts_mut(ptr, max_size)
        }
    }

    #[inline(always)]
    fn get_stack(&self) -> &mut [Value] {
        unsafe {
            let (nbr_args, nbr_locals) = {
                let f = self.to_obj();
                (f.nbr_args, f.nbr_locals)
            };

            let ptr: *mut Value = self.ptr.add(ARG_OFFSET).add((nbr_args + nbr_locals) * size_of::<Value>()).to_mut_ptr();
            std::slice::from_raw_parts_mut(ptr, MAX_STACK_SIZE)
        }
    }

    #[inline(always)]
    fn lookup_argument(&self, idx: usize) -> &Value {
        let arr = self.get_value_arr(idx + 1); // we just say idx + 1 since that's enough. we don't need to actually know the total number of args.
        &arr[idx]
    }

    /// Assign to an argument.
    #[inline(always)]
    fn assign_arg(&mut self, idx: usize, value: Value) {
        let arr: &mut [Value] = self.get_value_arr_mut(idx + 1);
        arr[idx] = value
    }

    /// Search for a local binding.
    #[inline(always)]
    fn lookup_local(&self, idx: usize) -> &Value {
        let local_idx = self.to_obj().nbr_args + idx;
        let arr: &[Value] = self.get_value_arr(local_idx + 1);
        &arr[local_idx]
    }

    /// Assign to a local binding.
    #[inline(always)]
    fn assign_local(&mut self, idx: usize, value: Value) {
        let local_idx = self.to_obj().nbr_args + idx;
        let arr: &mut [Value] = self.get_value_arr_mut(local_idx + 1);
        arr[local_idx] = value
    }

    #[inline(always)]
    fn stack_push(&mut self, value: Value) {
        let stack = self.get_stack();
        let stack_ptr = &mut self.to_obj().stack_len;
        
        stack[*stack_ptr] = value;
        *stack_ptr += 1;
    }

    #[inline(always)]
    fn stack_pop(&mut self) -> Value {
        let stack_ptr = &mut self.to_obj().stack_len;
        *stack_ptr -= 1;
        self.get_stack()[*stack_ptr]
    }

    #[inline(always)]
    fn stack_last(&self) -> &Value {
        &self.get_stack()[self.to_obj().stack_len - 1]
    }

    #[inline(always)]
    fn stack_last_mut(&self) -> &mut Value {
        &mut self.get_stack()[self.to_obj().stack_len - 1]
    }

    #[inline(always)]
    fn stack_len(&self) -> usize {
        self.to_obj().stack_len
    }

    fn stack_nth_back(&self, n: usize) -> &Value {
        &self.get_stack()[self.to_obj().stack_len - n - 1]
    }

    fn stack_n_last_elements(&self, n: usize) -> Vec<Value> {
        unsafe {
            let (nbr_args, nbr_locals) = {
                let f = self.to_obj();
                (f.nbr_args, f.nbr_locals)
            };

            let ptr: *mut Value = self.ptr
                .add(ARG_OFFSET)
                .add((nbr_args + nbr_locals) * size_of::<Value>())
                .add((self.to_obj().stack_len - n) * size_of::<Value>())
                .to_mut_ptr();
            
            self.to_obj().stack_len -= n;
            
            let arr = std::slice::from_raw_parts_mut(ptr, n); // todo: we should return that.
            arr.iter().map(|v| v.clone()).collect()
        }
    }
}

impl CustomAlloc<Frame> for Frame {
    fn alloc(frame: Frame, gc_interface: &mut GCInterface) -> GCRef<Frame> {
        let nbr_locals = frame.nbr_locals;
        let nbr_args = frame.nbr_args;
        let max_stack_size = MAX_STACK_SIZE;
        let size = size_of::<Frame>() + ((nbr_args + nbr_locals + max_stack_size) * size_of::<Value>());

        let frame_ptr = GCRef::<Frame>::alloc_with_size(frame, gc_interface, size);

        unsafe {
            let mut locals_addr = frame_ptr.ptr.add(ARG_OFFSET).add(nbr_args * size_of::<Value>());
            for _ in 0..nbr_locals {
                *locals_addr.as_mut_ref() = Value::NIL;
                locals_addr = locals_addr.add(size_of::<Value>());
            }
        };

        frame_ptr
    }
}