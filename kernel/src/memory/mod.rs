mod paging;
mod segment;
pub mod uefi_mmap;

pub fn init() {
    segment::init();
    paging::setup_identity_page_table();
}
