use crate::{
    asm::{LgdtField, load_gdt},
    memory::segment::descriptor::{DescriptorType, SegmentDescriptor},
};

mod descriptor;

const GDT_SIZE: usize = 3;
static mut GDT: [SegmentDescriptor; GDT_SIZE] = [SegmentDescriptor::ZERO; GDT_SIZE];

pub fn init() {
    let gdt = core::ptr::addr_of!(GDT).cast_mut();

    unsafe {
        gdt.write([
            SegmentDescriptor::ZERO,
            SegmentDescriptor::new(DescriptorType::ExecuteRead, 0, 0, 0xFFFFF, true),
            SegmentDescriptor::new(DescriptorType::ReadWrite, 0, 0, 0xFFFFF, false),
        ]);
    }

    load_gdt(&LgdtField {
        limit: (size_of::<SegmentDescriptor>() * GDT_SIZE - 1) as u16,
        offset: gdt.addr() as u64,
    });
}
