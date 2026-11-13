mod frameid;
mod init;
mod manager;

pub(super) use init::init;

pub use frameid::FrameId;
pub use manager::MemoryManager;

pub static MEMORY_MANAGER: spin::Mutex<MemoryManager> = spin::Mutex::new(MemoryManager::ZEROED);
