pub mod i2c;
pub mod spi;
pub mod uart;

pub use i2c::{I2cBus, I2cDevice};
pub use spi::{SpiBus, SpiDevice};
pub use uart::{SerialBuffer, SerialRequest, SerialSnapshot};
