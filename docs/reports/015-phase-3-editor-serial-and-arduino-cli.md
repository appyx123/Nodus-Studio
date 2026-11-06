# 015 — Fase 3: Integrasi Editor Kode, Serial Monitor, dan `arduino_cli.rs`

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Antigravity (Coding)
- **Fase dan langkah blueprint:** Fase 3, Langkah 2 (Integrasi editor kode, Serial Monitor, dan `arduino_cli.rs`)
- **Status subtahap:** completed
- **Status fase:** in-progress

## Tujuan dan batas cakupan

Menginstal `arduino-cli` dan core `arduino:avr` di user-space sesuai persetujuan pengguna, membangun eksekutor `src-tauri/src/compiler/arduino_cli.rs` (argumen subprocess terstruktur tanpa interpolasi shell, workspace sementara terisolasi, timeout/pembatalan, dan pemetaan error kompilasi ke baris/kolom editor), menambahkan buffer UART real-time di `src-tauri/src/engine/bus/uart.rs`, serta mengintegrasikan `CodeEditor.tsx`, `SerialMonitor.tsx`, dan `editorStore.ts` ke dalam `Workbench.tsx`.

## Hasil dan perubahan

- Lingkungan user-space: menginstal `arduino-cli` `1.5.1` (`~/.local/bin/arduino-cli`, commit `01f3d4f2b`) dan platform `arduino:avr@1.8.8` (`avr-gcc@7.3.0-atmel3.6.1-arduino7`).
- Backend Rust:
  - `src-tauri/src/compiler/{mod.rs,arduino_cli.rs}`: `CompilerController`, `TempWorkspace` terisolasi (`0o700`), timeout (`30_000 ms` default), cancellation atomik, dan `parse_compiler_diagnostics`.
  - `src-tauri/src/engine/bus/{mod.rs,uart.rs}`: `SerialBuffer` (`MAX_TX_BUFFER_BYTES = 64 KiB`, `MAX_RX_QUEUE_BYTES = 4 KiB`) untuk antrean TX/RX UART.
  - `src-tauri/src/ipc/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/main.json`: mendaftarkan command IPC `compile_sketch`, `cancel_compilation`, dan `serial_io`.
- Frontend SolidJS:
  - `src/store/editorStore.ts`: state reaktif untuk kode sketch, status kompilasi, diagnostik baris/kolom, dan polling Serial Monitor.
  - `src/components/editor/CodeEditor.tsx` & `src/components/editor/SerialMonitor.tsx`: panel editor kode C++/Arduino dengan line gutter bertanda error, daftar diagnostik (`role="alert"`), terminal output UART, dan form input RX.
  - `src/components/canvas/Workbench.tsx` & `src/styles.css`: tombol toggle `"Firmware & Serial"` dan layout drawer kanan.

## Keputusan teknis

1. **Subprocess terstruktur tanpa shell**: `Command::new(&cli).args(["compile", "--no-color", "--fqbn", fqbn, "--output-dir", ...])` dipanggil langsung dengan `Stdio::null()` pada stdin dan batas baca output `256 KiB`.
2. **Isolasi temp workspace**: setiap kompilasi membuat direktori unik `nodus-sketch-<pid>-<seq>` bermode `0o700` di `std::env::temp_dir()` dan menghapusnya otomatis melalui `Drop` baik ketika sukses, gagal, maupun timeout/dibatalkan.
3. **Kepatuhan CSP produksi**: permukaan editor kode tidak menyuntikkan `<style>` inline sehingga CSP ketat `style-src 'self'` di `tauri.conf.json` tetap terjaga.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `npm test` | passed | 26 passed, 0 failed |
| `npm run build` | passed | `tsc --noEmit` + Vite build sukses |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust bersih |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 72 passed (termasuk kompilasi sketch nyata dan error baris 3 melalui `arduino-cli 1.5.1`) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Tanpa warning |

## Hambatan dan risiko

- Artefak `.hex` dan `.elf` yang dihasilkan `CompilerController` sudah disimpan di memori, namun belum dihubungkan ke eksekusi siklus `avr_uno.rs` sampai Langkah 3 selesai.

## Handoff: langkah berikutnya

1. Lanjutkan Fase 3 Langkah 3: implementasikan `src-tauri/src/mcu/{mod.rs,traits.rs,avr_uno.rs}` menggunakan `avr-emulator = "=0.1.2"` (parser `.hex`/`.elf`, GPIO Port B/C/D, Timer0/1/2 + PWM, USART0 TX/RX, reset) dan integrasikan dengan `SimulationRuntime` serta `Scheduler`.
2. Lanjutkan Fase 3 Langkah 4: perluas kontrak manifest `.nodpkg` untuk deklarasi backend MCU secara backward-compatible dan buat `components-registry/mcu-arduino-uno.nodpkg`.

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
