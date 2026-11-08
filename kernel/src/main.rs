#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]

use crate::{
    asm::stop,
    interrupt::init_interrupt,
    pci::{DEVICES, init_pci},
    screen::{Color, SCREEN},
    xhc::init_xhc,
};
use core::fmt::Write;

mod asm;
mod console;
mod interrupt;
mod panic;
mod pci;
mod screen;
mod utils;
mod xhc;

// mainに直接置いてるとキモいので然るべきときが来たら別の場所に移動してください
#[repr(align(16))]
struct Stack([u8; 1048576]);
static mut KERNEL_STACK: Stack = Stack([0; 1048576]);

#[unsafe(no_mangle)]
extern "sysv64" fn _start(options: *const shared::KernelEntrypointOptions) -> ! {
    let shared::KernelEntrypointOptions {
        frame_buffer,
        image,
        table,
        ..
    } = unsafe { options.read() };

    unsafe {
        screen::assume_init_screen(frame_buffer);
        uefi::boot::set_image_handle(image);
        uefi::table::set_system_table(table.as_ptr());
    }

    let stack_pointer = unsafe { core::ptr::addr_of!(KERNEL_STACK.0) }.addr() + size_of::<Stack>();

    unsafe {
        core::arch::asm!(
            "mov rsp, {0}
            call main",
            in(reg) stack_pointer
        );
        core::hint::unreachable_unchecked();
    }
}

#[unsafe(no_mangle)]
extern "sysv64" fn main() -> ! {
    init_pci();
    init_xhc();
    init_interrupt();

    {
        let mut screen = SCREEN.lock();
        let width = screen.width();
        let height = screen.height();

        screen.draw_rect(0, 0, width, height, Color::WHITE);
    }

    for device in DEVICES.get().unwrap().iter() {
        println!("{device:?}");
    }
    let mut stack_tmp = [0];
    stack_tmp[core::hint::black_box(0)] = core::hint::black_box(0);
    println!("Stack position: {:X}", stack_tmp.as_ptr().addr());

    stop();
}
