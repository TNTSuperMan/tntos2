#![no_main]
#![no_std]

mod blockio;
mod file;
mod gop;
mod kernel;
mod rootdir;

use log::{debug, error, info};
use uefi::{Status, boot, mem::memory_map::MemoryMap};

use crate::gop::into_frame_buffer_options;

// TODO:
// - More good error
// - Kernel

fn main() -> Result<core::convert::Infallible, &'static str> {
    info!("loading tntos2...");

    let memory_map =
        boot::memory_map(boot::MemoryType::LOADER_DATA).map_err(|_| "failed to load memmap")?;
    debug!("memmap loaded");

    let mut rootdir = rootdir::open_root_dir().map_err(|_| "failed to open rootdir")?;
    debug!("rootdir opened");

    let gop = gop::open_gop().map_err(|_| "failed to open gop")?;
    debug!("gop opened");

    let entry = kernel::load_kernel(&mut rootdir)?;
    debug!("kernel loaded");

    info!("loading volume...");
    let volume_image = blockio::read_volume().map_err(|_| "failed to read volume")?;
    info!("volume loaded");

    let options = shared::KernelEntrypointOptions {
        frame_buffer: into_frame_buffer_options(gop),
        memory_map: shared::MemoryMap {
            // memory_mapはこの関数の返りまで生存する entryは値を返さない(Infallible) そのためUse-After-Freeは起こり得ない
            ptr: memory_map.buffer().as_ptr(),
            len: memory_map.buffer().len(),

            map_size: memory_map.meta().map_size,
            desc_size: memory_map.meta().desc_size,
            map_key: memory_map.meta().map_key,
            desc_version: memory_map.meta().desc_version,
        },
        volume_image: shared::VolumeImage {
            ptr: volume_image.as_ptr(),
            len: volume_image.len(),
        },
        image: uefi::boot::image_handle(),
        table: uefi::table::system_table_raw().unwrap(),
    };

    unsafe {
        let _ = boot::exit_boot_services(None);
    }

    Ok(entry(options))
}

#[uefi::entry]
fn uefi_main() -> Status {
    uefi::helpers::init().unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    });

    match main() {
        Err(err) => {
            error!("boot error: {err}");
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    }
}

#[panic_handler]
fn on_panic(info: &core::panic::PanicInfo) -> ! {
    uefi::println!("[PANIC] {info}");
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
