use crate::{
    asm::{LidtField, load_idt},
    interrupt::descriptor::{Descriptor, DescriptorType, IDT, make_attr},
    memory::KERNEL_CS,
};

mod descriptor;

pub fn init() {
    let idt = IDT.as_ptr().cast_mut();
    {
        fn addc<const ID: u8>(idt: *mut Descriptor) {
            unsafe {
                // SAFETY: IDはつねにu8(0..=256)の範囲内なのでダングリングポインタにはなり得ない
                // まだ割り込みは有効じゃないのでデータ競合にもならないはず
                idt.add(ID as usize).write(Descriptor::new(
                    make_attr(DescriptorType::InterruptGate, 0, true, 0),
                    (handle_fault_with_code::<ID> as *const ()).addr() as u64,
                    KERNEL_CS,
                ));
            }
        }
        fn addn<const ID: u8>(idt: *mut Descriptor) {
            unsafe {
                idt.add(ID as usize).write(Descriptor::new(
                    make_attr(DescriptorType::InterruptGate, 0, true, 0),
                    (handle_fault_no_code::<ID> as *const ()).addr() as u64,
                    KERNEL_CS,
                ));
            }
        }
        addc::<0>(idt); // DE
        addc::<1>(idt); // BD
        addc::<3>(idt); // BP
        addc::<4>(idt); // OF
        addc::<5>(idt); // BR
        addc::<6>(idt); // UD
        addc::<7>(idt); // NM
        addc::<8>(idt); // DF
        addc::<10>(idt); // TS
        addc::<11>(idt); // NP
        addc::<12>(idt); // SS
        addc::<13>(idt); // GP
        addc::<14>(idt); // PF

        addn::<16>(idt); // MF
        addn::<17>(idt); // AC
        addn::<18>(idt); // MC
        addn::<19>(idt); // XM
        addn::<20>(idt); // VE
    }

    load_idt(&LidtField {
        limit: size_of::<[Descriptor; 256]>() as u16 - 1,
        offset: idt.addr() as u64,
    });
}

#[repr(C)]
struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}
impl core::fmt::Debug for InterruptFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Interrupt frame:
RIP: {:>016X} RSP: {:>016X}
 CS: {:>016X}  SS: {:>016X}
rfg: {:>016X}",
            self.rip, self.rsp, self.cs, self.ss, self.rflags
        )
    }
}

extern "x86-interrupt" fn handle_fault_with_code<const ID: u8>(frame: InterruptFrame, code: u64) {
    panic!("fault occured. id: {ID} code: {code} {frame:?}");
}

extern "x86-interrupt" fn handle_fault_no_code<const ID: u8>(frame: InterruptFrame) {
    panic!("fault occured. id: {ID} {frame:?}");
}
