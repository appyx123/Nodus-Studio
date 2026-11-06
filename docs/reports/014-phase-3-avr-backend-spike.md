# 014 — Fase 3: Spike Evaluasi Backend Emulator AVR

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Antigravity (Coding)
- **Fase dan langkah blueprint:** Fase 3, Langkah 1 (Spike backend AVR)
- **Status subtahap:** completed
- **Status fase:** in-progress

## Tujuan dan batas cakupan

Mengevaluasi kandidat wrapper `simavr` (`simavr-sys 1.5.9`, `simavr-ffi 1.2.7`, `avr-simulator 0.6.4`) terhadap persyaratan Fase 3 Nodus Studio (load `.elf`/`.hex`, stepping siklus, GPIO, timer/PWM, UART, reset, integrasi `SimTime`/`Scheduler`, lisensi/redistribusi MIT, dependensi native Fedora 45, dan keamanan FFI), meminta persetujuan pengguna ketika kandidat `simavr` gagal memenuhi syarat, serta mengunci crate backend pengganti yang disetujui. Implementasi `arduino_cli.rs`, Monaco Editor, dan `avr_uno.rs` dikerjakan pada Langkah 2–3.

## Hasil dan perubahan

- Mengevaluasi arsip crate `simavr-sys 1.5.9`, `simavr-ffi 1.2.7`, `avr-simulator 0.6.4`, dan `avr-emulator 0.1.2` pada Fedora Workstation 45.
- Mendokumentasikan hasil spike dan arsitektur pengganti pada `docs/avr-backend-spike.md`.
- Menambahkan `avr-emulator = "=0.1.2"` pada `src-tauri/Cargo.toml` dan `src-tauri/Cargo.lock` setelah mendapat persetujuan eksplisit pengguna.

## Keputusan teknis

1. **Penolakan `simavr-sys` dan `simavr-ffi`**:
   - Bundel C `simavr` berlisensi **GPL-3.0** (`vendor/simavr/COPYING`), bertabrakan dengan lisensi **MIT** repositori saat di-link statis (`rustc-link-lib=static=simavr`).
   - `simavr-ffi 1.2.7` memaksa `rustc-link-lib=static=elf` (`libelf.a` statis tidak tersedia pada paket standar `elfutils-libelf-devel` Fedora 45) serta membutuhkan `libclang` (`bindgen`) dan `avr-gcc` saat kompilasi.
   - Core C `simavr` memakai state global dan dapat memanggil `exit(1)` pada input ELF yang rusak.
2. **Persetujuan `avr-emulator = "=0.1.2"` + periferal Rust `avr_uno.rs`**:
   - Pengguna menyetujui penggunaan `avr-emulator 0.1.2` (BSD-3-Clause, 100% safe Rust, `no_std`, nol dependensi C) dengan profil `Profile::avre_plus().with_program_counter_bits(16)` untuk CPU ATmega328P dan pemodelan periferal (GPIO Port B/C/D, Timer0/1/2 + PWM, USART0, parser `.hex`/`.elf`) di `src-tauri/src/mcu/avr_uno.rs`.
3. **Persetujuan instalasi `arduino-cli` + `arduino:avr` di user-space**:
   - Pengguna menyetujui pengunduhan binary `arduino-cli` ke `~/.local/bin/arduino-cli` dan instalasi `arduino:avr` core untuk pengujian Langkah 2–4.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` | passed | 70 passed, 0 failed; `avr-emulator v0.1.2` terkompilasi bersih |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Selesai tanpa warning dengan `--locked --offline` |

## Hambatan dan risiko

- Tidak ada blocker aktif setelah persetujuan pengguna untuk backend `avr-emulator` dan instalasi `arduino-cli` user-space.
- Cakupan periferal `avr_uno.rs` pada Fase 3 difokuskan pada GPIO (`DDRx`/`PORTx`/`PINx`), Timer0/1/2 (overflow/CTC/PWM untuk `millis()`, `delay()`, `analogWrite()`), dan USART0 (`Serial`); periferal TWI/I2C dan SPI dijadwalkan pada Fase 4.

## Handoff: langkah berikutnya

1. Lanjutkan Fase 3 Langkah 2: instal `arduino-cli` dan `arduino:avr` di user-space, buat `src-tauri/src/compiler/arduino_cli.rs` (argumen terstruktur tanpa shell interpolation, temp workspace terisolasi, timeout/cancellation, parser diagnostik baris/kolom), serta integrasikan Monaco Editor (`CodeEditor.tsx`), `SerialMonitor.tsx`, dan `editorStore.ts`.
2. Lanjutkan Fase 3 Langkah 3–4: implementasikan `traits.rs`, `avr_uno.rs`, dan `components-registry/mcu-arduino-uno.nodpkg`.

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
