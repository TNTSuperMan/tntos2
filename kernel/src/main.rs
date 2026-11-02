#![no_main]
#![no_std]

use crate::{
    pci::scan_all_bus,
    screen::{Color, SCREEN},
};
use core::{fmt::Write, ops::ControlFlow};

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

    unsafe {
        screen::assume_init_screen(frame_buffer);
        uefi::boot::set_image_handle(image);
        uefi::table::set_system_table(table.as_ptr());
    }

    let mut screen_lock = SCREEN.lock();
    let screen = screen_lock.as_mut().unwrap();

    screen.draw_rect(0, 0, screen.width(), screen.height(), Color::WHITE);
    let mut writer = screen.str_writer(100, 100, Color::BLACK);
    writer.write_str("Hello, World!\n").unwrap();

    let _ = scan_all_bus::<()>(&mut |dev| {
        writer.write_fmt(format_args!("{dev:?}\n")).unwrap();
        ControlFlow::Continue(())
    });

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
