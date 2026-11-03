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

core::arch::global_asm!(
    "
.text

.global _start
_start:
    call rust_main
    "
);

#[unsafe(no_mangle)]
extern "sysv64" fn rust_main(options: *const shared::KernelEntrypointOptions) -> ! {
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

    init_pci();
    init_interrupt();

    let mut screen_lock = SCREEN.lock();
    let screen = screen_lock.as_mut().unwrap();

    screen.draw_rect(0, 0, screen.width(), screen.height(), Color::WHITE);
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
