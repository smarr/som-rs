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

const OFFSET_TO_STACK: usize = size_of::<Frame>();

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

    pub stack_ptr: *mut Value,
    pub args_ptr: *mut Value,
    pub locals_ptr: *mut Value,
    
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
    pub fn alloc_from_method(method: GCRef<Method>, 
                             args: &[Value], 
                             prev_frame: GCRef<Frame>, 
                             mutator: &mut GCInterface) -> GCRef<Frame> {
        let frame_ptr = Frame::alloc(Frame::from_method(method, args.len(), prev_frame), mutator);
        Frame::init_frame_post_alloc(frame_ptr, args);
        frame_ptr
    }

    pub fn alloc_from_block(block: GCRef<Block>,
                            args: &[Value],
                            current_method: *const Method,
                            prev_frame: GCRef<Frame>,
                            mutator: &mut GCInterface) -> GCRef<Frame> {
        let frame_ptr = Frame::alloc(Frame::from_block(block, args.len(), current_method, prev_frame), mutator);
        Frame::init_frame_post_alloc(frame_ptr, args);
        frame_ptr
    }
    
    fn init_frame_post_alloc(frame_ptr: GCRef<Frame>, args: &[Value]) {
        unsafe {
            let frame = frame_ptr.to_obj();
            
            // setting up the self-referential pointers for args/locals accesses 
            frame.stack_ptr = frame_ptr.ptr.add(OFFSET_TO_STACK).as_mut_ref();
            frame.args_ptr = frame.stack_ptr.add(MAX_STACK_SIZE);
            frame.locals_ptr = frame.args_ptr.add(frame.nbr_args);

            // initializing arguments from the args slice
            std::slice::from_raw_parts_mut(frame.args_ptr, args.len()).copy_from_slice(args);

            // setting all locals to NIL.
            for idx in 0..frame.nbr_locals {
                *frame.locals_ptr.add(idx) = Value::NIL;
            }
        }
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
            inline_cache: std::ptr::addr_of!(block_obj.blk_info.to_obj().inline_cache),
            stack_ptr: std::ptr::null_mut(),
            args_ptr: std::ptr::null_mut(),
            locals_ptr: std::ptr::null_mut(),
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
                    stack_ptr: std::ptr::null_mut(),
                    args_ptr: std::ptr::null_mut(),
                    locals_ptr: std::ptr::null_mut(),
                    inline_cache: std::ptr::addr_of!(env.inline_cache),
                    args_marker: PhantomData,
                    locals_marker: PhantomData,
                    stack_marker: PhantomData,
                }
            }
            _ => unreachable!()
        }
    }

    /// Get the self value for this frame.
    pub(crate) fn get_self(&self) -> Value {
        let self_arg = self.lookup_argument(0);
        match self_arg.as_block() {
            Some(b) => {
                let block_frame = b.to_obj().frame.unwrap();
                block_frame.to_obj().get_self()
            }
            None => self_arg.clone()
        }
    }

    /// Get the holder for this current method.
    pub(crate) fn get_method_holder(&self) -> GCRef<Class> {
        match self.lookup_argument(0).as_block() {
            Some(b) => {
                let block_frame = b.to_obj().frame.as_ref().unwrap();
                let x = block_frame.to_obj().get_method_holder();
                x
            },
            None => {
                unsafe { (*self.current_method).holder }
            }
        }
    }

    /// Search for a local binding.
    #[inline(always)]
    pub fn lookup_local(&self, idx: usize) -> &Value {
        unsafe { self.locals_ptr.add(idx).as_ref().unwrap() }

    }

    /// Assign to a local binding.
    #[inline(always)]
    pub fn assign_local(&mut self, idx: usize, value: Value) {
        unsafe { *self.locals_ptr.add(idx) = value }
    }

    #[inline(always)]
    pub fn lookup_argument(&self, idx: usize) -> &Value {
        unsafe { self.args_ptr.add(idx).as_ref().unwrap() }
    }

    /// Assign to an argument.
    #[inline(always)]
    pub fn assign_arg(&mut self, idx: usize, value: Value) {
        unsafe { *self.args_ptr.add(idx) = value }
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

        let mut target_frame: GCRef<Frame> = match current_frame.to_obj().lookup_argument(0).as_block() {
            Some(block) => {
                *block.to_obj().frame.as_ref().unwrap()
            }
            v => panic!("attempting to access a non local var/arg from a method instead of a block: self wasn't blockself but {:?}.", v)
        };
        for _ in 1..n {
            target_frame = match &target_frame.to_obj().lookup_argument(0).as_block() {
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
    fn get_stack(&self) -> &mut [Value];
    fn stack_push(&mut self, value: Value);
    fn stack_pop(&mut self) -> Value;
    fn stack_last(&self) -> &Value;
    fn stack_last_mut(&self) -> &mut Value;
    fn stack_nth_back(&self, n: usize) -> &Value;
    fn stack_len(&self) -> usize;
    fn stack_n_last_elements(&self, at_idx: usize) -> &[Value];
}

impl FrameAccess for GCRef<Frame> {
    #[inline(always)]
    fn get_stack(&self) -> &mut [Value] {
        unsafe {
            let ptr: *mut Value = self.ptr.add(OFFSET_TO_STACK).to_mut_ptr();
            std::slice::from_raw_parts_mut(ptr, MAX_STACK_SIZE)
        }
    }

    #[inline(always)]
    fn stack_push(&mut self, value: Value) {
        unsafe {
            let frame = self.to_obj();
            *frame.stack_ptr = value;
            frame.stack_ptr = frame.stack_ptr.add(1);
        }
    }

    #[inline(always)]
    fn stack_pop(&mut self) -> Value {
        unsafe {
            let frame = self.to_obj();
            frame.stack_ptr = frame.stack_ptr.sub(1);
            *frame.stack_ptr
        }
    }

    #[inline(always)]
    fn stack_last(&self) -> &Value {
        unsafe { &*self.to_obj().stack_ptr.sub(1) }
    }

    #[inline(always)]
    fn stack_last_mut(&self) -> &mut Value {
        unsafe { &mut *self.to_obj().stack_ptr.sub(1) }
    }

    fn stack_nth_back(&self, n: usize) -> &Value {
        unsafe { &(*self.to_obj().stack_ptr.sub( n + 1)) }
    }

    /// only used for testing
    fn stack_len(&self) -> usize {
        ((self.to_obj().stack_ptr as usize) - (self.ptr.as_usize() + OFFSET_TO_STACK)) / size_of::<Value>()
    }

    fn stack_n_last_elements(&self, n: usize) -> &[Value] {
        unsafe {
            let slice_ptr = self.to_obj().stack_ptr.sub(n);
            self.to_obj().stack_ptr = slice_ptr;
            std::slice::from_raw_parts_mut(slice_ptr, n)
        }
    }
}

impl CustomAlloc<Frame> for Frame {
    fn alloc(frame: Frame, gc_interface: &mut GCInterface) -> GCRef<Frame> {
        let nbr_locals = frame.nbr_locals;
        let nbr_args = frame.nbr_args;
        let max_stack_size = MAX_STACK_SIZE;
        let size = size_of::<Frame>() + ((max_stack_size + nbr_args + nbr_locals) * size_of::<Value>());

        GCRef::<Frame>::alloc_with_size(frame, gc_interface, size)
    }
}