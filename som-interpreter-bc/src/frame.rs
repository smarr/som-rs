use crate::block::{Block, BodyInlineCache};
use crate::class::Class;
use crate::compiler::Literal;
use crate::method::{Method, MethodKind};
use crate::value::Value;
use core::mem::size_of;
use som_core::bytecode::Bytecode;
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::Gc;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

pub(crate) const OFFSET_TO_STACK: usize = size_of::<Frame>();

/// Represents a stack frame.
pub struct Frame {
    /// The previous frame. Frames are handled as a linked list
    pub prev_frame: Gc<Frame>,
    /// The method the execution context currently is in.
    pub current_method: Gc<Method>,
    /// The bytecodes associated with the frame.
    pub bytecodes: *const Vec<Bytecode>,
    /// Literals/constants associated with the frame.
    pub literals: *const Vec<Literal>,
    /// Inline cache associated with the frame.
    pub inline_cache: *mut BodyInlineCache,
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
    pub fn alloc_from_method(method: Gc<Method>, args: &[Value], prev_frame: &Gc<Frame>, gc_interface: &mut GCInterface) -> Gc<Frame> {
        let frame = Frame::from_method(method, args.len());
        let max_stack_size = match &method.kind {
            MethodKind::Defined(m_env) => m_env.max_stack_size as usize,
            _ => unreachable!("if we're allocating a method frame, it has to be defined."),
        };

        let size = frame.get_true_size(max_stack_size);
        let mut frame_ptr = gc_interface.alloc_with_size(frame, size);
        Frame::init_frame_post_alloc(frame_ptr, args, max_stack_size, *prev_frame);
        frame_ptr.current_method = method; // TODO: this is INVALID for semispace! we need to pass it a REFERENCE directly to the method, in case it gets moved during GC..
        frame_ptr
    }

    pub fn alloc_from_block(
        block: Gc<Block>,
        args: &[Value],
        current_method: &Gc<Method>,
        prev_frame: &Gc<Frame>,
        gc_interface: &mut GCInterface,
    ) -> Gc<Frame> {
        let frame = Frame::from_block(block, args.len(), *current_method);
        let max_stack_size = block.blk_info.max_stack_size as usize;
        let size = frame.get_true_size(max_stack_size);

        let mut frame_ptr = gc_interface.alloc_with_size(frame, size);
        Frame::init_frame_post_alloc(frame_ptr, args, max_stack_size, *prev_frame);
        frame_ptr.current_method = *current_method;

        frame_ptr
    }

    fn init_frame_post_alloc(frame_ptr: Gc<Frame>, args: &[Value], stack_size: usize, prev_frame: Gc<Frame>) {
        unsafe {
            let mut frame = frame_ptr;

            // setting up the self-referential pointers for args/locals accesses
            frame.stack_ptr = (frame_ptr.ptr + OFFSET_TO_STACK) as *mut Value;

            // for idx in 0..stack_size {
            //     *frame.stack_ptr.add(idx) = Value::NIL;
            // }

            frame.args_ptr = frame.stack_ptr.add(stack_size);
            frame.locals_ptr = frame.args_ptr.add(frame.nbr_args);

            // initializing arguments from the args slice
            std::slice::from_raw_parts_mut(frame.args_ptr, args.len()).copy_from_slice(args);

            // setting all locals to NIL.
            for idx in 0..frame.nbr_locals {
                *frame.locals_ptr.add(idx) = Value::NIL;
            }

            frame.prev_frame = prev_frame; // because GC can have moved the previous frame!
        }
    }

    // Creates a frame from a block. Meant to only be called by the alloc_from_block function
    fn from_block(block: Gc<Block>, nbr_args: usize, current_method: Gc<Method>) -> Self {
        let mut block_obj = block;
        Self {
            prev_frame: Gc::default(),
            current_method,
            nbr_locals: block_obj.blk_info.nb_locals,
            nbr_args,
            literals: &block_obj.blk_info.literals,
            bytecodes: &block_obj.blk_info.body,
            bytecode_idx: 0,
            inline_cache: std::ptr::addr_of_mut!(block_obj.blk_info.inline_cache),
            stack_ptr: std::ptr::null_mut(),
            args_ptr: std::ptr::null_mut(),
            locals_ptr: std::ptr::null_mut(),
            args_marker: PhantomData,
            locals_marker: PhantomData,
            stack_marker: PhantomData,
        }
    }

    // Creates a frame from a block. Meant to only be called by the alloc_from_method function
    fn from_method(mut method: Gc<Method>, nbr_args: usize) -> Self {
        match &mut method.kind {
            MethodKind::Defined(env) => {
                let inline_cache = std::ptr::addr_of_mut!(env.inline_cache);
                Self {
                    prev_frame: Gc::default(),
                    nbr_locals: env.nbr_locals,
                    nbr_args,
                    literals: &env.literals,
                    bytecodes: &env.body,
                    current_method: method,
                    bytecode_idx: 0,
                    stack_ptr: std::ptr::null_mut(),
                    args_ptr: std::ptr::null_mut(),
                    locals_ptr: std::ptr::null_mut(),
                    inline_cache,
                    args_marker: PhantomData,
                    locals_marker: PhantomData,
                    stack_marker: PhantomData,
                }
            }
            _ => unreachable!(),
        }
    }

    /// Returns the true size of the `Frame`, counting the extra memory needed for its stack/locals/arguments.
    /// Takes in the maximum stack size, but could also fetch it from its methodenv.
    /// But it's currently only invoked in contexts where we need the max stack size for other calculations, so it takes it to not have to re-compute it.
    pub fn get_true_size(&self, max_stack_size: usize) -> usize {
        size_of::<Frame>() + ((max_stack_size + self.nbr_args + self.nbr_locals) * size_of::<Value>())
    }

    /// Get the self value for this frame.
    pub(crate) fn get_self(&self) -> Value {
        let self_arg = self.lookup_argument(0);
        match self_arg.as_block() {
            Some(b) => {
                let block_frame = b.frame.unwrap();
                block_frame.get_self()
            }
            None => *self_arg,
        }
    }

    /// Get the holder for this current method.
    pub(crate) fn get_method_holder(&self) -> Gc<Class> {
        match self.lookup_argument(0).as_block() {
            Some(b) => {
                let block_frame = b.frame.as_ref().unwrap();
                block_frame.get_method_holder()
            }
            None => self.current_method.holder,
        }
    }

    /// Search for a local binding.
    #[inline(always)]
    pub fn lookup_local(&self, idx: usize) -> &Value {
        unsafe { &*self.locals_ptr.add(idx) }
    }

    /// Assign to a local binding.
    #[inline(always)]
    pub fn assign_local(&mut self, idx: usize, value: Value) {
        unsafe { *self.locals_ptr.add(idx) = value }
    }

    #[inline(always)]
    pub fn lookup_argument(&self, idx: usize) -> &Value {
        unsafe { &*self.args_ptr.add(idx) }
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
            false => unsafe { (*self.literals).get_unchecked(idx).clone() },
        }
    }

    pub fn nth_frame_back(current_frame: &Gc<Frame>, n: u8) -> Gc<Frame> {
        if n == 0 {
            return *current_frame;
        }

        let mut target_frame: Gc<Frame> = match current_frame.lookup_argument(0).as_block() {
            Some(block) => *block.frame.as_ref().unwrap(),
            v => panic!(
                "attempting to access a non local var/arg from a method instead of a block: self wasn't blockself but {:?}.",
                v
            ),
        };
        for _ in 1..n {
            target_frame = match &target_frame.lookup_argument(0).as_block() {
                Some(block) => {
                    *block.frame.as_ref().unwrap()
                }
                v => panic!("attempting to access a non local var/arg from a method instead of a block (but the original frame we were in was a block): self wasn't blockself but {:?}.", v)
            };
        }
        target_frame
    }

    /// nth_frame_back but through prev_frame ptr. TODO: clarify why different implems are needed
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

    #[inline(always)]
    pub fn stack_push(&mut self, value: Value) {
        unsafe {
            *self.stack_ptr = value;
            self.stack_ptr = self.stack_ptr.add(1);
        }
    }

    #[inline(always)]
    pub fn stack_pop(&mut self) -> Value {
        unsafe {
            self.stack_ptr = self.stack_ptr.sub(1);
            *self.stack_ptr
        }
    }

    #[inline(always)]
    pub fn stack_last(&self) -> &Value {
        unsafe { &*self.stack_ptr.sub(1) }
    }

    #[inline(always)]
    pub fn stack_last_mut(&mut self) -> &mut Value {
        unsafe { &mut *self.stack_ptr.sub(1) }
    }

    #[inline(always)]
    pub fn stack_nth_back(&self, n: usize) -> &Value {
        unsafe { &(*self.stack_ptr.sub(n + 1)) }
    }

    #[inline(always)]
    pub fn stack_n_last_elements(&mut self, n: usize) -> &[Value] {
        unsafe {
            let slice_ptr = self.stack_ptr.sub(n);
            // self.stack_ptr = slice_ptr;
            std::slice::from_raw_parts_mut(slice_ptr, n)
        }
    }

    #[inline(always)]
    pub fn remove_n_last_elements(&mut self, n: usize) {
        unsafe { self.stack_ptr = self.stack_ptr.sub(n) }
    }

    /// Gets the total number of elements on the stack. Only used for debugging.
    #[cfg(test)]
    pub fn stack_len(frame_ptr: Gc<Frame>) -> usize {
        ((frame_ptr.stack_ptr as usize) - (frame_ptr.ptr + OFFSET_TO_STACK)) / size_of::<Value>()
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO: an iter on Frame instead..
        unsafe fn stack_printer(frame: &Frame) -> String {
            let mut stack_elements = vec![];
            let frame_stack_start_addr = (frame as *const Frame).byte_add(OFFSET_TO_STACK) as *const Value;
            let mut backwards_stack_ptr = frame_stack_start_addr;
            while !std::ptr::eq(backwards_stack_ptr, frame.stack_ptr) {
                let stack_val = &*backwards_stack_ptr;
                stack_elements.push(format!("{:?}", &stack_val));
                backwards_stack_ptr = backwards_stack_ptr.add(1);
            }

            format!("[{}]", stack_elements.join(", "))
        }

        f.debug_struct("Frame")
            .field(
                "current method",
                &format!("{}::>{}", self.current_method.holder.name(), self.current_method.signature()),
            )
            .field("bc idx", &self.bytecode_idx)
            .field("args", {
                let args: Vec<String> = (0..self.nbr_args).map(|idx| format!("{:?}", self.lookup_argument(idx))).collect();
                &format!("[{}]", args.join(", "))
            })
            .field("locals", {
                let locals: Vec<String> = (0..self.nbr_locals).map(|idx| format!("{:?}", self.lookup_local(idx))).collect();
                &format!("[{}]", locals.join(", "))
            })
            .field("stack", unsafe { &stack_printer(self) })
            .finish()
    }
}
