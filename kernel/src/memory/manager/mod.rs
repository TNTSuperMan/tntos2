mod init;
mod manager;

pub(super) use init::init;

pub use manager::{BYTES_PER_FRAME, Frames, MemoryManager};

pub static MEMORY_MANAGER: spin::Mutex<MemoryManager> = spin::Mutex::new(MemoryManager::ZEROED);
