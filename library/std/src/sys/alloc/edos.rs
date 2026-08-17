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
    // The allocator resizes in place where the block already covers the new
    // size or the block after it is free, which is most of what a growing
    // buffer asks for; the fallback would copy every time.
    unsafe { EDOS_ALLOC.realloc(ptr, layout, new_size) }
}
