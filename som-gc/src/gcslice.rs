use std::marker::PhantomData;

use mmtk::util::Address;

/// Special GC ref that stores a list.
/// It's really just a `Vec<T>` replacement, where Rust manages none of the memory itself.
/// Used because finalization might be a slowdown if we stored references to `Vec`s on the heap?
#[derive(Debug, Clone, Copy)]
pub struct GcSlice<T: Sized> {
    pub ptr: Address, // this should be right after the GcSlice for cache reasons, I'd say
    pub len: usize,
    _phantom: PhantomData<T>,
}

impl<T> GcSlice<T>
where
    T: std::fmt::Debug,
{
    pub fn new(ptr: Address, len: usize) -> GcSlice<T> {
        debug_assert!(!ptr.is_zero());
        GcSlice {
            ptr,
            len,
            _phantom: PhantomData,
        }
    }

    pub fn iter(&self) -> GCSliceIter<T> {
        GCSliceIter { gc_slice: self, cur_idx: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ref(), self.len) }
    }

    #[inline(always)]
    pub fn get(&self, idx: usize) -> &T {
        debug_assert!(idx < self.len);
        unsafe { self.ptr.add(idx * std::mem::size_of::<T>()).as_ref() }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, idx: usize) -> &mut T {
        debug_assert!(idx < self.len);
        unsafe { self.ptr.add(idx * std::mem::size_of::<T>()).as_mut_ref() }
    }

    pub fn set(&self, idx: usize, val: T) {
        debug_assert!(idx < self.len);
        unsafe {
            let val_ptr = self.ptr.add(idx * std::mem::size_of::<T>()).as_mut_ref();
            *val_ptr = val
        }
    }
}

impl<T> PartialEq for GcSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr // not correct, should compare each element individually instead.
    }
}

impl<T> From<&GcSlice<T>> for Address {
    fn from(ptr: &GcSlice<T>) -> Self {
        Address::from_ref(ptr)
    }
}

pub struct GCSliceIter<'a, T> {
    gc_slice: &'a GcSlice<T>,
    cur_idx: usize,
}

impl<'a, T: std::fmt::Debug> Iterator for GCSliceIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_idx >= self.gc_slice.len {
            return None;
        }

        //dbg!(&self.gc_slice.as_slice());

        let item = self.gc_slice.get(self.cur_idx);
        self.cur_idx += 1;
        Some(item)
    }
}
