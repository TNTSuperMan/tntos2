use core::ops::Range;

#[derive(Clone, Copy)]
pub struct FrameId(usize);
impl FrameId {
    pub const BYTES_PER_FRAME: usize = 4096;

    pub fn id(&self) -> usize {
        self.0
    }
    pub fn frame(&self) -> *mut () {
        unsafe { core::ptr::null_mut::<[u8; 4096]>().add(self.id()).cast() }
    }
}

pub struct MemoryManager {
    alloc_map: [u8; MemoryManager::FRAME_COUNT / 8], // 4MiBになる
    pub range: Range<usize>,
}
impl MemoryManager {
    pub const MAX_PHY_MEM_BYTES: usize = 1024 * 1024 * 1024 * 128; // 128GiB
    pub const FRAME_COUNT: usize = MemoryManager::MAX_PHY_MEM_BYTES / FrameId::BYTES_PER_FRAME;

    fn is_allocated(&self, frame: usize) -> bool {
        let arr_idx = frame >> 3;
        let bit_offset = frame & 0b111;
        self.alloc_map[arr_idx] & (1 << bit_offset) != 0
    }
    fn set_allocated(&mut self, frame: usize, allocated: bool) {
        let arr_idx = frame >> 3;
        let bit_offset = frame & 0b111;
        let bit = 1 << bit_offset;
        if allocated {
            self.alloc_map[arr_idx] |= bit;
        } else {
            self.alloc_map[arr_idx] &= !bit;
        }
    }

    pub fn mark_allocated(&mut self, start: FrameId, len: usize) {
        for i in 0..len {
            self.set_allocated(start.id() + i, true);
        }
    }
    pub fn allocate(&mut self, len: usize) -> Option<FrameId> {
        let mut curr_frame = self.range.start;

        'mainloop: loop {
            for i in 0..len {
                if self.is_allocated(curr_frame + i) {
                    curr_frame += i + 1;
                    continue 'mainloop;
                }
            }
            if self.range.contains(&(curr_frame + len - 1)) {
                self.mark_allocated(FrameId(curr_frame), len);
                break Some(FrameId(curr_frame));
            } else {
                break None;
            }
        }
    }
    pub fn free(&mut self, start: FrameId, len: usize) -> Result<(), ()> {
        for i in start.0..(start.0 + len) {
            if self.is_allocated(i) {
                self.set_allocated(i, false);
            } else {
                return Err(());
            }
        }
        Ok(())
    }
}
