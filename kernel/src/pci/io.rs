use crate::asm::{io_in, io_out};

fn make_addr(bus: u8, device: u8, func: u8, reg: u8) -> u32 {
    1 << 31
        | ((bus as u32) << 16)
        | ((device as u32 & 31) << 11)
        | ((func as u32 & 7) << 8)
        | ((reg as u32) & 0xfc)
}

const IOPORT_CONFIG_ADDRESS: u16 = 0xCF8;
const IOPORT_CONFIG_DATA: u16 = 0x0CFC;

pub fn write_addr(bus: u8, device: u8, func: u8, reg: u8) {
    io_out(IOPORT_CONFIG_ADDRESS, make_addr(bus, device, func, reg));
}
pub fn write_data(data: u32) {
    io_out(IOPORT_CONFIG_DATA, data);
}
pub fn read_data() -> u32 {
    io_in(IOPORT_CONFIG_DATA)
}

pub fn read_vendor_id(bus: u8, device: u8, func: u8) -> u16 {
    write_addr(bus, device, func, 0x00);
    read_data() as u16
}
pub fn read_device_id(bus: u8, device: u8, func: u8) -> u16 {
    write_addr(bus, device, func, 0x00);
    (read_data() >> 16) as u16
}
pub fn read_header_type(bus: u8, device: u8, func: u8) -> u8 {
    write_addr(bus, device, func, 0x0c);
    (read_data() >> 16) as u8
}
pub fn read_bus_numbers(bus: u8, device: u8, func: u8) -> u32 {
    write_addr(bus, device, func, 0x18);
    read_data()
}

pub fn is_single_func_device(header_type: u8) -> bool {
    (header_type & 0x80) == 0
}
