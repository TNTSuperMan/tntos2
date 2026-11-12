use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    ptr::{read_volatile, write_volatile},
    sync::atomic::{Ordering, compiler_fence},
};

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

impl Controller {
    unsafe fn new(mmio_base: u64) -> Result<Self, &'static str> {
        let mmio = mmio_base as *mut u8;
        let cap_length = unsafe { read_volatile(mmio.add(0) as *const u8) } as usize;
        let hcs_params1 = unsafe { Self::read32(mmio, 0x04) };
        let hcs_params2 = unsafe { Self::read32(mmio, 0x08) };
        let hcc_params1 = unsafe { Self::read32(mmio, 0x10) };
        let max_ports = (hcs_params1 >> 24) as u8;
        if max_ports == 0 || max_ports as usize > MAX_PORTS {
            return Err("invalid xHCI port count");
        }

        unsafe { Self::take_ownership(mmio, hcc_params1)? };
        let mut controller = Self {
            mmio,
            op: unsafe { mmio.add(cap_length) },
            doorbells: unsafe { mmio.add((Self::read32(mmio, 0x14) & !3) as usize) },
            runtime: unsafe { mmio.add((Self::read32(mmio, 0x18) & !0x1f) as usize) },
            max_ports,
            context_size: if hcc_params1 & (1 << 2) != 0 { 64 } else { 32 },
            command_ring: RingState::new(),
            event_index: 0,
            event_cycle: true,
            slots: [SlotState::EMPTY; MAX_SLOTS],
            pending: [None; MAX_SLOTS],
        };

        unsafe {
            controller.reset()?;
            controller.setup_dma(hcs_params2)?;
            controller.start()?;
        }
        Ok(controller)
    }

    unsafe fn read32(base: *mut u8, offset: usize) -> u32 {
        unsafe { read_volatile(base.add(offset) as *const u32) }
    }

    unsafe fn write32(base: *mut u8, offset: usize, value: u32) {
        unsafe { write_volatile(base.add(offset) as *mut u32, value) };
    }

    unsafe fn write64(base: *mut u8, offset: usize, value: u64) {
        unsafe { write_volatile(base.add(offset) as *mut u64, value) };
    }

    unsafe fn take_ownership(mmio: *mut u8, hcc_params1: u32) -> Result<(), &'static str> {
        let mut offset = ((hcc_params1 >> 16) & 0xffff) as usize * 4;
        for _ in 0..256 {
            if offset == 0 {
                return Ok(());
            }
            let header = unsafe { Self::read32(mmio, offset) };
            let capability_id = header as u8;
            let next = ((header >> 8) & 0xff) as usize * 4;
            if capability_id == 1 {
                unsafe {
                    let semaphore = Self::read32(mmio, offset);
                    Self::write32(mmio, offset, semaphore | (1 << 24));
                }
                for _ in 0..TIMEOUT {
                    let semaphore = unsafe { Self::read32(mmio, offset) };
                    if semaphore & (1 << 16) == 0 {
                        unsafe { Self::write32(mmio, offset + 4, 0) };
                        return Ok(());
                    }
                    spin_loop();
                }
                return Err("xHCI BIOS ownership handoff timed out");
            }
            offset = next;
        }
        Err("invalid xHCI extended capability list")
    }

    unsafe fn reset(&mut self) -> Result<(), &'static str> {
        let command = unsafe { Self::read32(self.op, 0) };
        unsafe { Self::write32(self.op, 0, command & !1) };
        for _ in 0..TIMEOUT {
            if unsafe { Self::read32(self.op, 4) } & 1 != 0 {
                break;
            }
            spin_loop();
        }
        if unsafe { Self::read32(self.op, 4) } & 1 == 0 {
            return Err("xHCI controller did not halt");
        }

        unsafe { Self::write32(self.op, 0, (command & !1) | (1 << 1)) };
        for _ in 0..TIMEOUT {
            let command = unsafe { Self::read32(self.op, 0) };
            let status = unsafe { Self::read32(self.op, 4) };
            if command & (1 << 1) == 0 && status & (1 << 11) == 0 {
                return Ok(());
            }
            spin_loop();
        }
        Err("xHCI controller reset timed out")
    }

    unsafe fn setup_dma(&mut self, hcs_params2: u32) -> Result<(), &'static str> {
        let scratchpad_count =
            ((((hcs_params2 >> 27) & 0x1f) << 5) | ((hcs_params2 >> 21) & 0x1f)) as usize;
        if scratchpad_count > MAX_SCRATCHPADS {
            return Err("xHCI requires more scratchpads than supported");
        }
        if unsafe { Self::read32(self.op, 0x08) } & 1 == 0 {
            return Err("xHCI does not support 4 KiB DMA pages");
        }

        let dma = unsafe { DMA.get() };
        *dma = Dma::ZERO;
        unsafe {
            *RING_STATES.0.get() = [RingState::new(); MAX_SLOTS * 2];
        }
        for index in 0..MAX_SLOTS {
            unsafe { Self::init_ring(core::ptr::addr_of_mut!(dma.slots[index].control_ring)) };
            unsafe { Self::init_ring(core::ptr::addr_of_mut!(dma.slots[index].interrupt_ring)) };
        }
        unsafe { Self::init_ring(core::ptr::addr_of_mut!(dma.command_ring)) };

        if scratchpad_count != 0 {
            for index in 0..scratchpad_count {
                dma.scratchpad_array.0[index] = core::ptr::addr_of!(dma.scratchpads[index]) as u64;
            }
            dma.dcbaa[0] = core::ptr::addr_of!(dma.scratchpad_array) as u64;
        }

        dma.erst = ErstEntry {
            ring_base: core::ptr::addr_of!(dma.event_ring) as u64,
            ring_size: RING_SIZE as u32,
            reserved: 0,
        };

        unsafe {
            Self::write64(
                self.op,
                0x18,
                core::ptr::addr_of!(dma.command_ring) as u64 | 1,
            );
            Self::write64(self.op, 0x30, core::ptr::addr_of!(dma.dcbaa) as u64);

            let runtime_interrupter = self.runtime.add(0x20);
            Self::write32(runtime_interrupter, 0x08, 1);
            Self::write64(
                runtime_interrupter,
                0x10,
                core::ptr::addr_of!(dma.erst) as u64,
            );
            Self::write64(
                runtime_interrupter,
                0x18,
                core::ptr::addr_of!(dma.event_ring) as u64,
            );

            let max_slots = (Self::read32(self.mmio, 0x04) & 0xff).min(MAX_SLOTS as u32);
            Self::write32(self.op, 0x38, max_slots);
        }
        Ok(())
    }

    unsafe fn init_ring(ring: *mut Ring) {
        let link = Trb {
            parameter: ring.addr() as u64,
            status: 0,
            control: (TRB_TYPE_LINK << 10) | (1 << 1) | 1,
        };
        unsafe { write_volatile(core::ptr::addr_of_mut!((*ring).0[LINK_INDEX]), link) };
    }

    unsafe fn start(&mut self) -> Result<(), &'static str> {
        let command = unsafe { Self::read32(self.op, 0) };
        unsafe { Self::write32(self.op, 0, (command & !(1 << 2)) | 1) };
        for _ in 0..TIMEOUT {
            if unsafe { Self::read32(self.op, 4) } & 1 == 0 {
                return Ok(());
            }
            spin_loop();
        }
        Err("xHCI controller failed to start")
    }

    unsafe fn next_event(&mut self) -> Option<Trb> {
        let dma = unsafe { DMA.get() };
        let event_ptr = core::ptr::addr_of!(dma.event_ring.0[self.event_index]);
        let event = unsafe { read_volatile(event_ptr) };
        if (event.control & 1 != 0) != self.event_cycle {
            return None;
        }
        compiler_fence(Ordering::Acquire);

        self.event_index += 1;
        if self.event_index == RING_SIZE {
            self.event_index = 0;
            self.event_cycle = !self.event_cycle;
        }

        let erdp = core::ptr::addr_of!(dma.event_ring.0[self.event_index]) as u64 | (1 << 3);
        unsafe { Self::write64(self.runtime.add(0x20), 0x18, erdp) };
        Some(event)
    }

    fn ring_doorbell(&self, slot_id: u8, target: u8) {
        unsafe { Self::write32(self.doorbells, slot_id as usize * 4, target as u32) };
    }

    fn cache_unmatched_event(&mut self, event: Trb) {
        if event.kind() != TRB_TYPE_TRANSFER_EVENT {
            return;
        }
        let slot_id = (event.control >> 24) as usize;
        if slot_id != 0 && slot_id <= MAX_SLOTS {
            self.pending[slot_id - 1] = Some(event);
        }
    }

    fn wait_command(&mut self) -> Result<Trb, &'static str> {
        for _ in 0..TIMEOUT {
            if let Some(event) = unsafe { self.next_event() } {
                if event.kind() == TRB_TYPE_COMMAND_EVENT {
                    return Ok(event);
                }
                self.cache_unmatched_event(event);
            }
            spin_loop();
        }
        Err("xHCI command timed out")
    }

    fn issue_command(&mut self, command: Trb) -> Result<Trb, &'static str> {
        let dma = unsafe { DMA.get() };
        let command_type = ((command.control >> 10) & 0x3f) as u8;
        self.command_ring
            .push(core::ptr::addr_of_mut!(dma.command_ring), command);
        compiler_fence(Ordering::Release);
        self.ring_doorbell(0, 0);
        let completion = self.wait_command()?;
        if completion.completion_code() != COMPLETION_SUCCESS {
            crate::println!(
                "xHCI command {command_type} failed, completion {}",
                completion.completion_code()
            );
            return Err("xHCI command failed");
        }
        Ok(completion)
    }

    fn wait_transfer(&mut self, slot_id: u8, endpoint_id: u8) -> Result<Trb, &'static str> {
        let slot_index = slot_id as usize - 1;
        if let Some(event) = self.pending[slot_index].take() {
            if (event.control >> 16) & 0x1f == endpoint_id as u32 {
                return Self::check_transfer_event(event);
            }
            self.pending[slot_index] = Some(event);
        }
        for _ in 0..TIMEOUT {
            if let Some(event) = unsafe { self.next_event() } {
                if event.kind() == TRB_TYPE_TRANSFER_EVENT {
                    let event_slot = (event.control >> 24) as u8;
                    let event_endpoint = ((event.control >> 16) & 0x1f) as u8;
                    if event_slot == slot_id && event_endpoint == endpoint_id {
                        return Self::check_transfer_event(event);
                    }
                    self.cache_unmatched_event(event);
                } else if event.kind() == TRB_TYPE_COMMAND_EVENT {
                    return Err("unexpected xHCI command completion");
                }
            }
            spin_loop();
        }
        Err("xHCI transfer timed out")
    }

    fn check_transfer_event(event: Trb) -> Result<Trb, &'static str> {
        match event.completion_code() {
            COMPLETION_SUCCESS | COMPLETION_SHORT_PACKET => Ok(event),
            _ => Err("xHCI transfer failed"),
        }
    }

    fn control_transfer(
        &mut self,
        slot_index: usize,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        buffer_length: usize,
    ) -> Result<usize, &'static str> {
        let dma = unsafe { DMA.get() };
        let slot_dma = &mut dma.slots[slot_index];
        let has_data = buffer_length != 0;
        let setup = request_type as u64
            | ((request as u64) << 8)
            | ((value as u64) << 16)
            | ((index as u64) << 32)
            | ((buffer_length as u64) << 48);
        let setup_trb = Trb {
            parameter: setup,
            status: 8,
            control: (TRB_TYPE_SETUP_STAGE << 10)
                | (1 << 6)
                | (1 << 4)
                | ((if has_data {
                    if request_type & 0x80 != 0 { 3 } else { 2 }
                } else {
                    0
                }) << 16),
        };
        let control_ring = core::ptr::addr_of_mut!(slot_dma.control_ring);
        let mut producer = RING_STATES.get(slot_index);
        producer.push(control_ring, setup_trb);

        if has_data {
            let report_buffer = core::ptr::addr_of!(slot_dma.control_buffer) as u64;
            let data_trb = Trb {
                parameter: report_buffer,
                status: buffer_length as u32,
                control: (TRB_TYPE_DATA_STAGE << 10)
                    | (1 << 4)
                    | if request_type & 0x80 != 0 { 1 << 16 } else { 0 },
            };
            producer.push(control_ring, data_trb);
        }

        let status_trb = Trb {
            parameter: 0,
            status: 0,
            control: (TRB_TYPE_STATUS_STAGE << 10)
                | (1 << 5)
                | if !has_data || request_type & 0x80 == 0 {
                    1 << 16
                } else {
                    0
                },
        };
        producer.push(control_ring, status_trb);
        RING_STATES.set(slot_index, producer);
        compiler_fence(Ordering::Release);
        let slot_id = self.slots[slot_index].slot_id;
        self.ring_doorbell(slot_id, 1);
        let event = self.wait_transfer(slot_id, 1).map_err(|error| {
            crate::println!("USB control request {request_type:02x}/{request:02x} failed: {error}");
            error
        })?;
        Ok(buffer_length - (event.status as usize & 0x00ff_ffff).min(buffer_length))
    }

    fn command_input_context(
        &mut self,
        slot_index: usize,
        command_type: u32,
    ) -> Result<Trb, &'static str> {
        let dma = unsafe { DMA.get() };
        self.issue_command(Trb {
            parameter: core::ptr::addr_of!(dma.slots[slot_index].input_context) as u64,
            status: 0,
            control: (command_type << 10) | ((self.slots[slot_index].slot_id as u32) << 24),
        })
    }

    fn enumerate_port(&mut self, port: u8) -> Result<(), &'static str> {
        let portsc_offset = 0x400 + (port as usize - 1) * 0x10;
        let portsc = unsafe { Self::read32(self.op, portsc_offset) };
        if portsc & PORTSC_CONNECTED == 0 {
            return Ok(());
        }

        let speed = ((portsc >> 10) & 0xf) as u8;
        if speed == 0 {
            return Err("xHCI port reported an invalid speed");
        }
        let reset_bit = if speed >= 4 {
            PORTSC_WARM_RESET
        } else {
            PORTSC_RESET
        };
        let reset_value = (portsc & ((3 << 14) | (7 << 25))) | PORTSC_POWER | reset_bit;
        unsafe { Self::write32(self.op, portsc_offset, reset_value) };
        let mut ready = false;
        for _ in 0..TIMEOUT {
            let value = unsafe { Self::read32(self.op, portsc_offset) };
            if value & reset_bit == 0 && (value & PORTSC_ENABLED != 0 || speed >= 4) {
                ready = true;
                break;
            }
            spin_loop();
        }
        if !ready {
            return Err("xHCI port reset timed out");
        }

        let completion = self.issue_command(Trb {
            parameter: 0,
            status: 0,
            control: TRB_TYPE_ENABLE_SLOT << 10,
        })?;
        let slot_id = (completion.control >> 24) as u8;
        if slot_id == 0 || slot_id as usize > MAX_SLOTS {
            return Err("xHCI assigned an unsupported slot ID");
        }
        let slot_index = slot_id as usize - 1;
        let ep0_ring = {
            let dma = unsafe { DMA.get() };
            core::ptr::addr_of!(dma.slots[slot_index].control_ring) as u64
        };
        let device_context = {
            let dma = unsafe { DMA.get() };
            core::ptr::addr_of!(dma.slots[slot_index].device_context) as u64
        };
        let input_context = {
            let dma = unsafe { DMA.get() };
            core::ptr::addr_of!(dma.slots[slot_index].input_context) as u64
        };
        {
            let dma = unsafe { DMA.get() };
            dma.dcbaa[slot_id as usize] = device_context;
            let input = &mut dma.slots[slot_index].input_context.0;
            input.fill(0);
            let dwords = self.context_size / 4;
            let slot_ctx = dwords;
            let ep0_ctx = dwords * 2;
            input[1] = (1 << 0) | (1 << 1);
            input[slot_ctx] = ((speed as u32) << 20) | (1 << 27);
            input[slot_ctx + 1] = (port as u32) << 16;
            let max_packet = match speed {
                1 | 2 => 8,
                3 => 64,
                4 => 512,
                _ => 64,
            };
            Self::write_endpoint_context(input, ep0_ctx, 4, max_packet, ep0_ring, 8, 0);
        }
        self.slots[slot_index] = SlotState {
            active: true,
            slot_id,
            endpoint_id: 0,
            kind: HidKind::None,
            report_length: 0,
            keyboard_keys: [0; 6],
            mouse_buttons: 0,
        };
        compiler_fence(Ordering::Release);
        self.issue_command(Trb {
            parameter: input_context,
            status: 0,
            control: (TRB_TYPE_ADDRESS_DEVICE << 10) | ((slot_id as u32) << 24),
        })?;

        let initial_packet_size = match speed {
            1 | 2 => 8,
            3 => 64,
            4 => 512,
            _ => 64,
        };
        let _ = self.control_transfer(slot_index, 0x80, 6, 0x0100, 0, 8)?;
        let device_bytes = unsafe { DMA.get() }.slots[slot_index].control_buffer;
        if device_bytes[1] != 1 || device_bytes[0] < 8 {
            return Err("invalid USB device descriptor");
        }
        let packet_size = if speed == 4 {
            initial_packet_size
        } else {
            device_bytes[7] as u16
        };
        if packet_size == 0 {
            return Err("invalid USB endpoint-zero packet size");
        }
        self.evaluate_ep0_packet_size(slot_index, packet_size)?;

        let device_length = self.control_transfer(slot_index, 0x80, 6, 0x0100, 0, 18)?;
        if device_length < 18 {
            return Err("short USB device descriptor");
        }
        let config_header_length = self.control_transfer(slot_index, 0x80, 6, 0x0200, 0, 9)?;
        if config_header_length < 9 {
            return Err("short USB configuration descriptor");
        }
        let dma = unsafe { DMA.get() };
        let total_length = u16::from_le_bytes([
            dma.slots[slot_index].control_buffer[2],
            dma.slots[slot_index].control_buffer[3],
        ]) as usize;
        if !(9..=256).contains(&total_length) {
            return Err("unsupported USB configuration descriptor size");
        }
        let config_length = self.control_transfer(slot_index, 0x80, 6, 0x0200, 0, total_length)?;
        if config_length < total_length {
            return Err("short USB configuration descriptor");
        }

        let (config_value, interface_number, endpoint_address, endpoint_packet, interval, kind) = {
            let dma = unsafe { DMA.get() };
            Self::find_boot_hid(&dma.slots[slot_index].control_buffer[..config_length])?
        };
        self.control_transfer(slot_index, 0x00, 9, config_value as u16, 0, 0)?;
        self.control_transfer(slot_index, 0x21, 0x0b, 0, interface_number as u16, 0)?;

        let endpoint_id = ((endpoint_address & 0x0f) * 2) + 1;
        let interrupt_ring = {
            let dma = unsafe { DMA.get() };
            core::ptr::addr_of!(dma.slots[slot_index].interrupt_ring) as u64
        };
        {
            let dma = unsafe { DMA.get() };
            let input = &mut dma.slots[slot_index].input_context.0;
            input.fill(0);
            let dwords = self.context_size / 4;
            let slot_ctx = dwords;
            let endpoint_ctx = dwords * (endpoint_id as usize + 1);
            input[1] = 1 | (1 << endpoint_id);
            input[slot_ctx] = ((speed as u32) << 20) | ((endpoint_id as u32) << 27);
            input[slot_ctx + 1] = (port as u32) << 16;
            let interval_value = if speed == 3 || speed >= 4 {
                interval.clamp(1, 16)
            } else {
                let microframes = u32::from(interval.max(1)) * 8;
                (32 - (microframes - 1).leading_zeros() + 1).clamp(1, 16) as u8
            };
            Self::write_endpoint_context(
                input,
                endpoint_ctx,
                7,
                endpoint_packet,
                interrupt_ring,
                endpoint_packet as u32,
                interval_value,
            );
            input[endpoint_ctx + 4] = endpoint_packet as u32;
        }
        compiler_fence(Ordering::Release);
        self.issue_command(Trb {
            parameter: input_context,
            status: 0,
            control: (TRB_TYPE_CONFIGURE_ENDPOINT << 10) | ((slot_id as u32) << 24),
        })?;
        self.slots[slot_index].kind = kind;
        self.slots[slot_index].endpoint_id = endpoint_id;
        self.slots[slot_index].report_length = (endpoint_packet as usize).min(MAX_REPORT_SIZE);
        self.queue_interrupt_report(slot_index)?;
        let name = match kind {
            HidKind::Keyboard => "USB HID keyboard",
            HidKind::Mouse => "USB HID mouse",
            HidKind::None => "USB HID device",
        };
        crate::println!("{name} connected on xHCI port {port}");
        Ok(())
    }

    fn evaluate_ep0_packet_size(
        &mut self,
        slot_index: usize,
        max_packet: u16,
    ) -> Result<(), &'static str> {
        let dma = unsafe { DMA.get() };
        let input = &mut dma.slots[slot_index].input_context.0;
        input.fill(0);
        let dwords = self.context_size / 4;
        input[1] = 1 << 1;
        let ep0_ctx = dwords * 2;
        let control_ring = core::ptr::addr_of!(dma.slots[slot_index].control_ring) as u64;
        Self::write_endpoint_context(input, ep0_ctx, 4, max_packet, control_ring, 8, 0);
        compiler_fence(Ordering::Release);
        self.command_input_context(slot_index, TRB_TYPE_EVALUATE_CONTEXT)?;
        Ok(())
    }

    fn write_endpoint_context(
        context: &mut [u32],
        offset: usize,
        endpoint_type: u32,
        max_packet: u16,
        dequeue_pointer: u64,
        average_length: u32,
        interval: u8,
    ) {
        context[offset] = u32::from(interval) << 16;
        context[offset + 1] = (u32::from(max_packet) << 16) | (3 << 1) | (endpoint_type << 3);
        context[offset + 2] = (dequeue_pointer as u32 & !0xf) | 1;
        context[offset + 3] = (dequeue_pointer >> 32) as u32;
        context[offset + 4] = average_length << 16;
    }

    fn find_boot_hid(descriptors: &[u8]) -> Result<(u8, u8, u8, u16, u8, HidKind), &'static str> {
        if descriptors.len() < 9 || descriptors[1] != 2 {
            return Err("invalid USB configuration descriptor");
        }
        let configuration_value = descriptors[5];
        let mut offset = descriptors[0] as usize;
        let mut interface = None;
        while offset + 2 <= descriptors.len() {
            let length = descriptors[offset] as usize;
            if length < 2 || offset + length > descriptors.len() {
                return Err("malformed USB descriptor chain");
            }
            match descriptors[offset + 1] {
                4 if length >= 9 => {
                    let kind = if descriptors[offset + 5] == 3 && descriptors[offset + 6] == 1 {
                        match descriptors[offset + 7] {
                            1 => HidKind::Keyboard,
                            2 => HidKind::Mouse,
                            _ => HidKind::None,
                        }
                    } else {
                        HidKind::None
                    };
                    interface = if kind == HidKind::None {
                        None
                    } else {
                        Some((descriptors[offset + 2], kind))
                    };
                }
                5 if length >= 7 => {
                    if let Some((interface_number, kind)) = interface {
                        let address = descriptors[offset + 2];
                        let attributes = descriptors[offset + 3] & 3;
                        if address & 0x80 != 0 && attributes == 3 {
                            let packet = u16::from_le_bytes([
                                descriptors[offset + 4],
                                descriptors[offset + 5],
                            ]) & 0x07ff;
                            if packet != 0 {
                                return Ok((
                                    configuration_value,
                                    interface_number,
                                    address,
                                    packet,
                                    descriptors[offset + 6].max(1),
                                    kind,
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
            offset += length;
        }
        Err("no USB HID boot keyboard or mouse interface found")
    }

    fn queue_interrupt_report(&mut self, slot_index: usize) -> Result<(), &'static str> {
        let slot = self.slots[slot_index];
        let dma = unsafe { DMA.get() };
        let ring = core::ptr::addr_of_mut!(dma.slots[slot_index].interrupt_ring);
        let mut producer = RING_STATES.get(MAX_SLOTS + slot_index);
        producer.push(
            ring,
            Trb {
                parameter: core::ptr::addr_of!(dma.slots[slot_index].report_buffer) as u64,
                status: slot.report_length as u32,
                control: (1 << 2) | (1 << 5) | (slot.endpoint_id as u32) << 16 | (1 << 10),
            },
        );
        RING_STATES.set(MAX_SLOTS + slot_index, producer);
        compiler_fence(Ordering::Release);
        self.ring_doorbell(slot.slot_id, slot.endpoint_id);
        Ok(())
    }

    fn handle_report(&mut self, slot_index: usize, event: Trb) {
        let slot = self.slots[slot_index];
        if !slot.active || slot.kind == HidKind::None {
            return;
        }
        let residual = event.status as usize & 0x00ff_ffff;
        let actual_length = slot.report_length.saturating_sub(residual);
        let dma = unsafe { DMA.get() };
        let report = &dma.slots[slot_index].report_buffer[..actual_length.min(MAX_REPORT_SIZE)];
        match slot.kind {
            HidKind::Keyboard if report.len() >= 8 => {
                let mut keys = slot.keyboard_keys;
                for key in report[2..8].iter().copied().filter(|key| *key != 0) {
                    if !keys.contains(&key) {
                        let character = Self::key_to_ascii(key, report[0]);
                        crate::xhc::push_input(crate::xhc::InputEvent::Key {
                            usage: key,
                            pressed: true,
                            character: (character != 0).then_some(character as char),
                            modifiers: report[0],
                        });
                        if character == 0 {
                            crate::println!("keyboard down: usage 0x{key:02x}");
                        } else {
                            crate::println!("keyboard down: {}", character as char);
                        }
                    }
                }
                for key in slot.keyboard_keys.iter().copied().filter(|key| *key != 0) {
                    if !report[2..8].contains(&key) {
                        crate::xhc::push_input(crate::xhc::InputEvent::Key {
                            usage: key,
                            pressed: false,
                            character: None,
                            modifiers: report[0],
                        });
                        crate::println!("keyboard up: usage 0x{key:02x}");
                    }
                }
                keys.copy_from_slice(&report[2..8]);
                self.slots[slot_index].keyboard_keys = keys;
            }
            HidKind::Mouse if report.len() >= 3 => {
                let buttons = report[0];
                let dx = report[1] as i8;
                let dy = report[2] as i8;
                if buttons != slot.mouse_buttons || dx != 0 || dy != 0 {
                    crate::xhc::push_input(crate::xhc::InputEvent::Mouse { buttons, dx, dy });
                    crate::println!("mouse buttons={buttons:02x} dx={dx} dy={dy}");
                }
                self.slots[slot_index].mouse_buttons = buttons;
            }
            _ => {}
        }
    }

    fn key_to_ascii(usage: u8, modifiers: u8) -> u8 {
        let shifted = modifiers & 0x22 != 0;
        match usage {
            0x04..=0x1d => {
                let letter = b'a' + (usage - 0x04);
                if shifted {
                    letter.to_ascii_uppercase()
                } else {
                    letter
                }
            }
            0x1e..=0x26 => {
                let normal = b'1' + (usage - 0x1e);
                if shifted {
                    b"!@#$%^&*("[(usage - 0x1e) as usize]
                } else {
                    normal
                }
            }
            0x27 => {
                if shifted {
                    b')'
                } else {
                    b'0'
                }
            }
            0x28 => b'\n',
            0x2a => 8,
            0x2c => b' ',
            _ => 0,
        }
    }

    fn handle_transfer_event(&mut self, event: Trb) {
        let slot_id = (event.control >> 24) as usize;
        if slot_id == 0 || slot_id > MAX_SLOTS {
            return;
        }
        let slot_index = slot_id - 1;
        let endpoint_id = ((event.control >> 16) & 0x1f) as u8;
        if !self.slots[slot_index].active
            || self.slots[slot_index].endpoint_id != endpoint_id
            || self.slots[slot_index].kind == HidKind::None
        {
            return;
        }
        if event.completion_code() != COMPLETION_SUCCESS
            && event.completion_code() != COMPLETION_SHORT_PACKET
        {
            crate::println!("USB input transfer failed");
            self.slots[slot_index].active = false;
            return;
        }
        self.handle_report(slot_index, event);
        if let Err(error) = self.queue_interrupt_report(slot_index) {
            crate::println!("USB input error: {error}");
            self.slots[slot_index].active = false;
        }
    }

    fn poll(&mut self) {
        for slot_index in 0..MAX_SLOTS {
            if let Some(event) = self.pending[slot_index].take() {
                self.handle_transfer_event(event);
            }
        }
        while let Some(event) = unsafe { self.next_event() } {
            if event.kind() == TRB_TYPE_TRANSFER_EVENT {
                self.handle_transfer_event(event);
            }
        }
    }

    fn enumerate_ports(&mut self) {
        for port in 1..=self.max_ports {
            if let Err(error) = self.enumerate_port(port) {
                crate::println!("xHCI port {port}: {error}");
            }
        }
    }
}

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
