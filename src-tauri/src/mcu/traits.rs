use crate::engine::{LogicLevel, PinMode};
use crate::NodusError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McuPinState {
    pub pin_id: &'static str,
    pub mode: PinMode,
    pub level: LogicLevel,
}

/// Hardware-agnostic microcontroller emulation contract.
pub trait Microcontroller: Send {
    fn backend_id(&self) -> &'static str;
    fn load_hex(&mut self, hex_text: &str) -> Result<(), NodusError>;
    fn load_elf(&mut self, elf_bytes: &[u8]) -> Result<(), NodusError>;
    fn reset(&mut self);
    fn step_cycles(&mut self, target_cycles: u64) -> Result<u64, NodusError>;
    fn set_input_pin(&mut self, pin_id: &str, level: LogicLevel);
    fn pin_drives(&self) -> Vec<McuPinState>;
    fn drain_serial_tx(&mut self) -> Vec<u8>;
    fn push_serial_rx(&mut self, bytes: &[u8]);
    fn total_cycles(&self) -> u64;
    fn i2c_bus(&mut self) -> Option<&mut crate::engine::bus::I2cBus> {
        None
    }
    fn spi_bus(&mut self) -> Option<&mut crate::engine::bus::SpiBus> {
        None
    }
}

/// Parses Intel HEX (`.hex`) records into a flat flash byte image bounded by `max_bytes`.
pub fn parse_intel_hex(hex_text: &str, max_bytes: usize) -> Result<Vec<u8>, NodusError> {
    let mut flash = vec![0xFFu8; max_bytes];
    let mut upper_address: usize = 0;
    let mut saw_eof = false;

    for (line_idx, raw_line) in hex_text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(hex_body) = line.strip_prefix(':') else {
            return Err(NodusError::Circuit(format!(
                "Invalid Intel HEX line {} (missing ':')",
                line_idx + 1
            )));
        };
        if hex_body.len() < 10 || hex_body.len() % 2 != 0 {
            return Err(NodusError::Circuit(format!(
                "Malformed Intel HEX record at line {}",
                line_idx + 1
            )));
        }
        let mut bytes = Vec::with_capacity(hex_body.len() / 2);
        for chunk in hex_body.as_bytes().as_chunks::<2>().0 {
            let text = std::str::from_utf8(chunk).unwrap_or("");
            let byte = u8::from_str_radix(text, 16).map_err(|_| {
                NodusError::Circuit(format!("Invalid hex digits at line {}", line_idx + 1))
            })?;
            bytes.push(byte);
        }
        let sum: u8 = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        if sum != 0 {
            return Err(NodusError::Circuit(format!(
                "Intel HEX checksum mismatch at line {}",
                line_idx + 1
            )));
        }
        let count = bytes[0] as usize;
        if bytes.len() != count + 5 {
            return Err(NodusError::Circuit(format!(
                "Intel HEX byte count mismatch at line {}",
                line_idx + 1
            )));
        }
        let offset = u16::from_be_bytes([bytes[1], bytes[2]]) as usize;
        let record_type = bytes[3];
        let data = &bytes[4..4 + count];

        match record_type {
            0x00 => {
                let start = upper_address.saturating_add(offset);
                let end = start.saturating_add(count);
                if end > max_bytes {
                    return Err(NodusError::Circuit(format!(
                        "Firmware address 0x{end:X} exceeds flash size 0x{max_bytes:X}"
                    )));
                }
                flash[start..end].copy_from_slice(data);
            }
            0x01 => {
                saw_eof = true;
                break;
            }
            0x02 if count == 2 => {
                upper_address = (u16::from_be_bytes([data[0], data[1]]) as usize) << 4;
            }
            0x04 if count == 2 => {
                upper_address = (u16::from_be_bytes([data[0], data[1]]) as usize) << 16;
            }
            0x03 | 0x05 => {}
            other => {
                return Err(NodusError::Circuit(format!(
                    "Unsupported Intel HEX record type 0x{other:02X}"
                )));
            }
        }
    }
    if !saw_eof {
        return Err(NodusError::Circuit(
            "Intel HEX image is missing EOF record (:00000001FF)".into(),
        ));
    }
    Ok(flash)
}

/// Parses a 32-bit little-endian AVR `.elf` binary into a flat flash byte image bounded by `max_bytes`.
pub fn parse_avr_elf(elf_bytes: &[u8], max_bytes: usize) -> Result<Vec<u8>, NodusError> {
    if elf_bytes.len() < 52 || &elf_bytes[0..4] != b"\x7FELF" {
        return Err(NodusError::Circuit("Invalid ELF header".into()));
    }
    if elf_bytes[4] != 1 || elf_bytes[5] != 1 {
        return Err(NodusError::Circuit(
            "ELF must be 32-bit little-endian".into(),
        ));
    }
    let machine = u16::from_le_bytes([elf_bytes[18], elf_bytes[19]]);
    if machine != 0x53 {
        return Err(NodusError::Circuit(format!(
            "ELF machine 0x{machine:X} is not AVR (0x53)"
        )));
    }
    let phoff =
        u32::from_le_bytes([elf_bytes[28], elf_bytes[29], elf_bytes[30], elf_bytes[31]]) as usize;
    let phentsize = u16::from_le_bytes([elf_bytes[42], elf_bytes[43]]) as usize;
    let phnum = u16::from_le_bytes([elf_bytes[44], elf_bytes[45]]) as usize;
    if phentsize < 32 || phoff.saturating_add(phentsize.saturating_mul(phnum)) > elf_bytes.len() {
        return Err(NodusError::Circuit(
            "ELF program header table is truncated".into(),
        ));
    }

    let mut flash = vec![0xFFu8; max_bytes];
    let mut loaded_any = false;
    for i in 0..phnum {
        let base = phoff + i * phentsize;
        let p_type = u32::from_le_bytes([
            elf_bytes[base],
            elf_bytes[base + 1],
            elf_bytes[base + 2],
            elf_bytes[base + 3],
        ]);
        if p_type != 1 {
            continue;
        }
        let p_offset = u32::from_le_bytes([
            elf_bytes[base + 4],
            elf_bytes[base + 5],
            elf_bytes[base + 6],
            elf_bytes[base + 7],
        ]) as usize;
        let p_paddr = u32::from_le_bytes([
            elf_bytes[base + 12],
            elf_bytes[base + 13],
            elf_bytes[base + 14],
            elf_bytes[base + 15],
        ]) as usize;
        let p_filesz = u32::from_le_bytes([
            elf_bytes[base + 16],
            elf_bytes[base + 17],
            elf_bytes[base + 18],
            elf_bytes[base + 19],
        ]) as usize;
        if p_filesz == 0 || p_paddr >= 0x0080_0000 {
            continue;
        }
        let file_end = p_offset.saturating_add(p_filesz);
        let flash_end = p_paddr.saturating_add(p_filesz);
        if file_end > elf_bytes.len() || flash_end > max_bytes {
            return Err(NodusError::Circuit(
                "ELF PT_LOAD segment exceeds bounds".into(),
            ));
        }
        flash[p_paddr..flash_end].copy_from_slice(&elf_bytes[p_offset..file_end]);
        loaded_any = true;
    }
    if !loaded_any {
        return Err(NodusError::Circuit(
            "ELF contains no loadable AVR flash segments".into(),
        ));
    }
    Ok(flash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_intel_hex_and_rejects_bad_checksum_or_missing_eof() {
        let hex = ":040000000C94340028\n:00000001FF\n";
        let flash = parse_intel_hex(hex, 64).unwrap();
        assert_eq!(&flash[0..4], &[0x0C, 0x94, 0x34, 0x00]);
        assert!(parse_intel_hex(":040000000C94340000\n:00000001FF\n", 64).is_err());
        assert!(parse_intel_hex(":040000000C94340028\n", 64).is_err());
    }
}
