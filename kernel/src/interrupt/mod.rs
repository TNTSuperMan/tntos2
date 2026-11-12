use crate::{
    asm::{LidtField, get_cs, load_idt},
    interrupt::descriptor::{Descriptor, DescriptorType, IDT, make_attr},
};

mod descriptor;

pub fn init_interrupt() {
    let mut idt = IDT.lock();

    // PF
    idt[14] = Descriptor::new(
        make_attr(DescriptorType::InterruptGate, 0, true, 0),
        (int_handle::<14> as *const ()).addr() as u64,
        get_cs(),
    );

    load_idt(&LidtField {
        limit: size_of::<[Descriptor; 256]>() as u16 - 1,
        offset: idt.as_ptr().addr() as u64,
    });
}

#[derive(Debug)]
#[repr(C)]
struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

extern "x86-interrupt" fn int_handle<const ID: u8>(frame: *const InterruptFrame, code: u64) {
    crate::println!("{frame:?} {code}");
    crate::asm::stop();
}
