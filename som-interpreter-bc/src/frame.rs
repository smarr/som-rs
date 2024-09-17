use std::cell::RefCell;
use std::marker::PhantomData;
use core::mem::size_of;
use som_core::bytecode::Bytecode;
use crate::block::Block;
use crate::class::Class;
use crate::compiler::Literal;
use som_core::gc::{CustomAlloc, GCInterface, GCRef};
use crate::method::{Method, MethodKind};
use crate::value::Value;

#[cfg(feature = "frame-debug-info")]
/// The kind of a given frame.
#[derive(Clone)]
pub enum FrameKind {
    /// A frame created from a block evaluation.
    Block {
        /// The block instance for the current frame.
        block: GCRef<Block>,
    },
    /// A frame created from a method invocation.
    Method {
        /// The holder of the current method (used for lexical self/super).
        holder: GCRef<Class>,
        /// The current method.
        method: GCRef<Method>,
        /// The self value.
        self_value: Value,
    },
}

/// Represents a stack frame.
pub struct Frame {
    #[cfg(feature = "frame-debug-info")]
    /// This frame's kind.
    pub kind: FrameKind,
    pub prev_frame: GCRef<Frame>,
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

    /// markers. we don't use them... it's mostly a reminder that the struct looks different in memory... not the cleanest but not sure how else to go about it
    pub args_marker: PhantomData<Vec<Value>>,
    pub locals_marker: PhantomData<Vec<Value>>,
    
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

    pub fn alloc_from_block(block: GCRef<Block>, mut args: Vec<Value>, prev_frame: GCRef<Frame>, mutator: &mut GCInterface) -> GCRef<Frame> {
        let mut frame_ptr = Frame::alloc(Frame::from_block(block, args.len(), prev_frame), mutator);

        for i in (0..args.len()).rev() {
            frame_ptr.assign_arg(i, args.pop().unwrap())
        }

        frame_ptr
    }
    
    fn from_block(block: GCRef<Block>, nbr_args: usize, prev_frame: GCRef<Frame>) -> Self {
        let block_obj = block.to_obj();
        Self {
            #[cfg(feature = "frame-debug-info")]
            kind: FrameKind::Block { block },
            prev_frame,
            nbr_locals: block_obj.blk_info.to_obj().nb_locals,
            nbr_args,
            literals: &block_obj.blk_info.to_obj().literals,
            bytecodes: &block_obj.blk_info.to_obj().body,
            bytecode_idx: 0,
            inline_cache: std::ptr::addr_of!(block_obj.blk_info.to_obj().inline_cache),
            args_marker: PhantomData,
            locals_marker: PhantomData
        }
    }

    fn from_method(method: GCRef<Method>, nbr_args: usize, prev_frame: GCRef<Frame>) -> Self {
        match method.to_obj().kind() {
            MethodKind::Defined(env) => {
                Self {
                    #[cfg(feature = "frame-debug-info")]
                    kind: {
                        let holder = method.holder.upgrade().unwrap();
                        FrameKind::Method {
                            self_value: args.get(0).unwrap().clone(),
                            method: Rc::clone(&method),
                            holder,
                        }
                    },
                    prev_frame,
                    nbr_locals: env.nbr_locals,
                    nbr_args,
                    literals: &env.literals,
                    bytecodes: &env.body,
                    bytecode_idx: 0,
                    inline_cache: std::ptr::addr_of!(env.inline_cache),
                    args_marker: PhantomData,
                    locals_marker: PhantomData
                }
            }
            _ => unreachable!()
        }
    }

    #[cfg(feature = "frame-debug-info")]
    /// Construct a new empty frame from its kind.
    pub fn from_kind(kind: FrameKind) -> Self {
        match &kind {
            FrameKind::Block { block } => {
                // let locals = block.blk_info.locals.iter().map(|_| Value::Nil).collect();
                let locals =  (0..block.blk_info.nb_locals).map(|_| Value::Nil).collect();
                let frame = Self {
                    locals,
                    args: vec![Value::Block(Rc::clone(&block))],
                    literals: &block.blk_info.literals,
                    bytecodes: &block.blk_info.body,
                    bytecode_idx: 0,
                    inline_cache: std::ptr::addr_of!(block.blk_info.inline_cache),
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
                        args: vec![],
                        literals: &env.literals,
                        bytecodes: &env.body,
                        bytecode_idx: 0,
                        inline_cache: std::ptr::addr_of!(env.inline_cache),
                        kind
                    }
                } else {
                    Self {
                        locals: vec![],
                        args: vec![],
                        literals: std::ptr::null(), // todo this is totally safe haha i think
                        bytecodes: std::ptr::null(), // yeah ditto
                        inline_cache: std::ptr::null(), // inline cache is never accessed in prims so this will never fail.. right?
                        bytecode_idx: 0,
                        kind
                    }
                }
            }
        }
    }

    #[cfg(feature = "frame-debug-info")]
    /// Get the frame's kind.
    pub fn kind(&self) -> &FrameKind {
        &self.kind
    }

    #[cfg(feature = "frame-debug-info")]
    /// Get the current method itself.
    pub fn get_method(&self) -> GCRef<Method> {
        match &self.kind {
            FrameKind::Method { method, .. } => *method,
            FrameKind::Block { block, .. } => block.to_obj().frame.as_ref().unwrap().to_obj().get_method(),
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

        let mut target_frame: GCRef<Frame> = match current_frame.lookup_argument(0) {
            Value::Block(block) => {
                *block.to_obj().frame.as_ref().unwrap()
            }
            v => panic!("attempting to access a non local var/arg from a method instead of a block: self wasn't blockself but {:?}.", v)
        };
        for _ in 1..n {
            target_frame = match &target_frame.lookup_argument(0) {
                Value::Block(block) => {
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
    const ARG_OFFSET: usize = size_of::<Frame>();
    fn get_self(&self) -> Value;
    fn lookup_argument(&self, idx: usize) -> &Value;
    fn assign_arg(&mut self, idx: usize, value: Value);
    fn lookup_local(&self, idx: usize) -> &Value;
    fn assign_local(&mut self, idx: usize, value: Value);
}

impl FrameAccess for GCRef<Frame> {
    /// Get the self value for this frame.
    fn get_self(&self) -> Value {
        match self.lookup_argument(0) {
            Value::Block(b) => {
                let block_frame = b.to_obj().frame.unwrap();
                block_frame.get_self()
            },
            s => s.clone()
        }
    }
    
    #[inline(always)]
    fn lookup_argument(&self, idx: usize) -> &Value {
        unsafe { self.ptr.add(Self::ARG_OFFSET).add(idx * size_of::<Value>()).as_ref() }
    }

    /// Assign to an argument.
    #[inline(always)]
    fn assign_arg(&mut self, idx: usize, value: Value) {
        unsafe { *self.ptr.add(Self::ARG_OFFSET).add(idx * size_of::<Value>()).as_mut_ref() = value }
    }
    
    /// Search for a local binding.
    #[inline(always)]
    fn lookup_local(&self, idx: usize) -> &Value {
        let nbr_args = self.to_obj().nbr_args;
        unsafe { self.ptr.add(Self::ARG_OFFSET).add((nbr_args + idx) * size_of::<Value>()).as_ref() }
    }

    /// Assign to a local binding.
    #[inline(always)]
    fn assign_local(&mut self, idx: usize, value: Value) {
        let nbr_args = self.to_obj().nbr_args;
        unsafe { *self.ptr.add(Self::ARG_OFFSET).add((nbr_args + idx) * size_of::<Value>()).as_mut_ref() = value }
    }
}

impl CustomAlloc<Frame> for Frame {
    fn alloc(frame: Frame, gc_interface: &mut GCInterface) -> GCRef<Frame> {
        let nbr_locals = frame.nbr_locals;
        let nbr_args = frame.nbr_args;
        let size = size_of::<Frame>() + ((nbr_args + nbr_locals) * size_of::<Value>());

        let frame_ptr = GCRef::<Frame>::alloc_with_size(frame, gc_interface, size);
        
        unsafe {
            let mut locals_addr = frame_ptr.ptr.add(size_of::<Frame>()).add(nbr_args * size_of::<Value>());
            for _ in 0..nbr_locals {
                *locals_addr.as_mut_ref() = Value::Nil;
                locals_addr = locals_addr.add(size_of::<Value>());
            }
        };
        
        frame_ptr
    }
}