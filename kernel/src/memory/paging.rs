use crate::asm::set_cr3;

const PAGE_DIR_COUNT: usize = 0;

const P4K: usize = 4096;
const P2M: usize = P4K * 512;
const P1G: usize = P2M * 512;

#[repr(align(4096))]
struct PageTable {
    pml4: [u64; 512],
    pdp: [u64; 512],
    page_directory: [[u64; 512]; PAGE_DIR_COUNT],
}

static PAGE_TABLE: spin::Mutex<PageTable> = spin::Mutex::new(PageTable {
    pml4: [0; 512],
    pdp: [0; 512],
    page_directory: [[0; 512]; PAGE_DIR_COUNT],
});

pub fn setup_identity_page_table() {
    let mut page_table = PAGE_TABLE.lock();

    page_table.pml4[0] = (page_table.pdp.as_ptr().addr() as u64) | 0x003;
    for i_pdpt in 0..PAGE_DIR_COUNT {
        page_table.pdp[i_pdpt] =
            unsafe { page_table.page_directory.as_ptr().add(i_pdpt) }.addr() as u64 | 0x003;
        for i_pd in 0..512 {
            page_table.page_directory[i_pdpt][i_pd] = (i_pdpt * P1G + i_pd * P2M | 0x083) as u64;
        }
    }
    set_cr3(page_table.pml4.as_ptr().addr() as u64);
}
