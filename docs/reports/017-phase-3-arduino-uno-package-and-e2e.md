# 017 — Fase 3: Paket `mcu-arduino-uno.nodpkg`, Kontrak MCU, dan Validasi E2E

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Antigravity (Coding)
- **Fase dan langkah blueprint:** Fase 3, Langkah 4 (Paket `mcu-arduino-uno.nodpkg`, skema manifest MCU, dan verifikasi integrasi end-to-end)
- **Status subtahap:** completed
- **Status fase:** completed

## Tujuan dan batas cakupan

Menyelesaikan Fase 3 Langkah 4:
1. Memperluas skema manifest `.nodpkg` v1 secara backward-compatible (`src-tauri/src/sandbox/manifest.rs` dan `src/types/nodpkg.ts`) dengan blok opsional `[mcu]` (`backend = "avr_atmega328p"`).
2. Membangun paket komponen `components-registry/mcu-arduino-uno.nodpkg` (`manifest.toml`, `graphic.svg`, `logic.lua`) dengan 22 pin grid-aligned (2.54 mm), terminal daya VCC/GND, header I/O digital/analog, dan visual LED built-in `L` (`D13`).
3. Mengintegrasikan eksekusi sketch nyata (`arduino-cli`) melalui `SimulationRuntime` dan membuktikan rangkaian MCU mengendalikan `basic-led` dan membaca input `basic-pushbutton`.
4. Menjalankan seluruh rangkaian verifikasi otomatis Rust, TypeScript/Node, WebKit DOM smoke, dan GNOME/Wayland AT-SPI desktop smoke.

## Hasil dan perubahan

- `src-tauri/src/sandbox/manifest.rs` & `src/types/nodpkg.ts`:
  - Menambahkan `McuConfig` dan `McuBackend::AvrAtmega328p` ke struktur `Manifest` menggunakan `#[serde(default, skip_serializing_if = "Option::is_none")]`.
  - Memperbarui tipe TypeScript `McuBackend = "avr_atmega328p"`, `McuConfig`, dan properti opsional `mcu?: McuConfig` pada `ComponentManifest`.
- `components-registry/mcu-arduino-uno.nodpkg/`:
  - `manifest.toml`: Deklarasi komponen Arduino Uno R3, `[mcu] backend = "avr_atmega328p"`, 22 pin (`VCC`, `GND`, `D0`..`D13`, `A0`..`A5`), dan visual `led_builtin` (`dyn-led-l`, opacity `0.15..1.0`).
  - `graphic.svg`: Layout visual PCB Arduino Uno R3 dengan header pin grid-aligned 2.54 mm, chip ATmega328P, port USB, dan indikator onboard LED `L` (`dyn-led-l`).
  - `logic.lua`: Skrip Lua visual menginisialisasi rel VCC (HIGH) dan GND (LOW), serta menyinkronkan visual `led_builtin` mengikuti logika pin `D13`.
- `src-tauri/src/ipc/simulation.rs`:
  - `attach_firmware`: Memuat artefak firmware (`.hex` / `.elf`) hasil kompilasi ke instance MCU yang terdaftar pada circuit saat simulasi dimulai.
- `src-tauri/src/engine/runtime_tests.rs`:
  - Menambahkan pengujian integrasi runtime penuh: `arduino_uno_drives_external_led_and_reads_pushbutton_in_runtime` menguji `mcu-arduino-uno` menjalankan sketch Arduino (`pinMode(13, OUTPUT); pinMode(2, INPUT); digitalWrite(13, digitalRead(2));`), membaca pushbutton eksternal pada D2, dan mengendalikan brightness `basic-led` pada D13.
  - Memperbarui ekspektasi jumlah paket registry dari 3 menjadi 4.
- `src-tauri/src/sandbox/pkg_loader_tests.rs`:
  - Memperbarui daftar paket bawaan terurut (`basic-led`, `basic-power-source`, `basic-pushbutton`, `mcu-arduino-uno`).
- `scripts/nodpkg-contract.test.mjs`:
  - Menambahkan pengujian kontrak TypeScript untuk validasi `mcu: { backend: "avr_atmega328p" }` dan penolakan backend yang tidak dikenal (`TS2322`).
- `scripts/desktop-catalog-smoke.py`:
  - Menyesuaikan asersi katalog yang dimuat dari 3 menjadi 4 paket.

## Keputusan teknis

1. **Perluasan Skema Manifest Tanpa Merusak v1**:
   - `Option<McuConfig>` dengan `#[serde(default, skip_serializing_if = "Option::is_none")]` menjamin file JSON/fixture kontrak sebelumnya (`docs/nodpkg-v1-led.json`) tetap 100% identik dan valid tanpa memerlukan mutasi field `mcu`.
2. **Penyelarasan Siklus Periferal dengan Edge Netlist**:
   - Firmware Arduino yang membaca `digitalRead(2)` segera mendeteksi perubahan level saat `set_control("pressed", 1.0)` mengalirkan tegangan dari terminal VCC Uno melalui pushbutton ke pin D2, menggerakkan output D13 ke HIGH, dan diteruskan ke pin Anoda `basic-led`.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `npm test` | passed | 27 passed (5 kontrak nodpkg TypeScript/Rust, 22 geometri/routing/layout) |
| `npm run build` (`npm run typecheck && vite build`) | passed | TypeScript lolos, bundle Vite selesai tanpa error |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Seluruh kode Rust terformat rapi dan semua file ≤ 400 baris |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 75 passed, 0 failed (termasuk emulasi MCU end-to-end, compiler, scheduler, parser HEX/ELF, sandbox Lua, parser SVG) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Selesai tanpa warning |
| `/usr/bin/python3 scripts/webkit-workbench-smoke.py` | passed | 13 cek DOM WebKitGTK lolos (mocked discovery & browser fallback) |
| `npx tauri build --debug --no-bundle && /usr/bin/python3 scripts/desktop-catalog-smoke.py` | passed | 20 cek AT-SPI GNOME/Wayland native binary lolos (4 paket loaded, runtime add, wiring, zoom, simulation stream, reload registry) |

## Hambatan dan risiko

- Tidak ada blocker. Semua kriteria penerimaan Fase 3 telah diverifikasi dengan bukti uji konkret.

## Handoff: langkah berikutnya

1. Fase 3 telah selesai sepenuhnya (`completed`).
2. Tahap berikutnya adalah memulai Fase 4 (Debugging Interaktif & Analisis Sirkuit) sesuai blueprint, mencakup instrumen virtual (Logic Analyzer, Oscilloscope), breakpoint firmware/kondisional net, inspeksi memori/register ATmega328P, dan visualisasi timing waveform digital.

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
