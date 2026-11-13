use core::ops::Range;

pub const BYTES_PER_FRAME: usize = 4096 * 512;

type Frame = [u8; BYTES_PER_FRAME];

pub struct Frames {
    frameid: usize,
    length: usize,
}
impl Frames {
    pub fn as_slice(&self) -> &[Frame] {
        unsafe {
            core::slice::from_raw_parts(core::ptr::null::<Frame>().add(self.frameid), self.length)
        }
    }
    pub fn as_mut_slice(&mut self) -> &mut [Frame] {
        unsafe {
            core::slice::from_raw_parts_mut(
                core::ptr::null_mut::<Frame>().add(self.frameid),
                self.length,
            )
        }
    }
}

pub struct MemoryManager {
    alloc_map: [u8; MemoryManager::FRAME_COUNT / 8],
    pub range: Range<usize>,
}
impl MemoryManager {
    pub const ZEROED: MemoryManager = MemoryManager {
        alloc_map: [0; MemoryManager::FRAME_COUNT / 8],
        range: 0..0,
    };

    pub const MAX_PHY_MEM_BYTES: usize = 1024 * 1024 * 1024 * 128; // 1GiB
    pub const FRAME_COUNT: usize = MemoryManager::MAX_PHY_MEM_BYTES / BYTES_PER_FRAME;

    fn is_allocated(&self, frame: usize) -> bool {
        let arr_idx = frame >> 3;
        let bit_offset = frame & 0b111;
        if let Some(line) = self.alloc_map.get(arr_idx) {
            line & (1 << bit_offset) != 0
        } else {
            true // とりあえず使用中ってことにする
        }
    }
    fn set_allocated(&mut self, frame: usize, allocated: bool) {
        let arr_idx = frame >> 3;
        let bit_offset = frame & 0b111;
        let bit = 1 << bit_offset;
        let Some(line) = self.alloc_map.get_mut(arr_idx) else {
            return;
        };
        if allocated {
            *line |= bit;
        } else {
            *line &= !bit;
        }
    }

    pub fn mark_allocated(&mut self, start: usize, len: usize) {
        for i in 0..len {
            self.set_allocated(start + i, true);
        }
    }
    pub fn allocate(&mut self, len: usize) -> Option<Frames> {
        let mut curr_frame = self.range.start;

        'mainloop: loop {
            for i in 0..len {
                if self.is_allocated(curr_frame + i) {
                    curr_frame += i + 1;
                    continue 'mainloop;
                }
            }
            if self.range.contains(&(curr_frame + len - 1)) {
                self.mark_allocated(curr_frame, len);
                break Some(Frames {
                    frameid: curr_frame,
                    length: len,
                });
            } else {
                break None;
            }
        }
    }
    pub fn free(&mut self, frames: Frames) -> Result<(), ()> {
        for i in frames.frameid..(frames.frameid + frames.length) {
            if self.is_allocated(i) {
                self.set_allocated(i, false);
            } else {
                return Err(());
            }
        }
        Ok(())
    }
}
