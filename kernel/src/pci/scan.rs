use core::ops::ControlFlow;

use crate::pci::device::Device;

pub fn scan_all_bus<T>(mut f: impl FnMut(Device) -> ControlFlow<T, ()>) -> Option<T> {
    if Device::new(0, 0, 0).is_single_func() {
        return scan_bus(&mut f, 0).break_value();
    }

    for func in 0..8 {
        if Device::new(0, 0, func).vendor_id() != 0xFFFF {
            if let Some(b) = scan_bus(&mut f, func).break_value() {
                return Some(b);
            }
        }
    }

    None
}

fn scan_bus<T>(f: &mut impl FnMut(Device) -> ControlFlow<T, ()>, bus: u8) -> ControlFlow<T, ()> {
    for device in 0..32 {
        if Device::new(bus, device, 0).vendor_id() != 0xFFFF {
            scan_device(f, bus, device)?;
        }
    }

    ControlFlow::Continue(())
}

fn scan_device<T>(
    f: &mut impl FnMut(Device) -> ControlFlow<T, ()>,
    bus: u8,
    device: u8,
) -> ControlFlow<T, ()> {
    let zero_device = Device::new(bus, device, 0);
    scan_func(f, zero_device)?;

    if !zero_device.is_single_func() {
        for func in 1..8 {
            let device = Device::new(bus, device, func);
            if device.vendor_id() != 0xFFFF {
                scan_func(f, device)?;
            }
        }
    }

    ControlFlow::Continue(())
}

fn scan_func<T>(
    f: &mut impl FnMut(Device) -> ControlFlow<T, ()>,
    device: Device,
) -> ControlFlow<T, ()> {
    f(device)?;

    let class = device.class_code();

    if class.base() == 0x06 && class.sub() == 0x04 {
        // standard PCI-PCI bridge
        let bus_nums = device.read(0x18);
        let secondary_bus = (bus_nums >> 8) as u8;
        scan_bus(f, secondary_bus)
    } else {
        ControlFlow::Continue(())
    }
}
