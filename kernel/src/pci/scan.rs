use core::ops::ControlFlow;

use crate::pci::{
    ClassCode, Device,
    io::{is_single_func_device, read_bus_numbers, read_header_type, read_vendor_id},
};

pub fn scan_all_bus<T>(mut f: impl FnMut(Device) -> ControlFlow<T, ()>) -> Option<T> {
    if is_single_func_device(read_header_type(0, 0, 0)) {
        return scan_bus(&mut f, 0);
    }

    for func in 0..8 {
        if read_vendor_id(0, 0, func) != 0xFFFF {
            if let Some(result) = scan_bus(&mut f, func) {
                return Some(result);
            }
        }
    }

    None
}

fn scan_bus<T>(f: &mut impl FnMut(Device) -> ControlFlow<T, ()>, bus: u8) -> Option<T> {
    for device in 0..32 {
        if read_vendor_id(bus, device, 0) != 0xFFFF {
            if let Some(result) = scan_device(f, bus, device) {
                return Some(result);
            }
        }
    }

    None
}

fn scan_device<T>(
    f: &mut impl FnMut(Device) -> ControlFlow<T, ()>,
    bus: u8,
    device: u8,
) -> Option<T> {
    if let Some(result) = scan_func(f, bus, device, 0) {
        return Some(result);
    }

    if !is_single_func_device(read_header_type(bus, device, 0)) {
        for func in 1..8 {
            if read_vendor_id(bus, device, func) != 0xFFFF {
                if let Some(result) = scan_func(f, bus, device, func) {
                    return Some(result);
                }
            }
        }
    }

    None
}

fn scan_func<T>(
    f: &mut impl FnMut(Device) -> ControlFlow<T, ()>,
    bus: u8,
    device: u8,
    func: u8,
) -> Option<T> {
    let class_code = ClassCode::read(bus, device, func);
    let header_type = read_header_type(bus, device, func);
    let dev = Device {
        bus,
        device,
        func,
        header_type,
        class_code,
    };

    match f(dev) {
        ControlFlow::Continue(()) => {}
        ControlFlow::Break(value) => return Some(value),
    }

    if class_code.base == 0x06 && class_code.sub == 0x04 {
        // standard PCI-PCI bridge
        let bus_nums = read_bus_numbers(bus, device, func);
        let secondary_bus = (bus_nums >> 8) as u8;
        scan_bus(f, secondary_bus)
    } else {
        None
    }
}
