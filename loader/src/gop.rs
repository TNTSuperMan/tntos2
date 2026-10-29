use uefi::{
    Identify, boot,
    proto::console::gop::{GraphicsOutput, PixelFormat},
};

pub fn open_gop() -> uefi::Result<boot::ScopedProtocol<GraphicsOutput>> {
    let gop_handles =
        boot::locate_handle_buffer(boot::SearchType::ByProtocol(&GraphicsOutput::GUID))?;

    let gop = unsafe {
        boot::open_protocol::<GraphicsOutput>(
            boot::OpenProtocolParams {
                handle: gop_handles[0],
                agent: boot::image_handle(),
                controller: None,
            },
            boot::OpenProtocolAttributes::GetProtocol,
        )
    }?;

    Ok(gop)
}

pub fn into_frame_buffer_options(
    gop: boot::ScopedProtocol<GraphicsOutput>,
) -> shared::FrameBufferOptions {
    let mut gop = core::mem::ManuallyDrop::new(gop);
    let gop = gop.get_mut().unwrap();
    shared::FrameBufferOptions {
        frame_buffer: gop.frame_buffer().as_mut_ptr(),
        frame_buffer_size: gop.frame_buffer().size(),
        horizontal_resolution: gop.current_mode_info().resolution().0 as u32,
        vertical_resolution: gop.current_mode_info().resolution().1 as u32,
        pixels_per_scanline: gop.current_mode_info().stride() as u32,
        pixel_format: match gop.current_mode_info().pixel_format() {
            PixelFormat::Rgb => shared::PixelFormat::Rgb,
            PixelFormat::Bgr => shared::PixelFormat::Bgr,
            _ => unimplemented!(),
        },
    }
}
