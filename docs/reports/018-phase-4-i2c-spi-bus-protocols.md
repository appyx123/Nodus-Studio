# 018 — Fase 4: Protokol Bus Standar (I2C & SPI) dan Emulasi Periferal ATmega328P

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Antigravity (Coding)
- **Fase dan langkah blueprint:** Fase 4, Langkah 1 (Protokol Bus Standar: I2C/Two-Wire dan SPI pada backend Rust serta periferal ATmega328P)
- **Status subtahap:** completed
- **Status fase:** in-progress

## Tujuan dan batas cakupan

Menyelesaikan Fase 4 Langkah 1:
1. Mengimplementasikan bus coordinator dan trait responder untuk protokol I2C (`I2cBus`, `I2cDevice`) dan SPI (`SpiBus`, `SpiDevice`) di `src-tauri/src/engine/bus/`.
2. Mengintegrasikan periferal perangkat keras ATmega328P untuk TWI/I2C (register `TWBR`, `TWSR` di `0xB9`, `TWDR`, `TWCR`, vektor interupsi TWI word `48`) dan SPI (register `SPCR`, `SPSR`, `SPDR` di `0x2E`) pada `AvrUno`.
3. Memastikan library standar Arduino AVR core (`Wire.h` dan `SPI.h`) berfungsi penuh tanpa patch kode library, termasuk start/stop condition, pengiriman alamat slave, ACK/NACK, buffer data, dan transmisi SPI sinkron.
4. Memvalidasi seluruh fungsionalitas dengan pengujian integrasi berbasis firmware nyata (`avr_uno_hardware_wire_and_spi_transfers_succeed`) dan menjaga seluruh file sumber Rust ≤ 400 baris.

## Hasil dan perubahan

- `src-tauri/src/engine/bus/i2c.rs`:
  - Menambahkan trait `I2cDevice` dengan metode `address()`, `on_write(&[u8])`, dan `on_read() -> u8`.
  - Mengimplementasikan `I2cBus` dengan manajemen transaksi master I2C: pengalamatan 7-bit (`sla_rw >> 1`), validasi slave ack/nack, antrean buffer baca/tulis, serta penanganan `start_transaction`, `write_byte`, `read_byte`, dan `stop_transaction`.
- `src-tauri/src/engine/bus/spi.rs`:
  - Menambahkan trait `SpiDevice` dengan metode transfer byte sinkron full-duplex `transfer(byte: u8) -> u8`.
  - Mengimplementasikan `SpiBus` dengan kemampuan `attach_device` dan `transfer(byte)`.
- `src-tauri/src/engine/bus/mod.rs`:
  - Mengekspor modul `i2c` dan `spi` beserta tipe publik `I2cBus`, `I2cDevice`, `SpiBus`, dan `SpiDevice`.
- `src-tauri/src/mcu/avr_space.rs`:
  - Mengekstrak penanganan ruang data dan register I/O dari `avr_uno.rs` ke `avr_space.rs` agar ukuran file tetap ≤ 400 baris.
  - Mengimplementasikan state machine periferal TWI ATmega328P sesuai spesifikasi datasheet Atmel dan implementasi `twi.c` Arduino core: pemetaan alamat register `TWSR` ke `0xB9`, pemrosesan perintah `TWCR` (`TWINT`, `TWSTA`, `TWSTO`, `TWEA`), transisi status `TW_START` (0x08), `TW_REP_START` (0x10), `TW_MT_SLA_ACK` (0x18), `TW_MT_DATA_ACK` (0x28), `TW_MR_SLA_ACK` (0x40), `TW_MR_DATA_ACK` (0x50), dan `TW_MR_DATA_NACK` (0x58).
  - Mengimplementasikan transfer SPI master sinkron pada register `SPDR` (`0x2E`) dan auto-flag `SPSR.SPIF` (bit 7).
  - Mengimplementasikan flag buffer USART RX `sync_rx_flag`.
- `src-tauri/src/mcu/avr_uno.rs`:
  - Menghubungkan interupsi TWI ATmega328P ke vektor 24 (word address 48).
  - Menyediakan akses `i2c_bus()` dan `spi_bus()` melalui trait `Microcontroller`.
- `src-tauri/src/mcu/traits.rs`:
  - Memperluas trait `Microcontroller` dengan accessor opsional `i2c_bus(&mut self)` dan `spi_bus(&mut self)`.
- `src-tauri/src/mcu/tests.rs`:
  - Menambahkan pengujian integrasi firmware `avr_uno_hardware_wire_and_spi_transfers_succeed` yang mengompilasi sketch Arduino `#include <Wire.h>` dan `#include <SPI.h>`, memverifikasi `Wire.beginTransmission()`, `Wire.write()`, `Wire.endTransmission() == 0`, `Wire.requestFrom()`, pembacaan respon byte I2C (`66`), dan `SPI.transfer(5) == 15`.

## Keputusan teknis

1. **Pemetaan Akurat Register TWI ATmega328P**:
   - Register `TWBR` berada pada alamat `0xB8`, sedangkan register status `TWSR` berada pada alamat `0xB9`. Memetakan `TWSR` secara tepat ke `0xB9` memperbaiki respon status machine `TW_STATUS` pada ISR `twi.c` sehingga tidak terjadi `TW_BUS_ERROR` (kode error 4).
2. **Modularisasi `avr_space.rs` untuk Batas 400 Baris**:
   - Memisahkan bus memori `IoSpace` dan `DataSpace` ke dalam `avr_space.rs` (179 baris) menjaga `avr_uno.rs` (305 baris) jauh di bawah batas wajib repository (≤ 400 baris).

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml avr_uno_hardware_wire_and_spi_transfers_succeed --locked --offline` | passed | Wire.h I2C write/read & SPI.h transfer terverifikasi (`I2C_ERR:0;I2C_VAL:66;SPI_RESP:15`) |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 76 passed, 0 failed (seluruh rangkaian tes Rust) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline -- -D warnings` | passed | Lolos tanpa warning |
| `npm test` | passed | 27 passed, 0 failed (semua tes kontrak & geometri) |
| `wc -l $(find src-tauri/src -name '*.rs')` | passed | Seluruh 42 file Rust berada di bawah batas 400 baris |

## Hambatan dan risiko

- Tidak ada blocker. Protokol I2C dan SPI telah berfungsi penuh dan terintegrasi dengan core MCU.

## Handoff: langkah berikutnya

1. Langkah 1 Fase 4 telah selesai (`completed`).
2. Melanjutkan ke Fase 4 Langkah 2: Pembuatan paket komponen sensor dan aktuator canggih:
   - `sensor-dht22.nodpkg` (sensor suhu dan kelembapan protokol digital 1-wire)
   - `sensor-hcsr04.nodpkg` (sensor jarak ultrasonik sinyal trigger/echo)
   - `actuator-servo-sg90.nodpkg` (motor servo kendali sinyal PWM)
