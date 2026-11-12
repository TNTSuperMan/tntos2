use crate::xhc::registers::{CapabilityRegisters, OperationalRegisters};

pub struct Controller {
    mmio_base: *const (),
    cap: *const CapabilityRegisters,
    op: *const OperationalRegisters,
    max_ports: u8,
    devmgr: (),
    cr: (),
    er: (),
}

impl Controller {
    pub unsafe fn new(mmio_base: u64) -> Self {
        let mmio_base: *const () = unsafe { core::mem::transmute(mmio_base) };
        let cap = mmio_base.cast();
        Self {
            mmio_base,
            cap,
            op: unsafe { mmio_base.byte_add(cap.as_ref().unwrap().caplength.read() as usize) }
                .cast(),
            max_ports: 0,
            devmgr: (),
            cr: (),
            er: (),
        }
    }
}
