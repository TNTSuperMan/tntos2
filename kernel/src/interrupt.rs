use spin::Mutex;

use crate::asm::load_idt;

#[repr(packed)]
#[derive(Clone, Copy)]
struct Descriptor {
    offset_low: u16,
    segment_selector: u16,
    attribute: u16,
    offset_middle: u16,
    offset_high: u32,
    _reserved: u32,
}
impl Descriptor {
    pub const fn zeroed() -> Self {
        Self {
            offset_low: 0,
            segment_selector: 0,
            attribute: 0,
            offset_middle: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }
    pub fn new(attribute: u16, offset: u64, segment_selector: u16) -> Self {
        Self {
            attribute,
            offset_low: offset as u16,
            offset_middle: (offset >> 16) as u16,
            offset_high: (offset >> 32) as u32,
            segment_selector,
            _reserved: 0,
        }
    }
}

static IDT: Mutex<[Descriptor; 256]> = Mutex::new([Descriptor::zeroed(); 256]);

pub fn init_interrupt() {
    let size = size_of::<[Descriptor; 256]>();
    let ptr = IDT.lock().as_ptr();
    load_idt(size as u16, ptr.addr() as u64);
}
