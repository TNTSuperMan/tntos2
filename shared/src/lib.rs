#![no_std]

pub type KernelEntryPoint = extern "sysv64" fn(*const KernelEntrypointOptions) -> !;

#[repr(C)]
pub struct KernelEntrypointOptions {
    pub frame_buffer: FrameBufferOptions,
    pub memory_map: MemoryMap,
    pub volume_image: VolumeImage,

    pub image: uefi::Handle,
    pub table: core::ptr::NonNull<uefi_raw::table::system::SystemTable>,
}
impl core::fmt::Debug for KernelEntrypointOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "KEntryOpt
frame: #{:>016X} {}x{}
volume: #{:>016X} {}
image: #{:>016X}
table: #{:>016X}",
            self.frame_buffer.frame_buffer.addr(),
            self.frame_buffer.horizontal_resolution,
            self.frame_buffer.vertical_resolution,
            self.volume_image.ptr.addr(),
            self.volume_image.len,
            self.image.as_ptr().addr(),
            self.table.addr(),
        )
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct FrameBufferOptions {
    pub frame_buffer: *mut u8,
    pub frame_buffer_size: usize,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixels_per_scanline: u32,
    pub pixel_format: PixelFormat,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum PixelFormat {
    Rgb,
    Bgr,
}

#[repr(C)]
pub struct MemoryMap {
    pub ptr: *const u8,
    pub len: usize,

    pub map_size: usize,
    pub desc_size: usize,
    pub map_key: uefi::mem::memory_map::MemoryMapKey,
    pub desc_version: u32,
}

#[repr(C)]
pub struct VolumeImage {
    pub ptr: *const u8,
    pub len: usize,
}
