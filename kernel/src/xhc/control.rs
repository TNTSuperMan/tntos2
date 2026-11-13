use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    ptr::{read_volatile, write_volatile},
    sync::atomic::{Ordering, compiler_fence},
};

mod impls;

const MAX_SLOTS: usize = 8;
const MAX_PORTS: usize = 256;
const RING_SIZE: usize = 256;
const LINK_INDEX: usize = RING_SIZE - 1;
const MAX_SCRATCHPADS: usize = 16;
const PAGE_SIZE: usize = 4096;
const TIMEOUT: usize = 20_000_000;

const TRB_TYPE_LINK: u32 = 6;
const TRB_TYPE_ENABLE_SLOT: u32 = 9;
const TRB_TYPE_ADDRESS_DEVICE: u32 = 11;
const TRB_TYPE_CONFIGURE_ENDPOINT: u32 = 12;
const TRB_TYPE_EVALUATE_CONTEXT: u32 = 13;
const TRB_TYPE_SETUP_STAGE: u32 = 2;
const TRB_TYPE_DATA_STAGE: u32 = 3;
const TRB_TYPE_STATUS_STAGE: u32 = 4;
const TRB_TYPE_TRANSFER_EVENT: u8 = 32;
const TRB_TYPE_COMMAND_EVENT: u8 = 33;

const COMPLETION_SUCCESS: u8 = 1;
const COMPLETION_SHORT_PACKET: u8 = 13;
const PORTSC_CONNECTED: u32 = 1;
const PORTSC_ENABLED: u32 = 1 << 1;
const PORTSC_RESET: u32 = 1 << 4;
const PORTSC_POWER: u32 = 1 << 9;
const PORTSC_WARM_RESET: u32 = 1 << 31;
const MAX_REPORT_SIZE: usize = 64;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Trb {
    parameter: u64,
    status: u32,
    control: u32,
}

impl Trb {
    const ZERO: Self = Self {
        parameter: 0,
        status: 0,
        control: 0,
    };

    fn kind(&self) -> u8 {
        ((self.control >> 10) & 0x3f) as u8
    }

    fn completion_code(&self) -> u8 {
        (self.status >> 24) as u8
    }
}

#[repr(C, align(64))]
#[derive(Clone, Copy)]
struct Ring([Trb; RING_SIZE]);

impl Ring {
    const ZERO: Self = Self([Trb::ZERO; RING_SIZE]);
}

#[repr(C, align(64))]
#[derive(Clone, Copy)]
struct Context([u32; 33 * 16]);

impl Context {
    const ZERO: Self = Self([0; 33 * 16]);
}

#[repr(C, align(4096))]
#[derive(Clone, Copy)]
struct Page([u8; PAGE_SIZE]);

impl Page {
    const ZERO: Self = Self([0; PAGE_SIZE]);
}

#[repr(C, align(64))]
#[derive(Clone, Copy)]
struct ScratchpadArray([u64; MAX_SCRATCHPADS]);

impl ScratchpadArray {
    const ZERO: Self = Self([0; MAX_SCRATCHPADS]);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ErstEntry {
    ring_base: u64,
    ring_size: u32,
    reserved: u32,
}

#[repr(C, align(64))]
struct SlotDma {
    input_context: Context,
    device_context: Context,
    control_ring: Ring,
    interrupt_ring: Ring,
    control_buffer: [u8; 256],
    report_buffer: [u8; MAX_REPORT_SIZE],
}

impl SlotDma {
    const ZERO: Self = Self {
        input_context: Context::ZERO,
        device_context: Context::ZERO,
        control_ring: Ring::ZERO,
        interrupt_ring: Ring::ZERO,
        control_buffer: [0; 256],
        report_buffer: [0; MAX_REPORT_SIZE],
    };
}

#[repr(C, align(64))]
struct Dma {
    dcbaa: [u64; 256],
    command_ring: Ring,
    event_ring: Ring,
    erst: ErstEntry,
    scratchpad_array: ScratchpadArray,
    scratchpads: [Page; MAX_SCRATCHPADS],
    slots: [SlotDma; MAX_SLOTS],
}

impl Dma {
    const ZERO: Self = Self {
        dcbaa: [0; 256],
        command_ring: Ring::ZERO,
        event_ring: Ring::ZERO,
        erst: ErstEntry {
            ring_base: 0,
            ring_size: 0,
            reserved: 0,
        },
        scratchpad_array: ScratchpadArray::ZERO,
        scratchpads: [Page::ZERO; MAX_SCRATCHPADS],
        slots: [const { SlotDma::ZERO }; MAX_SLOTS],
    };
}

struct Global<T>(UnsafeCell<T>);

unsafe impl<T> Sync for Global<T> {}

impl<T> Global<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    unsafe fn get(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

static DMA: Global<Dma> = Global::new(Dma::ZERO);
static CONTROLLER: spin::Mutex<Option<Controller>> = spin::Mutex::new(None);

#[derive(Clone, Copy, PartialEq, Eq)]
enum HidKind {
    None,
    Keyboard,
    Mouse,
}

#[derive(Clone, Copy)]
struct SlotState {
    active: bool,
    slot_id: u8,
    endpoint_id: u8,
    kind: HidKind,
    report_length: usize,
    keyboard_keys: [u8; 6],
    mouse_buttons: u8,
}

impl SlotState {
    const EMPTY: Self = Self {
        active: false,
        slot_id: 0,
        endpoint_id: 0,
        kind: HidKind::None,
        report_length: 0,
        keyboard_keys: [0; 6],
        mouse_buttons: 0,
    };
}

#[derive(Clone, Copy)]
struct RingState {
    index: usize,
    cycle: bool,
}

impl RingState {
    const fn new() -> Self {
        Self {
            index: 0,
            cycle: true,
        }
    }

    fn push(&mut self, ring: *mut Ring, mut trb: Trb) {
        trb.control = (trb.control & !1) | u32::from(self.cycle);
        unsafe {
            write_volatile(core::ptr::addr_of_mut!((*ring).0[self.index]), trb);
        }
        self.index += 1;
        if self.index == LINK_INDEX {
            let link = Trb {
                parameter: ring.addr() as u64,
                status: 0,
                control: (TRB_TYPE_LINK << 10) | (1 << 1) | u32::from(self.cycle),
            };
            unsafe {
                write_volatile(core::ptr::addr_of_mut!((*ring).0[LINK_INDEX]), link);
            }
            self.index = 0;
            self.cycle = !self.cycle;
        }
    }
}

struct Controller {
    mmio: *mut u8,
    op: *mut u8,
    doorbells: *mut u8,
    runtime: *mut u8,
    max_ports: u8,
    context_size: usize,
    command_ring: RingState,
    event_index: usize,
    event_cycle: bool,
    slots: [SlotState; MAX_SLOTS],
    pending: [Option<Trb>; MAX_SLOTS],
}

unsafe impl Send for Controller {}

struct RingStates(UnsafeCell<[RingState; MAX_SLOTS * 2]>);
unsafe impl Sync for RingStates {}
static RING_STATES: RingStates = RingStates(UnsafeCell::new([RingState::new(); MAX_SLOTS * 2]));

impl RingStates {
    fn get(&self, index: usize) -> RingState {
        unsafe { (*self.0.get())[index] }
    }

    fn set(&self, index: usize, state: RingState) {
        unsafe { (*self.0.get())[index] = state };
    }
}

pub unsafe fn init(mmio_base: u64) -> Result<(), &'static str> {
    let controller = unsafe { Controller::new(mmio_base)? };
    *CONTROLLER.lock() = Some(controller);
    if let Some(controller) = CONTROLLER.lock().as_mut() {
        controller.enumerate_ports();
    }
    Ok(())
}

pub fn poll() {
    if let Some(controller) = CONTROLLER.lock().as_mut() {
        controller.poll();
    }
}
