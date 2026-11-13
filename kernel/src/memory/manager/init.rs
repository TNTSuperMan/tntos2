use uefi::mem::memory_map::MemoryMap;

use crate::memory::{
    manager::{MEMORY_MANAGER, frameid::FrameId, manager::MemoryManager},
    uefi_mmap::{MEMORY_MAP, is_available},
};

const UEFI_PAGE_BYTES: u64 = 4096;

pub fn init() {
    let mut memory_manager = MemoryManager::ZEROED;

    let memory_map = MEMORY_MAP.get().unwrap();

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
