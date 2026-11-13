#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(allocator_ext)]

extern crate alloc;

use crate::{
    pci::DEVICES,
    screen::{Color, SCREEN},
    xhc::poll_xhc,
};

mod asm;
mod console;
mod interrupt;
mod memory;
mod panic;
mod pci;
mod screen;
mod utils;
pub mod xhc;

// mainに直接置いてるとキモいので然るべきときが来たら別の場所に移動してください
#[repr(align(16))]
struct Stack([u8; 1048576]);
static mut KERNEL_STACK: Stack = Stack([0; 1048576]);

#[unsafe(no_mangle)]
extern "sysv64" fn _start(options: *const shared::KernelEntrypointOptions) -> ! {
    let shared::KernelEntrypointOptions {
        frame_buffer,
        memory_map,
        image,
        table,
        ..
    } = unsafe { options.read() };

    unsafe {
        screen::assume_init_screen(frame_buffer);
        uefi::boot::set_image_handle(image);
        uefi::table::set_system_table(table.as_ptr());
        memory::uefi_mmap::init(memory_map);
    }

    {
        let mut screen = SCREEN.lock();
        let width = screen.width();
        let height = screen.height();

        screen.draw_rect(0, 0, width, height, Color::WHITE);
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
    pci::init();
    xhc::init();
    interrupt::init();
    memory::init();

    let (width, height, format) = {
        let screen = SCREEN.lock();
        (screen.width(), screen.height(), screen.format())
    };

    println!("screen: {width}x{height} {format:?}");

    for device in DEVICES.get().unwrap().iter() {
        println!("{device:?}");
    }

    loop {
        poll_xhc();
        core::hint::spin_loop();
    }
}
