#![no_main]
#![no_std]

use crate::{
    asm::stop,
    interrupt::init_interrupt,
    pci::{DEVICES, init_pci},
    screen::{Color, SCREEN},
};
use core::fmt::Write;

mod asm;
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
    init_interrupt();

    let mut screen = SCREEN.lock();
    let width = screen.width();
    let height = screen.height();

    screen.draw_rect(0, 0, width, height, Color::WHITE);
    let mut writer = screen.str_writer(100, 100, Color::BLACK);
    writer.write_str("Hello, World!\n").unwrap();

    for device in DEVICES.get().unwrap().iter() {
        writer.write_fmt(format_args!("{device:?}\n")).unwrap();
    }

    writer
        .write_fmt(format_args!(
            "Stack position: {:X}",
            core::ptr::addr_of!(writer).addr()
        ))
        .unwrap();

    stop();
}
