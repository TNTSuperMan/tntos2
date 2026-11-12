use crate::{
    asm::{LgdtField, load_gdt},
    segment::descriptor::{DescriptorType, SegmentDescriptor},
};

mod descriptor;

const GDT_SIZE: usize = 3;
static GDT: spin::Mutex<[SegmentDescriptor; GDT_SIZE]> =
    spin::Mutex::new([SegmentDescriptor::ZERO; GDT_SIZE]);

pub fn init() {
    let mut gdt = GDT.lock();

    *gdt = [
        SegmentDescriptor::ZERO,
        SegmentDescriptor::new(DescriptorType::ExecuteRead, 0, 0, 0xFFFFF, true),
        SegmentDescriptor::new(DescriptorType::ReadWrite, 0, 0, 0xFFFFF, false),
    ];

    load_gdt(&LgdtField {
        limit: (size_of::<SegmentDescriptor>() * GDT_SIZE - 1) as u16,
        offset: gdt.as_ptr().addr() as u64,
    });
}
