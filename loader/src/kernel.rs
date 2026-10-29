use crate::file::FileBuffer;
use core::ops::Range;
use uefi::{
    boot::{self, AllocateType, MemoryType},
    proto::media::file::{Directory, File, FileAttribute, FileMode},
};

struct Elf64Ehdr<'a> {
    slice: &'a [u8],
}
#[allow(unused)]
impl<'a> Elf64Ehdr<'a> {
    fn half(&self, start: usize) -> u16 {
        u16::from_le_bytes(self.slice[start..(start + 2)].try_into().unwrap())
    }
    fn word(&self, start: usize) -> u32 {
        u32::from_le_bytes(self.slice[start..(start + 4)].try_into().unwrap())
    }
    fn off(&self, start: usize) -> u64 {
        u64::from_le_bytes(self.slice[start..(start + 8)].try_into().unwrap())
    }

    pub fn ident(&self) -> [u8; 16] {
        self.slice[0..16].try_into().unwrap()
    }
    pub fn typ(&self) -> u16 {
        self.half(16)
    }
    pub fn machine(&self) -> u16 {
        self.half(18)
    }
    pub fn version(&self) -> u32 {
        self.word(20)
    }
    pub fn entry(&self) -> u64 {
        self.off(24)
    }
    pub fn phoff(&self) -> u64 {
        self.off(32)
    }
    pub fn shoff(&self) -> u64 {
        self.off(40)
    }
    pub fn flags(&self) -> u32 {
        self.word(48)
    }
    pub fn ehsize(&self) -> u16 {
        self.half(52)
    }
    pub fn phentsize(&self) -> u16 {
        self.half(54)
    }
    pub fn phnum(&self) -> u16 {
        self.half(56)
    }
    pub fn shentsize(&self) -> u16 {
        self.half(58)
    }
    pub fn shnum(&self) -> u16 {
        self.half(60)
    }
    pub fn shstrndx(&self) -> u16 {
        self.half(62)
    }

    pub fn phdrs(&self) -> PhdrIter<'a, '_> {
        PhdrIter {
            ehdr: self,
            count: 0,
        }
    }
}

struct PhdrIter<'a, 'b> {
    ehdr: &'b Elf64Ehdr<'a>,
    count: usize,
}
impl<'a, 'b> Iterator for PhdrIter<'a, 'b> {
    type Item = Elf64Phdr<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.ehdr.phnum() as usize {
            None
        } else {
            let range = (self.ehdr.phoff() as usize + self.count * Elf64Phdr::SIZE)
                ..(self.ehdr.phoff() as usize + (self.count + 1) * Elf64Phdr::SIZE);
            self.count += 1;
            Some(Elf64Phdr {
                slice: &self.ehdr.slice[range],
            })
        }
    }
}

struct Elf64Phdr<'a> {
    slice: &'a [u8],
}
#[allow(unused)]
impl<'a> Elf64Phdr<'a> {
    const SIZE: usize = 56;
    fn word(&self, start: usize) -> u32 {
        u32::from_le_bytes(self.slice[start..(start + 4)].try_into().unwrap())
    }
    fn off(&self, start: usize) -> u64 {
        u64::from_le_bytes(self.slice[start..(start + 8)].try_into().unwrap())
    }

    pub fn typ(&self) -> u32 {
        self.word(0)
    }
    pub fn flags(&self) -> u32 {
        self.word(4)
    }
    pub fn offset(&self) -> u64 {
        self.off(8)
    }
    pub fn vaddr(&self) -> u64 {
        self.off(16)
    }
    pub fn paddr(&self) -> u64 {
        self.off(24)
    }
    pub fn filesz(&self) -> u64 {
        self.off(32)
    }
    pub fn memsz(&self) -> u64 {
        self.off(40)
    }
    pub fn align(&self) -> u64 {
        self.off(48)
    }
}

fn calc_load_address_range(elf: &[u8]) -> Range<u64> {
    let ehdr = Elf64Ehdr { slice: elf };

    let mut range = (u64::MAX)..(u64::MIN);

    for phdr in ehdr.phdrs() {
        if phdr.typ() != 1
        /* PT_LAOD */
        {
            continue;
        }
        range.start = range.start.min(phdr.vaddr());
        range.end = range.end.max(phdr.vaddr() + phdr.memsz());
    }

    range
}

unsafe fn copy_load_segments(elf: &[u8]) {
    let ehdr = Elf64Ehdr { slice: elf };

    for phdr in ehdr.phdrs() {
        if phdr.typ() != 1
        /* PT_LOAD */
        {
            continue;
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                elf.as_ptr().add(phdr.offset() as usize),
                core::ptr::null_mut::<u8>().add(phdr.vaddr() as usize),
                phdr.filesz() as usize,
            );
        }

        let remain_bytes = phdr.memsz() - phdr.filesz();
        unsafe {
            core::ptr::write_bytes(
                core::ptr::null_mut::<u8>()
                    .add(phdr.vaddr() as usize)
                    .add(phdr.filesz() as usize),
                0,
                remain_bytes as usize,
            );
        }
    }
}

pub fn load_kernel(
    rootdir: &mut Directory,
) -> Result<impl FnOnce(shared::KernelEntrypointOptions) -> core::convert::Infallible, &'static str>
{
    let kernel_file = rootdir
        .open(
            uefi::CStr16::from_str_with_buf("\\kernel.elf", &mut [0; 12])
                .map_err(|_| "failed to load kernel path")?,
            FileMode::Read,
            FileAttribute::empty(),
        )
        .map_err(|_| "failed to open kernel")?;
    let kernel_data = FileBuffer::new(kernel_file)?;

    let kernel_addr_range = calc_load_address_range(kernel_data.as_slice());
    let num_pages = kernel_addr_range.clone().count().div_ceil(0x1000);

    let entrypoint = Elf64Ehdr {
        slice: kernel_data.as_slice(),
    }
    .entry();

    log::debug!(
        "kernel file loaded. range: {:>016X} ~ {:>016X} ({num_pages} pages), entry: {entrypoint:>016X}",
        kernel_addr_range.start,
        kernel_addr_range.end
    );

    boot::allocate_pages(
        AllocateType::Address(kernel_addr_range.start),
        MemoryType::LOADER_DATA,
        num_pages as usize,
    )
    .map_err(|_| "failed to alloc kernel page")?;

    unsafe { copy_load_segments(kernel_data.as_slice()) };

    Ok(move |options| {
        // MEMO: エントリポイントの関数が終了しないため、optionsはdropしない
        // そのためこのアドレス渡しによるUse-after-Freeは起こり得ない
        // 関数呼び出しの安全性についてはしらん
        let fn_ptr: shared::KernelEntryPoint = unsafe { core::mem::transmute(entrypoint) };
        (fn_ptr)(core::ptr::addr_of!(options));
    })
}
