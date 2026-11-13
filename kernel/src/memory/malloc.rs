use core::{
    alloc::{AllocError, Allocator, GlobalAllocator, Layout, StaticAllocator},
    ptr::NonNull,
};

use crate::memory::manager::{BYTES_PER_FRAME, Frames, MEMORY_MANAGER};

struct SimpleBumpMalloc {
    frame: Frames,
    index: usize,
}
impl SimpleBumpMalloc {
    fn alloc(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // 少なくともフレームは4KiBのalignがあるためこれで多分OK
        if layout.align() > 4096 {
            return Err(AllocError);
        }
        while self.index % layout.align() != 0 {
            // 数学弱いごめんLLVMさんよろ
            self.index += 1;
        }
        let begin_at = self.index;
        self.index += layout.size();
        if self.index < self.frame.bytes_len() {
            // SAFETY: self.indexがself.frameの正しい長さ未満にあることは確認されました
            NonNull::new(unsafe { self.frame.as_mut_ptr().byte_add(begin_at) as *mut [u8] })
                .ok_or(AllocError)
        } else {
            self.index = 0;
            let old_frame = core::mem::replace(
                &mut self.frame,
                MEMORY_MANAGER
                    .lock()
                    .allocate(layout.size().div_ceil(BYTES_PER_FRAME))
                    .ok_or(AllocError)?,
            );
            core::mem::forget(old_frame);

            NonNull::new(self.frame.as_mut_ptr() as *mut [u8]).ok_or(AllocError)
        }
    }
}

static GLOBAL_LOCK: spin::Once<spin::Mutex<SimpleBumpMalloc>> = spin::Once::new();

struct Global;
unsafe impl Allocator for Global {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        GLOBAL_LOCK
            .try_call_once(|| {
                Ok(spin::Mutex::new(SimpleBumpMalloc {
                    frame: MEMORY_MANAGER.lock().allocate(1).ok_or(AllocError)?,
                    index: 0,
                }))
            })?
            .lock()
            .alloc(layout)
    }
    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        crate::println!("deallocate todo lol");
    }
}
unsafe impl StaticAllocator for Global {}
unsafe impl GlobalAllocator for Global {}

#[global_allocator]
static GLOBAL: Global = Global;
