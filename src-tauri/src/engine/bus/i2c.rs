use std::collections::VecDeque;

/// An I2C slave responder capable of receiving writes and supplying read bytes.
pub trait I2cDevice: Send {
    fn address(&self) -> u8;
    fn on_write(&mut self, data: &[u8]);
    fn on_read(&mut self) -> u8;
}

#[derive(Default)]
pub struct I2cBus {
    devices: Vec<Box<dyn I2cDevice>>,
    active_address: Option<u8>,
    write_buffer: Vec<u8>,
    read_buffer: VecDeque<u8>,
}

impl I2cBus {
    pub fn register_device(&mut self, dev: Box<dyn I2cDevice>) {
        self.devices.push(dev);
    }

    pub fn start_transaction(&mut self, sla_rw: u8) -> bool {
        let addr = sla_rw >> 1;
        let is_read = (sla_rw & 1) != 0;
        self.write_buffer.clear();
        self.read_buffer.clear();
        if self.devices.iter().any(|d| d.address() == addr) {
            self.active_address = Some(addr);
            if is_read {
                if let Some(dev) = self.devices.iter_mut().find(|d| d.address() == addr) {
                    for _ in 0..64 {
                        self.read_buffer.push_back(dev.on_read());
                    }
                }
            }
            true
        } else {
            self.active_address = None;
            false
        }
    }

    pub fn write_byte(&mut self, byte: u8) -> bool {
        if self.active_address.is_some() {
            self.write_buffer.push(byte);
            true
        } else {
            false
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        self.read_buffer.pop_front().unwrap_or(0xFF)
    }

    pub fn stop_transaction(&mut self) {
        if let Some(addr) = self.active_address.take() {
            if !self.write_buffer.is_empty() {
                if let Some(dev) = self.devices.iter_mut().find(|d| d.address() == addr) {
                    dev.on_write(&self.write_buffer);
                }
                self.write_buffer.clear();
            }
        }
        self.read_buffer.clear();
    }
}
