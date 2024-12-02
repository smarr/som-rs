use crate::compiler::Literal;
use crate::value::Value;
use crate::vm_objects::block::{Block, BodyInlineCache};
use crate::vm_objects::class::Class;
use crate::vm_objects::method::Method;
use crate::HACK_FRAME_FRAME_ARGS_PTR;
use core::mem::size_of;
use som_core::bytecode::Bytecode;
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::Gc;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::DerefMut;

pub(crate) const OFFSET_TO_STACK: usize = size_of::<Frame>();

/// Represents a stack frame.
pub struct Frame {
    /// The previous frame. Frames are handled as a linked list
    pub prev_frame: Gc<Frame>,

    /// The method the execution context currently is in.
    /// Interestingly, this is a Gc<Method> and not a pointer to MethodInfo, what we really need (it's never a primitive).
    /// In fact, we induce (minimal) runtime overhead by having to fetch the info from the Method enum regularly.
    /// So why do we do things that way? Because of moving GC. If we have a pointer to a MethodInfo, that's an inner pointer to a Method object. So when GC moves the frame, it can't update that pointer.
    /// It could update if Gc<MethodInfo> was a thing. And it was, at some point, and it turned out that really broke things (not sure why, I assume because MMTk didn't play well with a Gc<Enum> that had a Gc<SomethingElse> variant)  
    pub current_context: Gc<Method>,

    /// Bytecode index.
    pub bytecode_idx: u16,

    /// Stack pointer/index. Points to the NEXT element that can be written to the stack;
    /// Alternatively, can be seen as number of elements on the stack
    pub stack_ptr: u8,

    // pub stack_ptr: *mut Value,
    pub args_ptr: *mut Value,
    pub locals_ptr: *mut Value,

    /// markers. we don't use them directly. it's mostly a reminder that the struct looks different in memory... not the cleanest but not sure how else to go about it
    pub stack_marker: PhantomData<[Value]>,
    pub args_marker: PhantomData<[Value]>,
    pub locals_marker: PhantomData<[Value]>,
}

impl Frame {
    pub fn alloc_from_method_from_frame(
        method: Gc<Method>,
        nbr_args: usize,
        prev_frame: &mut Gc<Frame>,
        gc_interface: &mut GCInterface,
    ) -> Gc<Frame> {
        let (max_stack_size, nbr_locals) = match &*method {
            Method::Defined(m_env) => (m_env.max_stack_size as usize, m_env.nbr_locals),
            _ => unreachable!("if we're allocating a method frame, it has to be defined."),
        };

        let size = Frame::get_true_size(max_stack_size, nbr_args, nbr_locals);

        prev_frame.stack_push(Value::new_invokable(method));

        let mut frame_ptr: Gc<Frame> = gc_interface.request_memory_for_type(size);

        // ...I spent ages debugging a release-only bug, and this turned out to be the fix.
        // Whatever rust thinks it CAN do with the prev_frame ref (likely assume it points to the same data), it can't do safely in some cases... So we tell it not to.
        std::hint::black_box(&prev_frame);

        let method = prev_frame.stack_pop().as_invokable().unwrap();
        *frame_ptr = Frame::from_method(method);
        let args = prev_frame.stack_n_last_elements(nbr_args);

        Frame::init_frame_post_alloc(frame_ptr, args, max_stack_size, *prev_frame);

        prev_frame.remove_n_last_elements(nbr_args);
        frame_ptr
    }

    pub fn alloc_from_method_with_args(method: Gc<Method>, args: &[Value], prev_frame: &mut Gc<Frame>, gc_interface: &mut GCInterface) -> Gc<Frame> {
        let (max_stack_size, nbr_locals) = match &*method {
            Method::Defined(m_env) => (m_env.max_stack_size as usize, m_env.nbr_locals),
            _ => unreachable!("if we're allocating a method frame, it has to be defined."),
        };

        let size = Frame::get_true_size(max_stack_size, args.len(), nbr_locals);

        unsafe {
            HACK_FRAME_FRAME_ARGS_PTR = Some(Vec::from(args));
        }

        prev_frame.stack_push(Value::new_invokable(method));

        let mut frame_ptr: Gc<Frame> = gc_interface.request_memory_for_type(size);

        unsafe {
            let method = prev_frame.stack_pop().as_invokable().unwrap();
            *frame_ptr = Frame::from_method(method);
            Frame::init_frame_post_alloc(
                frame_ptr,
                HACK_FRAME_FRAME_ARGS_PTR.as_ref().unwrap().as_slice(),
                max_stack_size,
                *prev_frame,
            );

            HACK_FRAME_FRAME_ARGS_PTR = None;
        }

        frame_ptr
    }

    pub fn alloc_from_block(block: Gc<Block>, args: &[Value], prev_frame: &mut Gc<Frame>, gc_interface: &mut GCInterface) -> Gc<Frame> {
        let max_stack_size = block.blk_info.get_env().max_stack_size as usize;
        let nbr_locals = block.blk_info.get_env().nbr_locals;

        let size = Frame::get_true_size(max_stack_size, nbr_locals, args.len());

        unsafe {
            HACK_FRAME_FRAME_ARGS_PTR = Some(Vec::from(args));
        }

        prev_frame.stack_push(Value::new_block(block));

        let mut frame_ptr: Gc<Frame> = gc_interface.request_memory_for_type(size);

        unsafe {
            let block = prev_frame.stack_pop().as_block().unwrap();
            *frame_ptr = Frame::from_block(block);
            Frame::init_frame_post_alloc(
                frame_ptr,
                HACK_FRAME_FRAME_ARGS_PTR.as_ref().unwrap().as_slice(),
                max_stack_size,
                *prev_frame,
            );
        }

        unsafe {
            HACK_FRAME_FRAME_ARGS_PTR = None;
        }

        frame_ptr
    }

    /// Allocates the very first frame, for the `initialize:` call and tests.
    /// Special-cased because the normal case pushes the previous value on the previous frame's
    /// stack for it to be reachable: we have no previous frame in some cases, so we can't.
    pub fn alloc_initial_method(init_method: Gc<Method>, args: &[Value], gc_interface: &mut GCInterface) -> Gc<Frame> {
        let (max_stack_size, nbr_locals) = match &*init_method {
            Method::Defined(m_env) => (m_env.max_stack_size as usize, m_env.nbr_locals),
            _ => unreachable!("if we're allocating a method frame, it has to be defined."),
        };

        let size = Frame::get_true_size(max_stack_size, args.len(), nbr_locals);

        let nbr_gc_runs = gc_interface.get_nbr_collections();
        let mut frame_ptr: Gc<Frame> = gc_interface.request_memory_for_type(size);

        assert_eq!(
            nbr_gc_runs,
            gc_interface.get_nbr_collections(),
            "We assume we can't trigger a collection when allocating a parent-less frame"
        );

        *frame_ptr = Frame::from_method(init_method);
        Frame::init_frame_post_alloc(frame_ptr, args, max_stack_size, Gc::default());

        frame_ptr
    }

    fn init_frame_post_alloc(frame_ptr: Gc<Frame>, args: &[Value], stack_size: usize, prev_frame: Gc<Frame>) {
        unsafe {
            let mut frame = frame_ptr;

            frame.stack_ptr = 0;

            // setting up the self-referential pointers for args/locals accesses
            frame.args_ptr = (frame_ptr.ptr + OFFSET_TO_STACK + stack_size * size_of::<Value>()) as *mut Value;
            frame.locals_ptr = frame.args_ptr.add(args.len());

            // initializing arguments from the args slice
            std::slice::from_raw_parts_mut(frame.args_ptr, args.len()).copy_from_slice(args);

            // setting all locals to NIL.
            for idx in 0..frame.get_nbr_locals() {
                *frame.locals_ptr.add(idx) = Value::NIL;
            }

            frame.prev_frame = prev_frame; // because GC can have moved the previous frame!
        }
    }

    // Creates a frame from a block. Meant to only be called by the alloc_from_block function
    fn from_block(block: Gc<Block>) -> Self {
        Self {
            prev_frame: Gc::default(),
            current_context: block.blk_info,
            bytecode_idx: 0,
            stack_ptr: 0,
            args_ptr: std::ptr::null_mut(),
            locals_ptr: std::ptr::null_mut(),
            args_marker: PhantomData,
            locals_marker: PhantomData,
            stack_marker: PhantomData,
        }
    }

    // Creates a frame from a block. Meant to only be called by the alloc_from_method function
    fn from_method(method: Gc<Method>) -> Self {
        Self {
            prev_frame: Gc::default(),
            current_context: method,
            bytecode_idx: 0,
            stack_ptr: 0,
            args_ptr: std::ptr::null_mut(),
            locals_ptr: std::ptr::null_mut(),
            args_marker: PhantomData,
            locals_marker: PhantomData,
            stack_marker: PhantomData,
        }
    }

    /// Returns the true size of the `Frame`, counting the extra memory needed for its stack/locals/arguments.
    pub fn get_true_size(max_stack_size: usize, nbr_args: usize, nbr_locals: usize) -> usize {
        size_of::<Frame>() + ((max_stack_size + nbr_args + nbr_locals) * size_of::<Value>())
    }

    #[inline(always)]
    pub fn get_bytecode_ptr(&self) -> *const Vec<Bytecode> {
        &self.current_context.get_env().body
    }

    #[inline(always)]
    pub fn get_max_stack_size(&self) -> usize {
        self.current_context.get_env().max_stack_size as usize
    }

    #[inline(always)]
    pub fn get_inline_cache(&mut self) -> &mut BodyInlineCache {
        match self.current_context.deref_mut() {
            Method::Defined(env) => &mut env.inline_cache,
            Method::Primitive(_, _, _) => unreachable!(),
        }
    }

    #[inline(always)]
    pub fn get_nbr_args(&self) -> usize {
        self.current_context.get_env().nbr_params + 1
    }

    #[inline(always)]
    pub fn get_nbr_locals(&self) -> usize {
        self.current_context.get_env().nbr_locals
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
        // TODO: just self.current_context.holder instead? most likely.
        match self.lookup_argument(0).as_block() {
            Some(b) => {
                let block_frame = b.frame.as_ref().unwrap();
                block_frame.get_method_holder()
            }
            None => self.current_context.get_env().holder,
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

    #[inline(always)]
    pub fn lookup_constant(&self, idx: usize) -> Literal {
        self.current_context.get_env().literals.get(idx).unwrap().clone()
    }

    pub fn nth_frame_back(current_frame: &Gc<Frame>, n: u8) -> Gc<Frame> {
        if n == 0 {
            return *current_frame;
        }

        let mut target_frame: Gc<Frame> = match current_frame.lookup_argument(0).as_block() {
            Some(block) => *block.frame.as_ref().unwrap(),
            None => panic!(
                "attempting to access a non local var/arg from a method instead of a block: self wasn't blockself but {:?}.",
                current_frame.lookup_argument(0)
            ),
        };
        for _ in 1..n {
            target_frame = match &target_frame.lookup_argument(0).as_block() {
                Some(block) => {
                    *block.frame.as_ref().unwrap()
                }
                None => panic!("attempting to access a non local var/arg from a method instead of a block (but the original frame we were in was a block): self wasn't blockself but {:?}.", current_frame.lookup_argument(0))
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

    /// Gets the nth element from the stack (not in reverse order - "3" yields the 3rd element from the bottom, not the top)
    /// # Safety
    /// The caller needs to ensure this is a valid stack value. That means not outside the stack's maximum size, and not pointing to an uninitialized value.
    #[inline(always)]
    pub unsafe fn nth_stack(&self, n: u8) -> &Value {
        let stack_ptr = self as *const Self as usize + OFFSET_TO_STACK;
        let val_ptr = stack_ptr + (n as usize * size_of::<Value>());
        &*(val_ptr as *const Value)
    }

    /// Gets the nth element from the stack mutably (not in reverse order - "3" yields the 3rd element from the bottom, not the top)
    /// # Safety
    /// The caller needs to ensure this is a valid stack value. That means not outside the stack's maximum size, and not pointing to an uninitialized value.
    #[inline(always)]
    pub unsafe fn nth_stack_mut(&mut self, n: u8) -> &mut Value {
        let stack_ptr = self as *mut Self as usize + OFFSET_TO_STACK;
        let val_ptr = stack_ptr + (n as usize * size_of::<Value>());
        &mut *(val_ptr as *mut Value)
    }

    #[inline(always)]
    pub fn stack_push(&mut self, value: Value) {
        debug_assert!(self.stack_ptr < self.current_context.get_env().max_stack_size);
        unsafe {
            *self.nth_stack_mut(self.stack_ptr) = value;
            self.stack_ptr += 1;
        }
    }

    #[inline(always)]
    pub fn stack_pop(&mut self) -> Value {
        debug_assert!(self.stack_ptr > 0);
        unsafe {
            self.stack_ptr -= 1;
            *self.nth_stack_mut(self.stack_ptr)
        }
    }

    #[inline(always)]
    pub fn stack_last(&self) -> &Value {
        debug_assert!(self.stack_ptr > 0);
        unsafe { self.nth_stack(self.stack_ptr - 1) }
    }

    #[inline(always)]
    pub fn stack_last_mut(&mut self) -> &mut Value {
        debug_assert!(self.stack_ptr > 0);
        unsafe { self.nth_stack_mut(self.stack_ptr - 1) }
    }

    #[inline(always)]
    pub fn stack_nth_back(&self, n: usize) -> &Value {
        debug_assert!(self.stack_ptr >= (n + 1) as u8);
        unsafe { self.nth_stack(self.stack_ptr - (n as u8 + 1)) }
    }

    #[inline(always)]
    pub fn stack_n_last_elements(&self, n: usize) -> &[Value] {
        unsafe {
            let slice_ptr = self.nth_stack(self.stack_ptr - n as u8);
            std::slice::from_raw_parts(slice_ptr, n)
        }
    }

    #[inline(always)]
    pub fn remove_n_last_elements(&mut self, n: usize) {
        self.stack_ptr -= n as u8
    }

    /// Gets the total number of elements on the stack. Only used for debugging.
    pub fn stack_len(&self) -> usize {
        self.stack_ptr as usize
    }
}

pub struct FrameStackIter<'a> {
    frame: &'a Frame,
    stack_idx: u8,
}

impl<'a> From<&'a Frame> for FrameStackIter<'a> {
    fn from(frame: &'a Frame) -> Self {
        Self { frame, stack_idx: 0 }
    }
}

impl<'a> Iterator for FrameStackIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stack_idx >= self.frame.stack_ptr {
            return None;
        }

        let val = unsafe { self.frame.nth_stack(self.stack_idx) };
        self.stack_idx += 1;
        Some(val)
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn stack_printer(frame: &Frame) -> String {
            let stack_elems = FrameStackIter::from(frame).map(|val| format!("{:?}", val)).collect::<Vec<_>>();
            format!("[{}]", stack_elems.join(", "))
        }

        f.debug_struct("Frame")
            .field(
                "current method",
                &format!("{}::>{}", self.current_context.holder().name(), self.current_context.signature()),
            )
            .field("bc idx", &self.bytecode_idx)
            .field("args", {
                let args: Vec<String> = (0..self.get_nbr_args()).map(|idx| format!("{:?}", self.lookup_argument(idx))).collect();
                &format!("[{}]", args.join(", "))
            })
            .field("locals", {
                let locals: Vec<String> = (0..self.get_nbr_locals()).map(|idx| format!("{:?}", self.lookup_local(idx))).collect();
                &format!("[{}]", locals.join(", "))
            })
            .field("stack", &stack_printer(self))
            .finish()
    }
}
