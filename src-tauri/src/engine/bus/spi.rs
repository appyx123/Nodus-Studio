/// An SPI slave responder capable of full-duplex byte transfer when Chip Select (CS) is low.
pub trait SpiDevice: Send {
    fn transfer(&mut self, byte: u8) -> u8;
}

#[derive(Default)]
pub struct SpiBus {
    device: Option<Box<dyn SpiDevice>>,
}

impl SpiBus {
    pub fn attach_device(&mut self, dev: Box<dyn SpiDevice>) {
        self.device = Some(dev);
    }

    pub fn transfer(&mut self, byte: u8) -> u8 {
        if let Some(dev) = self.device.as_mut() {
            dev.transfer(byte)
        } else {
            0xFF
        }
    }
}
