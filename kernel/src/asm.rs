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
            "mov dx, {0:x}
            mov eax, {1:e}
            out dx, eax", in(reg) addr, in(reg) data);
    }
}

pub fn io_in(addr: u16) -> u32 {
    let mut data = 0;
    unsafe {
        core::arch::asm!(
            "mov dx, {0:x}
            in eax, dx
            mov {1:e}, eax", in(reg) addr, out(reg) data);
    }
    data
}

pub fn load_idt(limit: u16, offset: u64) {
    let mut data = [0u8; 10];
    for (i, o) in limit.to_le_bytes().into_iter().zip(data.iter_mut()) {
        *o = i;
    }
    for (i, o) in offset
        .to_le_bytes()
        .into_iter()
        .zip(data.iter_mut().skip(2))
    {
        *o = i;
    }
    let addr = data.as_ptr().addr();
    unsafe {
        core::arch::asm!(
            "mov rax, {0}
            lidt [rax]",
            in(reg) addr,
        );
    }
}
