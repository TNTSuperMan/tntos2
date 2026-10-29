#![no_main]
#![no_std]

use crate::{pci::scan_all_bus, screen::SCREEN};
use core::fmt::Write;

mod asm;
mod pci;
mod screen;
mod utils;

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
    SCREEN.init(frame_buffer);

    unsafe {
        uefi::boot::set_image_handle(image);
        uefi::table::set_system_table(table.as_ptr());
    }

    SCREEN.draw_rect(0, 0, SCREEN.width(), SCREEN.height(), screen::Color::WHITE);
    let mut writer = SCREEN.str_writer(100, 100, screen::Color::BLACK);
    writer.write_str("Hello, World!\n").unwrap();

    let devices = scan_all_bus().unwrap();

    for dev in devices.iter() {
        writer.write_fmt(format_args!("{dev:?}\n")).unwrap();
    }

    halt();
}

#[panic_handler]
fn on_panic(info: &core::panic::PanicInfo) -> ! {
    SCREEN.draw_rect(0, 0, SCREEN.width(), SCREEN.height(), screen::Color::BLUE);

    let _ = SCREEN
        .str_writer(50, 50, screen::Color::WHITE)
        .write_fmt(format_args!(":(\ntntos2 {info}"));

    halt()
}
