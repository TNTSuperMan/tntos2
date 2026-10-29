use crate::pci::io::{read_data, write_addr};

mod io;
mod scan;

pub use scan::*;

#[derive(Default, Clone, Copy)]
pub struct ClassCode {
    pub base: u8,
    pub sub: u8,
    pub interface: u8,
}
impl ClassCode {
    fn read(bus: u8, device: u8, func: u8) -> Self {
        write_addr(bus, device, func, 0x08);
        let reg = read_data();
        ClassCode {
            base: (reg >> 24) as u8,
            sub: (reg >> 16) as u8,
            interface: (reg >> 8) as u8,
        }
    }
}
impl core::fmt::Debug for ClassCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ClassCode {
            base,
            sub,
            interface,
        } = self;
        write!(f, "{base:>02X} {sub:>02X} {interface:>02X}")
    }
}

#[derive(Default)]
pub struct Device {
    pub bus: u8,
    pub device: u8,
    pub func: u8,
    pub header_type: u8,
    pub class_code: ClassCode,
}
impl core::fmt::Debug for Device {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Device {
            bus,
            device,
            func,
            header_type,
            class_code,
            ..
        } = self;
        write!(
            f,
            "{bus:>02X}:{device:>02X}.{func} head {header_type:>02X}, class {class_code:?}"
        )
    }
}
