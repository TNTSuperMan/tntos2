#![no_main]
#![no_std]

use crate::screen::SCREEN;
use core::fmt::Write;

mod screen;

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
    SCREEN
        .str_writer(100, 100, screen::Color::BLACK)
        .write_str("Hello, World!")
        .unwrap();

    todo!();
}

#[panic_handler]
fn on_panic(info: &core::panic::PanicInfo) -> ! {
    SCREEN.draw_rect(0, 0, SCREEN.width(), SCREEN.height(), screen::Color::BLUE);

    let _ = SCREEN
        .str_writer(50, 50, screen::Color::WHITE)
        .write_fmt(format_args!(":(\ntntos2 {info}"));

    halt()
}
