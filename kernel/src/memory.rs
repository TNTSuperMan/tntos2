use uefi::{
    boot::MemoryType,
    mem::memory_map::{MemoryMapMeta, MemoryMapRef},
};

pub static MEMORY_MAP: spin::Once<MemoryMapRef<'static>> = spin::Once::new();

// SAFETY: 呼び出し元は、引数memmapのptr/lenの組みが正しいことを保証する必要がある
pub unsafe fn init(memmap: shared::MemoryMap) {
    MEMORY_MAP.call_once(|| {
        MemoryMapRef::new(
            unsafe { core::slice::from_raw_parts::<'static>(memmap.ptr, memmap.len) },
            MemoryMapMeta {
                map_size: memmap.map_size,
                desc_size: memmap.desc_size,
                map_key: memmap.map_key,
                desc_version: memmap.desc_version,
            },
        )
        .expect("corrupt memorymap received from loader")
    });
}

pub fn is_available(typ: MemoryType) -> bool {
    match typ {
        MemoryType::BOOT_SERVICES_CODE
        | MemoryType::BOOT_SERVICES_DATA
        | MemoryType::CONVENTIONAL => true,
        _ => false,
    }
}
