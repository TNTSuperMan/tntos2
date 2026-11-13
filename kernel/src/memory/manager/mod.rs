mod init;
mod manager;

pub(super) use init::init;

pub use manager::{Frames, MemoryManager};

pub static MEMORY_MANAGER: spin::Mutex<MemoryManager> = spin::Mutex::new(MemoryManager::ZEROED);
