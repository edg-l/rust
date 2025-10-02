use edos_rt::allocator::PoolAllocator;

use crate::alloc::{GlobalAlloc, Layout, System};

static EDOS_ALLOC: PoolAllocator = PoolAllocator::new();

#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { EDOS_ALLOC.alloc(layout) }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { EDOS_ALLOC.dealloc(ptr, layout) };
    }
}
