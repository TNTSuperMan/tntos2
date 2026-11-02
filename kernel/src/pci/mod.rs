mod device;
mod scan;

use core::ops::ControlFlow;

use crate::{pci::device::Device, utils::vecarr::VecArr};

pub static DEVICES: spin::Once<VecArr<Device, 32>> = spin::Once::new();

pub fn init_pci() {
    DEVICES.try_call_once::<_, core::convert::Infallible>(|| {
        let mut devices = VecArr::new(|_| Device::new(0, 0, 0));

        let _ = scan::scan_all_bus(|device| {
            if devices.push(device).is_err() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });

        Ok(devices)
    });
}
