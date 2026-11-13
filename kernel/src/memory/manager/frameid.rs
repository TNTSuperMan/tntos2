#[derive(Clone, Copy)]
pub struct FrameId(pub(super) usize);
impl FrameId {
    pub const BYTES_PER_FRAME: usize = 4096 * 512;

    pub fn id(&self) -> usize {
        self.0
    }
    pub fn frame(&self) -> *mut () {
        unsafe {
            core::ptr::null_mut::<[u8; FrameId::BYTES_PER_FRAME]>()
                .add(self.id())
                .cast()
        }
    }
}
