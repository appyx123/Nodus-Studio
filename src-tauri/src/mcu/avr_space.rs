use crate::engine::bus::{I2cBus, SpiBus};
use std::collections::VecDeque;

pub const DATA_SPACE_BYTES: usize = 0x0900;

pub struct IoSpace {
    pub regs: [u8; 64],
    pub ext_in: [u8; 3],
    pub spi_bus: SpiBus,
}

impl Default for IoSpace {
    fn default() -> Self {
        Self {
            regs: [0; 64],
            ext_in: [0; 3],
            spi_bus: SpiBus::default(),
        }
    }
}

impl IoSpace {
    pub fn new() -> Self {
        Self::default()
    }
}

impl avr_emulator::IoBus for IoSpace {
    fn read_io(&mut self, address: u8) -> u8 {
        match address {
            0x03 => (self.ext_in[0] & !self.regs[0x04]) | (self.regs[0x05] & self.regs[0x04]),
            0x06 => (self.ext_in[1] & !self.regs[0x07]) | (self.regs[0x08] & self.regs[0x07]),
            0x09 => (self.ext_in[2] & !self.regs[0x0A]) | (self.regs[0x0B] & self.regs[0x0A]),
            0x2D => self.regs[0x2D] | 0x80, // SPSR: always set SPIF on read
            addr => self.regs.get(addr as usize).copied().unwrap_or(0),
        }
    }

    fn write_io(&mut self, address: u8, value: u8) {
        match address {
            0x03 => self.regs[0x05] ^= value,
            0x06 => self.regs[0x08] ^= value,
            0x09 => self.regs[0x0B] ^= value,
            0x15..=0x17 => self.regs[address as usize] &= !value,
            0x2E => {
                let resp = self.spi_bus.transfer(value);
                self.regs[0x2E] = resp;
                self.regs[0x2D] |= 0x80;
            }
            addr => {
                if let Some(s) = self.regs.get_mut(addr as usize) {
                    *s = value;
                }
            }
        }
    }
}

pub struct DataSpace {
    pub mem: Vec<u8>,
    pub tx_out: Vec<u8>,
    pub rx_in: VecDeque<u8>,
    pub last_rx: u8,
    pub i2c_bus: I2cBus,
    pub twi_irq: bool,
}

impl Default for DataSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSpace {
    pub fn new() -> Self {
        let mut s = Self {
            mem: vec![0; DATA_SPACE_BYTES],
            tx_out: Vec::new(),
            rx_in: VecDeque::new(),
            last_rx: 0,
            i2c_bus: I2cBus::default(),
            twi_irq: false,
        };
        s.reset();
        s
    }

    pub fn reset(&mut self) {
        self.mem.fill(0);
        self.mem[0xC0] = 0x60;
        self.mem[0xC2] = 0x06;
        self.mem[0xB9] = 0xF8;
        self.twi_irq = false;
    }

    pub fn sync_rx_flag(&mut self) {
        self.mem[0xC0] = (self.mem[0xC0] & !0x80) | if self.rx_in.is_empty() { 0 } else { 0x80 };
    }

    pub fn process_twi(&mut self, twcr: u8) {
        if twcr & 0x80 == 0 {
            return;
        }
        if twcr & 0x20 != 0 {
            let rep = matches!(self.mem[0xB9] & 0xF8, 0x08 | 0x18 | 0x28 | 0x40 | 0x50);
            self.mem[0xB9] = if rep { 0x10 } else { 0x08 };
        } else if twcr & 0x10 != 0 {
            self.i2c_bus.stop_transaction();
            self.mem[0xB9] = 0xF8;
            self.mem[0xBC] &= !0x90;
            return;
        } else {
            match self.mem[0xB9] & 0xF8 {
                0x08 | 0x10 => {
                    let sla = self.mem[0xBB];
                    let ack = self.i2c_bus.start_transaction(sla);
                    let is_rd = (sla & 1) != 0;
                    self.mem[0xB9] = match (is_rd, ack) {
                        (true, true) => 0x40,
                        (true, false) => 0x48,
                        (false, true) => 0x18,
                        (false, false) => 0x20,
                    };
                }
                0x18 | 0x28 => {
                    let ack = self.i2c_bus.write_byte(self.mem[0xBB]);
                    self.mem[0xB9] = if ack { 0x28 } else { 0x30 };
                }
                0x40 | 0x50 => {
                    self.mem[0xBB] = self.i2c_bus.read_byte();
                    self.mem[0xB9] = if twcr & 0x40 != 0 { 0x50 } else { 0x58 };
                }
                _ => self.mem[0xB9] = 0xF8,
            }
        }
        self.mem[0xBC] |= 0x80;
        self.twi_irq = true;
    }
}

impl avr_emulator::DataBus for DataSpace {
    fn read(&mut self, addr: u32) -> u8 {
        match addr as usize {
            0xC6 => {
                if let Some(b) = self.rx_in.pop_front() {
                    self.last_rx = b;
                }
                self.mem[0xC0] =
                    (self.mem[0xC0] & !0x80) | if self.rx_in.is_empty() { 0 } else { 0x80 };
                self.last_rx
            }
            0xC0 => {
                self.mem[0xC0] =
                    (self.mem[0xC0] & !0x80) | if self.rx_in.is_empty() { 0 } else { 0x80 };
                self.mem[0xC0] | 0x20
            }
            i => self.mem.get(i).copied().unwrap_or(0),
        }
    }

    fn write(&mut self, addr: u32, val: u8) {
        match addr as usize {
            0xC6 => {
                self.tx_out.push(val);
                self.mem[0xC0] |= 0x60;
            }
            0xC0 => self.mem[0xC0] = (val & 0x03) | (self.mem[0xC0] & 0x80) | 0x60,
            0xBC => {
                self.mem[0xBC] = val;
                self.process_twi(val);
            }
            i => {
                if let Some(s) = self.mem.get_mut(i) {
                    *s = val;
                }
            }
        }
    }
}
