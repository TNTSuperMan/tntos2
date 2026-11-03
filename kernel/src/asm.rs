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
    unsafe {
        core::arch::asm!(
            "lidt [rax]",
            in("rax") data.as_ptr(),
        );
    }
}
