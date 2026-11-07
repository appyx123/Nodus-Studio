use super::i2c::I2cDevice;

pub const SSD1306_WIDTH: usize = 128;
pub const SSD1306_HEIGHT: usize = 64;
pub const SSD1306_PAGES: usize = 8;
pub const SSD1306_BUFFER_SIZE: usize = SSD1306_WIDTH * SSD1306_PAGES; // 1024 bytes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmdState {
    Idle,
    ExpectArg(u8),
    ExpectColStart,
    ExpectColEnd(u8),
    ExpectPageStart,
    ExpectPageEnd(u8),
}

pub struct Ssd1306Device {
    address: u8,
    buffer: [u8; SSD1306_BUFFER_SIZE],
    addressing_mode: u8, // 0 = Horizontal, 1 = Vertical, 2 = Page
    col_start: u8,
    col_end: u8,
    page_start: u8,
    page_end: u8,
    current_col: u8,
    current_page: u8,
    display_on: bool,
    contrast: u8,
    inverse: bool,
    cmd_state: CmdState,
}

impl Default for Ssd1306Device {
    fn default() -> Self {
        Self::new(0x3C)
    }
}

impl Ssd1306Device {
    pub fn new(address: u8) -> Self {
        Self {
            address,
            buffer: [0; SSD1306_BUFFER_SIZE],
            addressing_mode: 2, // Default is page addressing mode
            col_start: 0,
            col_end: 127,
            page_start: 0,
            page_end: 7,
            current_col: 0,
            current_page: 0,
            display_on: false,
            contrast: 0x7F,
            inverse: false,
            cmd_state: CmdState::Idle,
        }
    }

    pub fn buffer(&self) -> &[u8; SSD1306_BUFFER_SIZE] {
        &self.buffer
    }

    pub fn is_display_on(&self) -> bool {
        self.display_on
    }

    pub fn is_inverse(&self) -> bool {
        self.inverse
    }

    pub fn contrast(&self) -> u8 {
        self.contrast
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> bool {
        if x >= SSD1306_WIDTH || y >= SSD1306_HEIGHT {
            return false;
        }
        let page = y / 8;
        let bit = y % 8;
        let byte = self.buffer[page * SSD1306_WIDTH + x];
        (byte & (1 << bit)) != 0
    }

    fn handle_command(&mut self, cmd: u8) {
        match self.cmd_state {
            CmdState::ExpectArg(opcode) => {
                match opcode {
                    0x20 => self.addressing_mode = cmd & 0x03,
                    0x81 => self.contrast = cmd,
                    _ => {} // Other single-byte parameters ignored
                }
                self.cmd_state = CmdState::Idle;
            }
            CmdState::ExpectColStart => {
                self.cmd_state = CmdState::ExpectColEnd(cmd.min(127));
            }
            CmdState::ExpectColEnd(start) => {
                self.col_start = start;
                self.col_end = cmd.min(127).max(start);
                self.current_col = self.col_start;
                self.cmd_state = CmdState::Idle;
            }
            CmdState::ExpectPageStart => {
                self.cmd_state = CmdState::ExpectPageEnd(cmd.min(7));
            }
            CmdState::ExpectPageEnd(start) => {
                self.page_start = start;
                self.page_end = cmd.min(7).max(start);
                self.current_page = self.page_start;
                self.cmd_state = CmdState::Idle;
            }
            CmdState::Idle => match cmd {
                0x21 => self.cmd_state = CmdState::ExpectColStart,
                0x22 => self.cmd_state = CmdState::ExpectPageStart,
                0x20 | 0x81 | 0xD5 | 0xA8 | 0xD3 | 0x8D | 0xD9 | 0xDA | 0xDB => {
                    self.cmd_state = CmdState::ExpectArg(cmd);
                }
                0xAE => self.display_on = false,
                0xAF => self.display_on = true,
                0xA6 => self.inverse = false,
                0xA7 => self.inverse = true,
                0xB0..=0xB7 => {
                    self.current_page = cmd - 0xB0;
                }
                0x00..=0x0F => {
                    self.current_col = (self.current_col & 0xF0) | (cmd & 0x0F);
                }
                0x10..=0x1F => {
                    self.current_col = (self.current_col & 0x0F) | ((cmd & 0x0F) << 4);
                }
                _ => {}
            },
        }
    }

    fn write_data_byte(&mut self, byte: u8) {
        let p = (self.current_page as usize).min(SSD1306_PAGES - 1);
        let c = (self.current_col as usize).min(SSD1306_WIDTH - 1);
        let idx = p * SSD1306_WIDTH + c;
        if idx < SSD1306_BUFFER_SIZE {
            self.buffer[idx] = byte;
        }

        match self.addressing_mode {
            0 => {
                // Horizontal addressing mode
                self.current_col = self.current_col.saturating_add(1);
                if self.current_col > self.col_end {
                    self.current_col = self.col_start;
                    self.current_page = self.current_page.saturating_add(1);
                    if self.current_page > self.page_end {
                        self.current_page = self.page_start;
                    }
                }
            }
            1 => {
                // Vertical addressing mode
                self.current_page = self.current_page.saturating_add(1);
                if self.current_page > self.page_end {
                    self.current_page = self.page_start;
                    self.current_col = self.current_col.saturating_add(1);
                    if self.current_col > self.col_end {
                        self.current_col = self.col_start;
                    }
                }
            }
            _ => {
                // Page addressing mode (default)
                self.current_col = self.current_col.saturating_add(1);
                if self.current_col > self.col_end {
                    self.current_col = self.col_start;
                }
            }
        }
    }
}

impl I2cDevice for Ssd1306Device {
    fn address(&self) -> u8 {
        self.address
    }

    fn on_write(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let control_byte = data[0];
        let payload = &data[1..];
        let is_data = (control_byte & 0x40) != 0;

        for &byte in payload {
            if is_data {
                self.write_data_byte(byte);
            } else {
                self.handle_command(byte);
            }
        }
    }

    fn on_read(&mut self) -> u8 {
        // Status register: bit 6 = Display ON/OFF (0 = ON, 1 = OFF in status byte)
        if self.display_on {
            0x00
        } else {
            0x40
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bus::I2cBus;

    #[test]
    fn ssd1306_handles_commands_and_data_streaming() {
        let mut oled = Ssd1306Device::new(0x3C);
        assert!(!oled.is_display_on());

        // Send Command 0xAF (Display ON) and 0x81 0xAA (Set Contrast)
        oled.on_write(&[0x00, 0xAF, 0x81, 0xAA]);
        assert!(oled.is_display_on());
        assert_eq!(oled.contrast(), 0xAA);

        // Set addressing mode to Horizontal (0x20, 0x00) and set column/page bounds
        oled.on_write(&[0x00, 0x20, 0x00, 0x21, 0, 127, 0x22, 0, 7]);
        assert_eq!(oled.addressing_mode, 0);

        // Send Data bytes (0x40 control byte): write 0x01 (pixel at y=0, x=0) and 0x80 (pixel at y=7, x=1)
        oled.on_write(&[0x40, 0x01, 0x80]);
        assert!(oled.get_pixel(0, 0));
        assert!(!oled.get_pixel(0, 1));
        assert!(oled.get_pixel(1, 7));
        assert!(!oled.get_pixel(1, 0));
    }

    #[test]
    fn ssd1306_integrates_with_i2c_bus() {
        let mut bus = I2cBus::default();
        bus.register_device(Box::new(Ssd1306Device::new(0x3C)));

        // Begin transaction to address 0x3C (write mode: SLA+W = 0x3C << 1 | 0 = 0x78)
        assert!(bus.start_transaction(0x78));

        // Send command stream: Display ON (0xAF)
        assert!(bus.write_byte(0x00));
        assert!(bus.write_byte(0xAF));
        bus.stop_transaction();

        // Read status byte (SLA+R = 0x3C << 1 | 1 = 0x79)
        assert!(bus.start_transaction(0x79));
        let status = bus.read_byte();
        bus.stop_transaction();
        assert_eq!(status, 0x00); // 0x00 indicates Display is ON
    }
}
