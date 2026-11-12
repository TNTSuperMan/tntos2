use core::ops::Range;

use uefi::mem::memory_map::MemoryMap;

use crate::memory::uefi_mmap::is_available;

#[derive(Clone, Copy)]
pub struct FrameId(usize);
impl FrameId {
    pub const BYTES_PER_FRAME: usize = 4096;

    pub fn id(&self) -> usize {
        self.0
    }
    pub fn frame(&self) -> *mut () {
        unsafe { core::ptr::null_mut::<[u8; 4096]>().add(self.id()).cast() }
    }
}

pub struct MemoryManager {
    alloc_map: [u8; MemoryManager::FRAME_COUNT / 8], // 4MiBになる
    pub range: Range<usize>,
}
impl MemoryManager {
    const ZEROED: MemoryManager = MemoryManager {
        alloc_map: [0; MemoryManager::FRAME_COUNT / 8],
        range: 0..0,
    };

    pub const MAX_PHY_MEM_BYTES: usize = 1024 * 1024 * 1024 * 128; // 128GiB
    pub const FRAME_COUNT: usize = MemoryManager::MAX_PHY_MEM_BYTES / FrameId::BYTES_PER_FRAME;

    fn is_allocated(&self, frame: usize) -> bool {
        let arr_idx = frame >> 3;
        let bit_offset = frame & 0b111;
        self.alloc_map[arr_idx] & (1 << bit_offset) != 0
    }
    fn set_allocated(&mut self, frame: usize, allocated: bool) {
        let arr_idx = frame >> 3;
        let bit_offset = frame & 0b111;
        let bit = 1 << bit_offset;
        if allocated {
            self.alloc_map[arr_idx] |= bit;
        } else {
            self.alloc_map[arr_idx] &= !bit;
        }
    }

    pub fn mark_allocated(&mut self, start: FrameId, len: usize) {
        for i in 0..len {
            self.set_allocated(start.id() + i, true);
        }
    }
    pub fn allocate(&mut self, len: usize) -> Option<FrameId> {
        let mut curr_frame = self.range.start;

        'mainloop: loop {
            for i in 0..len {
                if self.is_allocated(curr_frame + i) {
                    curr_frame += i + 1;
                    continue 'mainloop;
                }
            }
            if self.range.contains(&(curr_frame + len - 1)) {
                self.mark_allocated(FrameId(curr_frame), len);
                break Some(FrameId(curr_frame));
            } else {
                break None;
            }
        }
    }
    pub fn free(&mut self, start: FrameId, len: usize) -> Result<(), ()> {
        for i in start.0..(start.0 + len) {
            if self.is_allocated(i) {
                self.set_allocated(i, false);
            } else {
                return Err(());
            }
        }
        Ok(())
    }
}

const UEFI_PAGE_BYTES: u64 = 4096;

pub static MEMORY_MANAGER: spin::Mutex<MemoryManager> = spin::Mutex::new(MemoryManager::ZEROED);

pub fn init() {
    let mut memory_manager = MemoryManager::ZEROED;

    let memory_map = super::uefi_mmap::MEMORY_MAP.get().unwrap();

    let mut available_end = 0u64;

    for desc in memory_map.entries() {
        if available_end < desc.phys_start {
            memory_manager.mark_allocated(
                FrameId(available_end as usize / FrameId::BYTES_PER_FRAME),
                (desc.phys_start - available_end) as usize / FrameId::BYTES_PER_FRAME,
            );
        }
        let phys_end = desc.phys_start + desc.page_count * UEFI_PAGE_BYTES;
        if is_available(desc.ty) {
            available_end = phys_end;
        } else {
            memory_manager.mark_allocated(
                FrameId(desc.phys_start as usize / FrameId::BYTES_PER_FRAME),
                (desc.page_count * UEFI_PAGE_BYTES) as usize / FrameId::BYTES_PER_FRAME,
            );
        }
    }
    memory_manager.range = 1..(available_end as usize / FrameId::BYTES_PER_FRAME);

    *MEMORY_MANAGER.lock() = memory_manager;
}
