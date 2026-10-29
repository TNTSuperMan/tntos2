use uefi::{
    boot::{self, MemoryType, ScopedProtocol},
    proto::{loaded_image::LoadedImage, media::block::BlockIO},
};

fn open_blockio() -> uefi::Result<ScopedProtocol<BlockIO>> {
    let loaded_image = unsafe {
        boot::open_protocol::<LoadedImage>(
            boot::OpenProtocolParams {
                handle: boot::image_handle(),
                agent: boot::image_handle(),
                controller: None,
            },
            boot::OpenProtocolAttributes::GetProtocol,
        )?
    };

    let blockio = unsafe {
        boot::open_protocol(
            boot::OpenProtocolParams {
                handle: loaded_image.device().unwrap(),
                agent: boot::image_handle(),
                controller: None,
            },
            boot::OpenProtocolAttributes::GetProtocol,
        )?
    };

    Ok(blockio)
}

fn read_blocks(
    block_io: &mut ScopedProtocol<BlockIO>,
    media_id: u32,
    bytes: usize,
) -> uefi::Result<&'static [u8]> {
    let buf = boot::allocate_pool(MemoryType::LOADER_DATA, bytes)?;

    {
        let mut_slice = unsafe { core::slice::from_raw_parts_mut(buf.as_ptr(), bytes) };
        block_io.read_blocks(media_id, 0, mut_slice)?;
    }

    Ok(unsafe { core::slice::from_raw_parts(buf.as_ptr(), bytes) })
}

pub fn read_volume() -> uefi::Result<&'static [u8]> {
    let mut block_io = open_blockio()?;
    let (volume_bytes, media_id) = {
        let media = block_io.media();
        (
            (media.block_size() as u64 * (media.last_block() + 1)).min(32 * 1024 * 1024),
            media.media_id(),
        )
    };
    read_blocks(&mut block_io, media_id, volume_bytes as usize)
}
