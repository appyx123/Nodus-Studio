pub mod avr_space;
pub mod avr_uno;
pub mod traits;

pub use avr_uno::AvrUno;
pub use traits::{parse_avr_elf, parse_intel_hex, McuPinState, Microcontroller};

#[cfg(test)]
mod tests;
