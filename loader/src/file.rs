use core::ptr::NonNull;
use uefi::{
    boot::{self, MemoryType},
    proto::media::file::{File, FileHandle, FileInfo},
};

const INFO_SIZE: usize = 80 + 2 * 12;

pub struct FileBuffer {
    buffer: NonNull<u8>,
    length: usize,
}
impl FileBuffer {
    pub fn new(mut file: FileHandle) -> Result<Self, &'static str> {
        let mut info_buf = [0; INFO_SIZE];
        let size = {
            let info = file
                .get_info::<FileInfo>(&mut info_buf)
                .map_err(|_| "failed to get file info")?;

            info.file_size() as usize
        };

        let file_buf_ptr = boot::allocate_pool(MemoryType::LOADER_DATA, size)
            .map_err(|_| "failed to alloc file buf")?;

        let file_buf_slice =
            unsafe { core::slice::from_raw_parts_mut(file_buf_ptr.as_ptr(), size) };
        file.into_regular_file()
            .unwrap()
            .read(file_buf_slice)
            .map_err(|_| "failed to read file")?;

        Ok(FileBuffer {
            buffer: file_buf_ptr,
            length: size,
        })
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.buffer.as_ptr(), self.length) }
    }
}
impl Drop for FileBuffer {
    fn drop(&mut self) {
        unsafe { boot::free_pool(self.buffer) }.unwrap();
    }
}
