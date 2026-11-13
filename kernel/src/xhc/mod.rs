use crate::pci::{DEVICES, Device};

mod control;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Key {
        usage: u8,
        pressed: bool,
        character: Option<char>,
        modifiers: u8,
    },
    Mouse {
        buttons: u8,
        dx: i8,
        dy: i8,
    },
}

struct InputQueue {
    events: [Option<InputEvent>; 128],
    head: usize,
    length: usize,
}

impl InputQueue {
    const fn new() -> Self {
        Self {
            events: [None; 128],
            head: 0,
            length: 0,
        }
    }

    fn push(&mut self, event: InputEvent) {
        if self.length == self.events.len() {
            self.head = (self.head + 1) % self.events.len();
            self.length -= 1;
        }
        let tail = (self.head + self.length) % self.events.len();
        self.events[tail] = Some(event);
        self.length += 1;
    }

    fn pop(&mut self) -> Option<InputEvent> {
        if self.length == 0 {
            return None;
        }
        let event = self.events[self.head].take();
        self.head = (self.head + 1) % self.events.len();
        self.length -= 1;
        event
    }
}

static INPUT_EVENTS: spin::Mutex<InputQueue> = spin::Mutex::new(InputQueue::new());

fn push_input(event: InputEvent) {
    INPUT_EVENTS.lock().push(event);
}

pub fn try_read_input() -> Option<InputEvent> {
    INPUT_EVENTS.lock().pop()
}

fn switch_ehci_xhci(xhc_device: &Device) {
    if DEVICES.get().unwrap().as_slice().iter().any(|device| {
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

pub fn init() {
    let Some(xhc_device) = DEVICES.get().unwrap().as_slice().iter().find(|device| {
        let class = device.class_code();
        class.base() == 0x0C && class.sub() == 0x03 && class.interface() == 0x30
    }) else {
        crate::println!("xHCI controller not found");
        return;
    };

    let Some(bar) = xhc_device.bar(0) else {
        crate::println!("xHCI controller has no BAR0");
        return;
    };
    if bar & 1 != 0 {
        crate::println!("xHCI BAR0 is not a memory BAR");
        return;
    }
    let xhc_mmio_base = bar & !0xF;
    if xhc_mmio_base == 0 {
        crate::println!("xHCI BAR0 is unassigned");
        return;
    }

    let command_status = xhc_device.read(0x04);
    xhc_device.write(0x04, (command_status as u16 | 0x0006) as u32);

    if xhc_device.vendor_id() == 0x8086 {
        switch_ehci_xhci(&xhc_device);
    }

    if let Err(error) = unsafe { control::init(xhc_mmio_base) } {
        crate::println!("xHCI initialization failed: {error}");
    }
}

pub fn poll_xhc() {
    control::poll();
}
