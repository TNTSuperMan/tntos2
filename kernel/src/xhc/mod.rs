use crate::pci::{DEVICES, Device};

mod control;
mod registers;

fn switch_ehci_xhci(xhc_device: &Device) {
    if DEVICES.get().unwrap().iter().any(|device| {
        // check intel EHC
        let class = device.class_code();
        (class.base() == 0x0C && class.sub() == 0x03 && class.interface() == 0x30)
            && device.vendor_id() == 0x8086
    }) {
        let superspeed_ports = xhc_device.read(0xDC); // USB3PRM
        xhc_device.write(0xD8, superspeed_ports); // USB3_PSSEN

        let ehci2xhci_ports = xhc_device.read(0xD4); // XUSB2PRM
        xhc_device.write(0xD0, ehci2xhci_ports); // XUSB2PR
    }
}

pub fn init_xhc() {
    let Some(xhc_device) = DEVICES.get().unwrap().iter().find(|device| {
        let class = device.class_code();
        class.base() == 0x0C && class.sub() == 0x03 && class.interface() == 0x30
    }) else {
        return;
    };

    let xhc_mmio_base = xhc_device.bar(0).unwrap() & !0xF; // この場合インデックスが5未満なのでunwrapパニックは起こり得ない

    if xhc_device.vendor_id() == 0x8086 {
        switch_ehci_xhci(&xhc_device);
    }

    let xhci = unsafe { control::Controller::new(xhc_mmio_base) };
}
