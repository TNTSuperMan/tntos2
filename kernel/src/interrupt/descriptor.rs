use spin::Mutex;

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct Descriptor {
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

#[repr(u8)]
pub enum DescriptorType {
    UpperuBytes = 0,
    LDT = 2,
    TSSAvailable = 9,
    TSSBusy = 11,
    CallGate = 12,
    InterruptGate = 14,
    TrapGate = 15,
}

pub fn make_attr(typ: DescriptorType, privilege_lv: u8, present: bool, ist: u8) -> u16 {
    ist as u16
        | (typ as u8 as u16) << 8
        | (privilege_lv as u16) << 12
        | if present { 0b1000000000000000 } else { 0 }
}

pub static IDT: Mutex<[Descriptor; 256]> = Mutex::new([Descriptor::zeroed(); 256]);
