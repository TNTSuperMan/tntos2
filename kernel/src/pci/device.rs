use crate::asm::{io_in, io_out};

const IOPORT_CONFIG_ADDR: u16 = 0xCF8;
const IOPORT_CONFIG_DATA: u16 = 0x0CFC;

#[derive(Clone, Copy)]
pub struct ClassCode {
    base: u8,
    sub: u8,
    interface: u8,
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
impl ClassCode {
    pub fn base(&self) -> u8 {
        self.base
    }
    pub fn sub(&self) -> u8 {
        self.sub
    }
    pub fn interface(&self) -> u8 {
        self.interface
    }
}

#[derive(Clone, Copy)]
pub struct Device {
    bus: u8,
    device: u8,
    func: u8,
}
impl core::fmt::Debug for Device {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Device { bus, device, func } = self;

        let vendor = self.vendor_id();
        let class = self.class_code();
        let head = self.header_type();

        write!(
            f,
            "{bus:>02x}:{device:>02x}.{func}: vend {vendor:>04x}, class {class:?}, head {head:>02x}"
        )
    }
}
impl Device {
    pub fn new(bus: u8, device: u8, func: u8) -> Self {
        Self { bus, device, func }
    }

    fn write_addr(&self, reg: u8) {
        io_out(
            IOPORT_CONFIG_ADDR,
            1 << 31
                | ((self.bus as u32) << 16)
                | ((self.device as u32 & 31) << 11)
                | ((self.func as u32 & 7) << 8)
                | ((reg as u32) & 0xfc),
        );
    }
    pub fn read(&self, reg: u8) -> u32 {
        self.write_addr(reg);
        io_in(IOPORT_CONFIG_DATA)
    }
    pub fn write(&self, reg: u8, data: u32) {
        self.write_addr(reg);
        io_out(IOPORT_CONFIG_DATA, data);
    }

    pub fn bus(&self) -> u8 {
        self.bus
    }
    pub fn device(&self) -> u8 {
        self.device
    }
    pub fn func(&self) -> u8 {
        self.func
    }

    pub fn vendor_id(&self) -> u16 {
        self.read(0x00) as u16
    }
    pub fn device_id(&self) -> u16 {
        (self.read(0x00) >> 16) as u16
    }
    pub fn class_code(&self) -> ClassCode {
        let reg = self.read(0x08);
        ClassCode {
            base: (reg >> 24) as u8,
            sub: (reg >> 16) as u8,
            interface: (reg >> 8) as u8,
        }
    }
    pub fn header_type(&self) -> u8 {
        (self.read(0x0c) >> 16) as u8
    }

    pub fn is_single_func(&self) -> bool {
        self.header_type() & 0x80 == 0
    }
    pub fn bar(&self, bar_index: u8) -> Option<u64> {
        if bar_index >= 6 {
            return None;
        }

        let bar_lo = self.read(0x10 + 4 * bar_index);

        if (bar_lo & 4) == 0 {
            Some(bar_lo as u64)
        } else {
            if bar_index >= 5 {
                return None;
            }
            let bar_hi = self.read(0x10 + 4 * (bar_index + 1));
            Some(bar_lo as u64 | ((bar_hi as u64) << 32))
        }
    }
}
