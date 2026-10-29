use uefi::{boot, proto::media::file::Directory};

pub fn open_root_dir() -> uefi::Result<Directory> {
    let loaded_image = unsafe {
        boot::open_protocol::<uefi::proto::loaded_image::LoadedImage>(
            boot::OpenProtocolParams {
                handle: boot::image_handle(),
                agent: boot::image_handle(),
                controller: None,
            },
            boot::OpenProtocolAttributes::GetProtocol, // EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOLはどうすれば？
        )
    }?;

    let mut fs = unsafe {
        boot::open_protocol::<uefi::proto::media::fs::SimpleFileSystem>(
            boot::OpenProtocolParams {
                handle: loaded_image.device().unwrap(),
                agent: boot::image_handle(),
                controller: None,
            },
            boot::OpenProtocolAttributes::GetProtocol,
        )
    }?;

    fs.open_volume()
}
