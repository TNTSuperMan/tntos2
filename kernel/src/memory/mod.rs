use crate::asm::{set_csss, set_ds_all};

mod manager;
mod paging;
mod segment;
pub mod uefi_mmap;

pub use manager::{FrameId, MEMORY_MANAGER, MemoryManager};

const KERNEL_CS: u16 = 1 << 3;
const KERNEL_SS: u16 = 2 << 3;

pub fn init() {
    segment::init();
    set_ds_all(0);
    set_csss(KERNEL_CS, KERNEL_SS);
    paging::setup_identity_page_table();
    manager::init();
}
