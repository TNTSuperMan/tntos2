#![no_main]
#![no_std]

use crate::{
    pci::{DEVICES, init_pci},
    screen::{Color, SCREEN},
};
use core::fmt::Write;

mod asm;
mod pci;
mod screen;
mod utils;
mod xhc;

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "sysv64" fn kernel_entry(options: *const shared::KernelEntrypointOptions) -> ! {
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

    let mut screen_lock = SCREEN.lock();
    let screen = screen_lock.as_mut().unwrap();

    screen.draw_rect(0, 0, screen.width(), screen.height(), Color::WHITE);
    let mut writer = screen.str_writer(100, 100, Color::BLACK);
    writer.write_str("Hello, World!\n").unwrap();

    for device in DEVICES.get().unwrap().iter() {
        writer.write_fmt(format_args!("{device:?}\n")).unwrap();
    }

    halt();
}

#[panic_handler]
fn on_panic(info: &core::panic::PanicInfo) -> ! {
    SCREEN.try_lock().map(|mut lock| {
        lock.as_mut().map(|screen| {
            screen.draw_rect(0, 0, screen.width(), screen.height(), screen::Color::BLUE);

            let _ = screen
                .str_writer(50, 50, Color::WHITE)
                .write_fmt(format_args!(":(\ntntos2 {info}"));
        })
    });

    halt()
}
