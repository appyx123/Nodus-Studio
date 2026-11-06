use super::traits::{parse_avr_elf, parse_intel_hex, McuPinState, Microcontroller};
use crate::engine::{LogicLevel, PinMode};
use crate::NodusError;
use avr_emulator::{Cpu, DataBus, IoBus, NoHooks, Profile, ProgramBus, FLAG_C, FLAG_I};
use std::collections::VecDeque;

pub const ATMEGA328P_FLASH_BYTES: usize = 32 * 1024;
const DATA_SPACE_BYTES: usize = 0x0900;
const RAMEND: u32 = 0x08FF;

#[rustfmt::skip]
pub const UNO_PINS: &[(&str, u8, u8)] = &[
    ("D0", 2, 0), ("D1", 2, 1), ("D2", 2, 2), ("D3", 2, 3),
    ("D4", 2, 4), ("D5", 2, 5), ("D6", 2, 6), ("D7", 2, 7),
    ("D8", 0, 0), ("D9", 0, 1), ("D10", 0, 2), ("D11", 0, 3),
    ("D12", 0, 4), ("D13", 0, 5), ("A0", 1, 0), ("A1", 1, 1),
    ("A2", 1, 2), ("A3", 1, 3), ("A4", 1, 4), ("A5", 1, 5),
];

struct FlashBus {
    words: Vec<u16>,
}

impl FlashBus {
    fn new() -> Self {
        Self {
            words: vec![0xFFFF; ATMEGA328P_FLASH_BYTES / 2],
        }
    }

    fn load_bytes(&mut self, bytes: &[u8]) {
        self.words.fill(0xFFFF);
        for (idx, chunk) in bytes.chunks(2).enumerate() {
            if let Some(slot) = self.words.get_mut(idx) {
                *slot = u16::from_le_bytes([chunk[0], chunk.get(1).copied().unwrap_or(0xFF)]);
            }
        }
    }
}

fn is_high_unary(word: u16) -> bool {
    (word & 0xff00) == 0x9500 && matches!(word & 0x000f, 0 | 1 | 2 | 3 | 5 | 6 | 7 | 0x0a)
}

impl ProgramBus for FlashBus {
    fn read_word(&mut self, word_address: u32) -> u16 {
        self.words
            .get(word_address as usize)
            .copied()
            .unwrap_or(0xFFFF)
    }
}

struct IoSpace {
    regs: [u8; 64],
    ext_in: [u8; 3],
}

impl IoSpace {
    fn new() -> Self {
        Self {
            regs: [0; 64],
            ext_in: [0; 3],
        }
    }
}

impl IoBus for IoSpace {
    fn read_io(&mut self, address: u8) -> u8 {
        match address {
            0x03 => (self.ext_in[0] & !self.regs[0x04]) | (self.regs[0x05] & self.regs[0x04]),
            0x06 => (self.ext_in[1] & !self.regs[0x07]) | (self.regs[0x08] & self.regs[0x07]),
            0x09 => (self.ext_in[2] & !self.regs[0x0A]) | (self.regs[0x0B] & self.regs[0x0A]),
            addr => self.regs.get(addr as usize).copied().unwrap_or(0),
        }
    }

    fn write_io(&mut self, address: u8, value: u8) {
        match address {
            0x03 => self.regs[0x05] ^= value,
            0x06 => self.regs[0x08] ^= value,
            0x09 => self.regs[0x0B] ^= value,
            0x15..=0x17 => self.regs[address as usize] &= !value,
            addr => {
                if let Some(slot) = self.regs.get_mut(addr as usize) {
                    *slot = value;
                }
            }
        }
    }
}

struct DataSpace {
    mem: Vec<u8>,
    tx_out: Vec<u8>,
    rx_in: VecDeque<u8>,
    last_rx: u8,
}

impl DataSpace {
    fn new() -> Self {
        let mut s = Self {
            mem: vec![0; DATA_SPACE_BYTES],
            tx_out: Vec::new(),
            rx_in: VecDeque::new(),
            last_rx: 0,
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.mem.fill(0);
        self.mem[0xC0] = 0x60;
        self.mem[0xC2] = 0x06;
    }

    fn sync_rx_flag(&mut self) {
        self.mem[0xC0] = (self.mem[0xC0] & !0x80) | if self.rx_in.is_empty() { 0 } else { 0x80 };
    }
}

impl DataBus for DataSpace {
    fn read(&mut self, address: u32) -> u8 {
        match address as usize {
            0xC6 => {
                if let Some(b) = self.rx_in.pop_front() {
                    self.last_rx = b;
                }
                self.sync_rx_flag();
                self.last_rx
            }
            0xC0 => {
                self.sync_rx_flag();
                self.mem[0xC0] | 0x20
            }
            idx => self.mem.get(idx).copied().unwrap_or(0),
        }
    }

    fn write(&mut self, address: u32, value: u8) {
        match address as usize {
            0xC6 => {
                self.tx_out.push(value);
                self.mem[0xC0] |= 0x60;
            }
            0xC0 => self.mem[0xC0] = (value & 0x03) | (self.mem[0xC0] & 0x80) | 0x60,
            idx => {
                if let Some(slot) = self.mem.get_mut(idx) {
                    *slot = value;
                }
            }
        }
    }
}

pub struct AvrUno {
    cpu: Cpu,
    flash: FlashBus,
    io: IoSpace,
    data: DataSpace,
    timer0_sub: u64,
    pwm_sub: u64,
    pwm_phase: u8,
}

impl Default for AvrUno {
    fn default() -> Self {
        Self::new()
    }
}

impl AvrUno {
    pub fn new() -> Self {
        let p = Profile::avre_plus()
            .with_program_counter_bits(16)
            .with_data_address_bits(16);
        let mut uno = Self {
            cpu: Cpu::new(p),
            flash: FlashBus::new(),
            io: IoSpace::new(),
            data: DataSpace::new(),
            timer0_sub: 0,
            pwm_sub: 0,
            pwm_phase: 0,
        };
        uno.reset();
        uno
    }

    fn advance_peripherals(&mut self, cycles: u64) {
        self.pwm_sub = self.pwm_sub.saturating_add(cycles);
        while self.pwm_sub >= 64 {
            self.pwm_sub -= 64;
            self.pwm_phase = self.pwm_phase.wrapping_add(1);
        }
        let prescaler = match self.io.regs[0x25] & 0x07 {
            1 => 1,
            2 => 8,
            3 => 64,
            4 => 256,
            5 => 1024,
            _ => 0,
        };
        if prescaler > 0 {
            self.timer0_sub = self.timer0_sub.saturating_add(cycles);
            while self.timer0_sub >= prescaler {
                self.timer0_sub -= prescaler;
                let tcnt0 = self.io.regs[0x26];
                let wgm = ((self.io.regs[0x25] & 0x08) >> 1) | (self.io.regs[0x24] & 0x03);
                let (next, wrapped) = if wgm == 2 && tcnt0 == self.io.regs[0x27] {
                    (0, false)
                } else {
                    tcnt0.overflowing_add(1)
                };
                self.io.regs[0x26] = next;
                self.io.regs[0x15] |= (u8::from(wrapped))
                    | (u8::from(next == self.io.regs[0x27]) << 1)
                    | (u8::from(next == self.io.regs[0x28]) << 2);
            }
        }
    }

    fn enter_irq(&mut self, vec_word: u32) -> u64 {
        let res = self
            .cpu
            .interrupt_entry(&mut self.data, &mut self.io, vec_word);
        u64::from(res.cycles)
    }

    fn check_interrupts(&mut self) -> u64 {
        if self.cpu.sreg() & FLAG_I == 0 {
            return 0;
        }
        self.data.sync_rx_flag();
        let active_t0 = self.data.mem[0x6E] & self.io.regs[0x15];
        for (mask, vec_word) in [(0x02, 28), (0x04, 30), (0x01, 32)] {
            if active_t0 & mask != 0 {
                self.io.regs[0x15] &= !mask;
                return self.enter_irq(vec_word);
            }
        }
        let (ucsr0a, ucsr0b) = (self.data.mem[0xC0], self.data.mem[0xC1]);
        if (ucsr0a & 0x80 != 0) && (ucsr0b & 0x80 != 0) {
            return self.enter_irq(36);
        }
        if (ucsr0a & 0x20 != 0) && (ucsr0b & 0x20 != 0) {
            return self.enter_irq(38);
        }
        0
    }

    fn pwm_override(&self, pin_id: &str) -> Option<bool> {
        let (tcnt0, phase) = (self.io.regs[0x26], self.pwm_phase);
        match pin_id {
            "D6" if self.io.regs[0x24] & 0x80 != 0 => Some(tcnt0 < self.io.regs[0x27]),
            "D5" if self.io.regs[0x24] & 0x20 != 0 => Some(tcnt0 < self.io.regs[0x28]),
            "D9" if self.data.mem[0x80] & 0x80 != 0 => Some(phase < self.data.mem[0x88]),
            "D10" if self.data.mem[0x80] & 0x20 != 0 => Some(phase < self.data.mem[0x8A]),
            "D11" if self.data.mem[0xB0] & 0x80 != 0 => Some(phase < self.data.mem[0xB3]),
            "D3" if self.data.mem[0xB0] & 0x20 != 0 => Some(phase < self.data.mem[0xB4]),
            _ => None,
        }
    }
}

impl Microcontroller for AvrUno {
    fn backend_id(&self) -> &'static str {
        "avr_atmega328p"
    }

    fn load_hex(&mut self, hex_text: &str) -> Result<(), NodusError> {
        let b = parse_intel_hex(hex_text, ATMEGA328P_FLASH_BYTES)?;
        self.flash.load_bytes(&b);
        self.reset();
        Ok(())
    }

    fn load_elf(&mut self, elf_bytes: &[u8]) -> Result<(), NodusError> {
        let b = parse_avr_elf(elf_bytes, ATMEGA328P_FLASH_BYTES)?;
        self.flash.load_bytes(&b);
        self.reset();
        Ok(())
    }

    fn reset(&mut self) {
        self.cpu = Cpu::new(self.cpu.profile());
        self.cpu.set_sp(RAMEND);
        self.cpu.set_pc(0);
        self.io.regs.fill(0);
        self.data.reset();
        self.timer0_sub = 0;
        self.pwm_sub = 0;
        self.pwm_phase = 0;
    }

    fn step_cycles(&mut self, target_cycles: u64) -> Result<u64, NodusError> {
        let mut executed = 0u64;
        let mut hooks = NoHooks;
        while executed < target_cycles {
            let pc = self.cpu.pc() as usize;
            let raw = self.flash.words.get(pc).copied().unwrap_or(0xFFFF);
            let next = self.flash.words.get(pc + 1).copied().unwrap_or(0xFFFF);
            if (raw & 0xf000) == 0x5000 {
                self.cpu.set_sreg(self.cpu.sreg() & !FLAG_C);
            }
            let unary_rd = is_high_unary(raw).then_some(((raw >> 4) & 0x1f) as u8);
            let saved_r0 = self.cpu.register(0);
            if let Some(rd) = unary_rd {
                self.cpu.set_register(0, self.cpu.register(rd));
                if let Some(w) = self.flash.words.get_mut(pc) {
                    *w = raw & !0x01f0;
                }
            } else if is_high_unary(next) {
                if let Some(w) = self.flash.words.get_mut(pc + 1) {
                    *w = next & !0x01f0;
                }
            }
            let res = self
                .cpu
                .step(&mut self.flash, &mut self.data, &mut self.io, &mut hooks);
            if let Some(w) = self.flash.words.get_mut(pc) {
                *w = raw;
            }
            if let Some(w) = self.flash.words.get_mut(pc + 1) {
                *w = next;
            }
            let res = res.map_err(|e| NodusError::Circuit(format!("AVR CPU step error: {e:?}")))?;
            if let Some(rd) = unary_rd {
                let updated = self.cpu.register(0);
                self.cpu.set_register(0, saved_r0);
                self.cpu.set_register(rd, updated);
            }
            let step_c = u64::from(res.cycles.max.max(1));
            self.advance_peripherals(step_c);
            let irq_c = self.check_interrupts();
            if irq_c > 0 {
                self.advance_peripherals(irq_c);
            }
            executed = executed.saturating_add(step_c + irq_c);
        }
        Ok(executed)
    }

    fn set_input_pin(&mut self, pin_id: &str, level: LogicLevel) {
        if let Some(&(_, p_idx, bit)) = UNO_PINS.iter().find(|&&(id, _, _)| id == pin_id) {
            let mask = 1u8 << bit;
            self.io.ext_in[p_idx as usize] = (self.io.ext_in[p_idx as usize] & !mask)
                | if level == LogicLevel::High { mask } else { 0 };
        }
    }

    fn pin_drives(&self) -> Vec<McuPinState> {
        let ddr = [self.io.regs[0x04], self.io.regs[0x07], self.io.regs[0x0A]];
        let port = [self.io.regs[0x05], self.io.regs[0x08], self.io.regs[0x0B]];
        UNO_PINS
            .iter()
            .map(|&(pin_id, p_idx, bit)| {
                let mask = 1u8 << bit;
                let is_out = (ddr[p_idx as usize] & mask) != 0;
                let port_high = (port[p_idx as usize] & mask) != 0;
                let (mode, level) = if is_out {
                    let lvl = if self.pwm_override(pin_id).unwrap_or(port_high) {
                        LogicLevel::High
                    } else {
                        LogicLevel::Low
                    };
                    (PinMode::Output, lvl)
                } else if port_high {
                    (PinMode::InputPullup, LogicLevel::High)
                } else {
                    (PinMode::Input, LogicLevel::HighZ)
                };
                McuPinState {
                    pin_id,
                    mode,
                    level,
                }
            })
            .collect()
    }

    fn drain_serial_tx(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.data.tx_out)
    }

    fn push_serial_rx(&mut self, bytes: &[u8]) {
        self.data.rx_in.extend(bytes.iter().copied());
        self.data.sync_rx_flag();
    }

    fn total_cycles(&self) -> u64 {
        self.cpu.total_cycles()
    }
}
