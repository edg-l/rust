use edos_rt::allocator::PoolAllocator;

use crate::alloc::{GlobalAlloc, Layout};

static EDOS_ALLOC: PoolAllocator = PoolAllocator::new();

#[inline]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    unsafe { EDOS_ALLOC.alloc(layout) }
}

#[inline]
pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    unsafe { EDOS_ALLOC.dealloc(ptr, layout) }
}

#[inline]
pub unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    // SAFETY: this is just a `pub` wrapper.
    unsafe { super::realloc_fallback(ptr, layout, new_size) }
}
