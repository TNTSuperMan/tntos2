pub fn stop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn io_out(addr: u16, data: u32) {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") addr,
            in("eax") data,
        );
    }
}

pub fn io_in(addr: u16) -> u32 {
    let mut data = 0;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            in("dx") addr,
            out("eax") data,
        );
    }
    data
}

pub fn get_cs() -> u16 {
    let mut data: u16 = 0;
    unsafe {
        core::arch::asm!(
            "mov ax, cs",
            out("ax") data,
        );
    }
    data
}

#[repr(C, packed)]
pub struct LidtField {
    pub limit: u16,
    pub offset: u64,
}
pub fn load_idt(field: &LidtField) {
    let addr = core::ptr::addr_of!(*field);

    unsafe {
        core::arch::asm!(
            "lidt [rax]",
            in("rax") addr,
        );
    }
}

#[repr(C, packed)]
pub struct LgdtField {
    pub limit: u16,
    pub offset: u64,
}
pub fn load_gdt(field: &LgdtField) {
    let addr = core::ptr::addr_of!(*field);

    unsafe {
        core::arch::asm!(
            "lgdt [rax]",
            in("rax") addr,
        );
    }
}

pub fn set_ds_all(value: u16) {
    unsafe {
        core::arch::asm!(
            "mov ds, di
            mov es, di
            mov fs, di
            mov gs, di",
            in("di") value,
        );
    }
}

pub fn set_csss(cs: u16, ss: u16) {
    todo!()
}

pub fn set_cr3(value: u64) {
    unsafe {
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) value,
        );
    }
}
