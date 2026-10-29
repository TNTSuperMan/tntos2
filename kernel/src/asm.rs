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
