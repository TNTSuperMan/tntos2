use crate::{
    pci::{
        ClassCode, Device,
        io::{is_single_func_device, read_bus_numbers, read_header_type, read_vendor_id},
    },
    utils::vecarr::VecArr,
};

#[derive(Debug)]
pub enum ScanError {
    VecArrOverflow,
}

pub type Devices = VecArr<Device, 32>;

pub fn scan_all_bus() -> Result<Devices, ScanError> {
    let mut devices = Devices::new();

    if is_single_func_device(read_header_type(0, 0, 0)) {
        scan_bus(&mut devices, 0).map(|_| devices)
    } else {
        for func in 0..8 {
            if read_vendor_id(0, 0, func) != 0xFFFF {
                scan_bus(&mut devices, func)?;
            }
        }
        Ok(devices)
    }
}

fn scan_bus(devices: &mut Devices, bus: u8) -> Result<(), ScanError> {
    for device in 0..32 {
        if read_vendor_id(bus, device, 0) != 0xFFFF {
            scan_device(devices, bus, device)?;
        }
    }
    Ok(())
}

fn scan_device(devices: &mut Devices, bus: u8, device: u8) -> Result<(), ScanError> {
    scan_func(devices, bus, device, 0)?;
    if !is_single_func_device(read_header_type(bus, device, 0)) {
        for func in 1..8 {
            if read_vendor_id(bus, device, func) != 0xFFFF {
                scan_func(devices, bus, device, func)?;
            }
        }
    }
    Ok(())
}

fn scan_func(devices: &mut Devices, bus: u8, device: u8, func: u8) -> Result<(), ScanError> {
    let class_code = ClassCode::read(bus, device, func);
    let header_type = read_header_type(bus, device, func);
    let dev = Device {
        bus,
        device,
        func,
        header_type,
        class_code,
    };
    devices.push(dev).map_err(|_| ScanError::VecArrOverflow)?;

    if class_code.base == 0x06 && class_code.sub == 0x04 {
        // standard PCI-PCI bridge
        let bus_nums = read_bus_numbers(bus, device, func);
        let secondary_bus = (bus_nums >> 8) as u8;
        scan_bus(devices, secondary_bus)
    } else {
        Ok(())
    }
}
