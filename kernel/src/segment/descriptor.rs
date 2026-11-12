#[repr(u8)]
pub enum DescriptorType {
    ReadWrite = 2,
    ExecuteRead = 10,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    flags1: u8, // PDDSTTTT
    flags2: u8, // GDLALLLL
    base_high: u8,
}
impl SegmentDescriptor {
    pub const ZERO: SegmentDescriptor = SegmentDescriptor {
        limit_low: 0,
        base_low: 0,
        base_middle: 0,
        flags1: 0,
        flags2: 0,
        base_high: 0,
    };
    pub fn new(
        typ: DescriptorType,
        desc_privilege_lv: u32,
        base: u32,
        limit: u32,
        is_code: bool,
    ) -> Self {
        let mut desc = SegmentDescriptor::ZERO;

        desc.base_low = base as u16;
        desc.base_middle = (base >> 16) as u8;
        desc.base_high = (base >> 24) as u8;

        desc.limit_low = limit as u16;
        let limit_high = ((limit >> 16) & 0b1111) as u8;

        desc.flags1 = (typ as u8) | 0b10010000 | (desc_privilege_lv & 0b11 << 5) as u8; // Present, SystemSegment
        desc.flags2 = limit_high
            | if is_code {
                0b10100000 // Granularity, !DefaultOperationSize, LongMode, !Available
            } else {
                0b11000000 // Granularity, DefaultOperationSize, !LongMode, !Available
            };

        desc
    }
}
