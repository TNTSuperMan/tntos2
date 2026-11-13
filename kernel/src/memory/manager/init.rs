use uefi::mem::memory_map::MemoryMap;

use crate::memory::{
    manager::{
        MEMORY_MANAGER,
        manager::{BYTES_PER_FRAME, MemoryManager},
    },
    uefi_mmap::{MEMORY_MAP, is_available},
};

const UEFI_PAGE_BYTES: u64 = 4096;

fn into_frame(val: u64) -> usize {
    (val as usize).div_ceil(BYTES_PER_FRAME)
}

pub fn init() {
    let mut memory_manager = MemoryManager::ZEROED;

    let memory_map = MEMORY_MAP.get().unwrap();

    let mut available_end = 0u64;

    for desc in memory_map.entries() {
        if available_end < desc.phys_start {
            memory_manager.mark_allocated(
                into_frame(available_end),
                into_frame(desc.phys_start - available_end),
            );
        }
        let phys_end = desc.phys_start + desc.page_count * UEFI_PAGE_BYTES;
        if is_available(desc.ty) {
            available_end = phys_end;
        } else {
            memory_manager.mark_allocated(
                into_frame(desc.phys_start),
                into_frame(desc.page_count * UEFI_PAGE_BYTES),
            );
        }
    }
    memory_manager.range = 1..into_frame(available_end);

    *MEMORY_MANAGER.lock() = memory_manager;
}
