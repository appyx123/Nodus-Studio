use super::avr_space::{DataSpace, IoSpace};
use super::traits::{parse_avr_elf, parse_intel_hex, McuPinState, Microcontroller};
use crate::engine::bus::{I2cBus, SpiBus};
use crate::engine::{LogicLevel, PinMode};
use crate::NodusError;
use avr_emulator::{Cpu, NoHooks, Profile, ProgramBus, FLAG_C, FLAG_I};

pub const ATMEGA328P_FLASH_BYTES: usize = 32 * 1024;
const RAMEND: u32 = 0x08FF;

#[rustfmt::skip]
pub const UNO_PINS: &[(&str, u8, u8)] = &[
    ("D0", 2, 0), ("D1", 2, 1), ("D2", 2, 2), ("D3", 2, 3),
    ("D4", 2, 4), ("D5", 2, 5), ("D6", 2, 6), ("D7", 2, 7),
    ("D8", 0, 0), ("D9", 0, 1), ("D10", 0, 2), ("D11", 0, 3),
    ("D12", 0, 4), ("D13", 0, 5), ("A0", 1, 0), ("A1", 1, 1),
    ("A2", 1, 2), ("A3", 1, 3), ("A4", 1, 4), ("A5", 1, 5),
];

struct FlashBus(Vec<u16>);
impl FlashBus {
    fn new() -> Self {
        Self(vec![0xFFFF; ATMEGA328P_FLASH_BYTES / 2])
    }
    fn load_bytes(&mut self, bytes: &[u8]) {
        self.0.fill(0xFFFF);
        for (idx, c) in bytes.chunks(2).enumerate() {
            if let Some(s) = self.0.get_mut(idx) {
                *s = u16::from_le_bytes([c[0], c.get(1).copied().unwrap_or(0xFF)]);
            }
        }
    }
}
impl ProgramBus for FlashBus {
    fn read_word(&mut self, addr: u32) -> u16 {
        self.0.get(addr as usize).copied().unwrap_or(0xFFFF)
    }
}

fn is_high_unary(w: u16) -> bool {
    (w & 0xff00) == 0x9500 && matches!(w & 0x000f, 0 | 1 | 2 | 3 | 5 | 6 | 7 | 0x0a)
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

    fn check_interrupts(&mut self) -> u64 {
        if self.cpu.sreg() & FLAG_I == 0 {
            return 0;
        }
        self.data.sync_rx_flag();
        let active_t0 = self.data.mem[0x6E] & self.io.regs[0x15];
        for (mask, vec_word) in [(0x02, 28), (0x04, 30), (0x01, 32)] {
            if active_t0 & mask != 0 {
                self.io.regs[0x15] &= !mask;
                return u64::from(
                    self.cpu
                        .interrupt_entry(&mut self.data, &mut self.io, vec_word)
                        .cycles,
                );
            }
        }
        let (a, b) = (self.data.mem[0xC0], self.data.mem[0xC1]);
        if (a & 0x80 != 0) && (b & 0x80 != 0) {
            return u64::from(
                self.cpu
                    .interrupt_entry(&mut self.data, &mut self.io, 36)
                    .cycles,
            );
        }
        if (a & 0x20 != 0) && (b & 0x20 != 0) {
            return u64::from(
                self.cpu
                    .interrupt_entry(&mut self.data, &mut self.io, 38)
                    .cycles,
            );
        }
        if self.data.twi_irq && (self.data.mem[0xBC] & 0x01 != 0) {
            self.data.twi_irq = false;
            return u64::from(
                self.cpu
                    .interrupt_entry(&mut self.data, &mut self.io, 48)
                    .cycles,
            );
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
            let raw = self.flash.0.get(pc).copied().unwrap_or(0xFFFF);
            let next = self.flash.0.get(pc + 1).copied().unwrap_or(0xFFFF);
            if (raw & 0xf000) == 0x5000 {
                self.cpu.set_sreg(self.cpu.sreg() & !FLAG_C);
            }
            let unary_rd = is_high_unary(raw).then_some(((raw >> 4) & 0x1f) as u8);
            let saved_r0 = self.cpu.register(0);
            if let Some(rd) = unary_rd {
                self.cpu.set_register(0, self.cpu.register(rd));
                if let Some(w) = self.flash.0.get_mut(pc) {
                    *w = raw & !0x01f0;
                }
            } else if is_high_unary(next) {
                if let Some(w) = self.flash.0.get_mut(pc + 1) {
                    *w = next & !0x01f0;
                }
            }
            let res = self
                .cpu
                .step(&mut self.flash, &mut self.data, &mut self.io, &mut hooks);
            if let Some(w) = self.flash.0.get_mut(pc) {
                *w = raw;
            }
            if let Some(w) = self.flash.0.get_mut(pc + 1) {
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
            let val = if level == LogicLevel::High { mask } else { 0 };
            self.io.ext_in[p_idx as usize] = (self.io.ext_in[p_idx as usize] & !mask) | val;
        }
    }

    fn pin_drives(&self) -> Vec<McuPinState> {
        let (d, p) = (
            [self.io.regs[4], self.io.regs[7], self.io.regs[10]],
            [self.io.regs[5], self.io.regs[8], self.io.regs[11]],
        );
        UNO_PINS
            .iter()
            .map(|&(pin_id, idx, bit)| {
                let mask = 1u8 << bit;
                let is_out = (d[idx as usize] & mask) != 0;
                let port_high = (p[idx as usize] & mask) != 0;
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

    fn i2c_bus(&mut self) -> Option<&mut I2cBus> {
        Some(&mut self.data.i2c_bus)
    }

    fn spi_bus(&mut self) -> Option<&mut SpiBus> {
        Some(&mut self.io.spi_bus)
    }
}
