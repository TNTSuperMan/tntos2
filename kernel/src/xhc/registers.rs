#[repr(C)]
pub struct VolatileCell<T> {
    cell: core::cell::UnsafeCell<T>,
}
impl<T: Copy> VolatileCell<T> {
    pub fn read(&self) -> T {
        unsafe { self.cell.get().read_volatile() }
    }
    pub fn write(&self, value: T) {
        unsafe {
            self.cell.get().write_volatile(value);
        }
    }
}

#[repr(C, packed)]
pub struct CapabilityRegisters {
    pub caplength: VolatileCell<u8>,
    reserved1: VolatileCell<u8>,
    pub hci_version: VolatileCell<u16>,
    pub hcs_params1: VolatileCell<u32>,
    pub hcs_params2: VolatileCell<u32>,
    pub hcs_params3: VolatileCell<u32>,
    pub hcc_params1: VolatileCell<u32>,
    pub dboff: VolatileCell<u32>,
    pub rtsoff: VolatileCell<u32>,
    pub hcc_params2: VolatileCell<u32>,
}

#[repr(C, packed)]
pub struct OperationalRegisters {
    pub usbcmd: VolatileCell<u32>,
    pub usbsts: VolatileCell<u32>,
    pub pagesize: VolatileCell<u32>,
    reversed1: u64,
    pub dnctrl: VolatileCell<u32>,
    pub crcr: VolatileCell<u64>,
    reserved2: u128,
    pub dcbaap: VolatileCell<u64>,
    pub config: VolatileCell<u32>,
}
