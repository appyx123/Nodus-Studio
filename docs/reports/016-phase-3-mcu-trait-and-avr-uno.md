# 016 — Fase 3: Trait `Microcontroller` dan Emulator `avr_uno.rs`

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Antigravity (Coding)
- **Fase dan langkah blueprint:** Fase 3, Langkah 3 (Trait `Microcontroller`, `avr_uno.rs`, dan integrasi `SimulationRuntime`)
- **Status subtahap:** completed
- **Status fase:** in-progress

## Tujuan dan batas cakupan

Membangun kontrak `Microcontroller` yang agnostik perangkat keras di `src-tauri/src/mcu/traits.rs`, mengimplementasikan emulator ATmega328P @ 16 MHz (`AvrUno`) di `src-tauri/src/mcu/avr_uno.rs` menggunakan `avr-emulator = "=0.1.2"`, serta menyinkronkan eksekusi siklus MCU dengan `Scheduler`, `CircuitState`, dan `SerialBuffer` di `SimulationRuntime` (`src-tauri/src/engine/runtime.rs`) dan `SimulationController` (`src-tauri/src/ipc/simulation.rs`). Pembuatan paket `components-registry/mcu-arduino-uno.nodpkg` dan perluasan skema manifest dilakukan pada Langkah 4.

## Hasil dan perubahan

- `src-tauri/src/mcu/traits.rs`:
  - Trait `Microcontroller` (`backend_id`, `load_hex`, `load_elf`, `reset`, `step_cycles`, `set_input_pin`, `pin_drives`, `drain_serial_tx`, `push_serial_rx`, `total_cycles`).
  - Parser native safe Rust `parse_intel_hex` (rekord `00`, `01`, `02`, `04`, validasi checksum dan batas ukuran flash) dan `parse_avr_elf` (validasi header ELF32 little-endian `EM_AVR = 0x53` dan segmen `PT_LOAD`).
- `src-tauri/src/mcu/avr_uno.rs`:
  - `AvrUno` berbasis `avr-emulator 0.1.2` (`Profile::avre_plus()` 16-bit PC) dengan flash 32 KiB dan SRAM + ruang I/O ATmega328P.
  - Pemodelan GPIO Port B/C/D (`PINx`/`DDRx`/`PORTx` beserta semantik toggle tulis-1-ke-`PINx`) yang dipetakan ke pin Arduino Uno `D0..D13` dan `A0..A5`.
  - Pemodelan Timer0 (prescaler $1, 8, 64, 256, 1024$, mode Normal/CTC/PWM, flag `TIFR0`, dan interupsi vektor 14/15/16) untuk mendukung `millis()`, `micros()`, dan `delay()`, serta fase PWM untuk Timer0/1/2 (`analogWrite()` pada `D3`, `D5`, `D6`, `D9`, `D10`, `D11`).
  - Pemodelan USART0 (`UDR0`, `UCSR0A/B/C`, `UDRE0`, `TXC0`, `RXC0`, vektor interupsi 18/19) untuk komunikasi serial TX/RX dengan `HardwareSerial` Arduino.
- `src-tauri/src/engine/runtime.rs` & `src-tauri/src/ipc/simulation.rs`:
  - Menambahkan `attach_mcu` dan `advance_with_serial` pada `SimulationRuntime` yang menjalankan slice siklus 16 MHz (`1_600` siklus = `100 µs` per slice) selaras dengan `SimTime` (`125 half-ns/siklus`), menyinkronkan pin input/output dengan `CircuitState`, memicu `InputEdge` untuk komponen Lua, dan mengalirkan byte UART ke/dari `SerialBuffer`.

## Keputusan teknis

1. **Kompensasi bug instruksi `avr-emulator 0.1.2` di `avr_uno.rs`**:
   - Pada `avr-emulator 0.1.2`, `OpcodeKind::Subi` (`0x5000..0x5FFF`) memanggil `sub_immediate(..., false)` yang tetap membaca `FLAG_C` sebagai borrow (layaknya `SBCI`). `AvrUno::step_cycles` membersihkan `FLAG_C` sebelum mengeksekusi opcode `SUBI` sehingga tabel pointer `PGM` Arduino (`digitalWrite`/`pinMode`) menghitung alamat dengan tepat.
   - Dekoder `avr-emulator 0.1.2` memfilter instruksi unary `0x9500..0x95FF` (`COM`, `NEG`, `SWAP`, `INC`, `ASR`, `LSR`, `ROR`, `DEC`) dengan mask `0xfe0f` yang mengasumsikan bit `rd` tertinggi 0 (`r0..r15`). `AvrUno::step_cycles` memetakan eksekusi unary `r16..r31` melalui `r0` secara transparan tanpa mengubah citra flash permanen.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Seluruh file Rust terformat rapi dan ≤ 400 baris |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 74 passed, 0 failed (termasuk kompilasi sketch nyata via `arduino-cli` dan verifikasi Blink `D13`, PWM 25% di `D9`, input `D2`, serta TX/RX `Serial`) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Selesai tanpa warning |

## Hambatan dan risiko

- Belum ada paket `.nodpkg` untuk Arduino Uno di `components-registry/` hingga Langkah 4 dijalankan.

## Handoff: langkah berikutnya

1. Lanjutkan Fase 3 Langkah 4: perluas skema manifest `.nodpkg` (`src-tauri/src/sandbox/manifest.rs` dan `src/types/nodpkg.ts`) secara backward-compatible dengan deklarasi backend MCU (`[mcu]` / `avr_atmega328p`), buat `components-registry/mcu-arduino-uno.nodpkg`, dan jalankan verifikasi end-to-end (Rust, Node, WebKit DOM smoke, dan GNOME desktop smoke).

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
