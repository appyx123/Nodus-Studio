# Nodus Studio — Progress & Handoff

## Mulai dari sini

Baca `AGENTS.md`, blueprint, lalu laporan terbaru sebelum melanjutkan. Blueprint tetap menjadi sumber kebenaran persyaratan; file ini merangkum keadaan implementasi.

- **Fase aktif:** Fase 3 — Integrasi Editor Kode, `arduino-cli`, & Emulasi MCU Pertama (`in-progress`).
- **Subtahap terakhir:** Fase 3 Langkah 3 — trait `Microcontroller`, `avr_uno.rs`, dan integrasi `SimulationRuntime` (`completed`).
- **Status kesiapan build:** frontend dan native debug `completed`; release build `not-started`.
- **Status Fase 1:** `completed` (seluruh deliverable dan kriteria kelulusan otomatis Fase 1 telah memiliki bukti uji; pemeriksaan input fisik manual dicatat `not-run`).
- **Laporan terbaru:** [016 — Trait `Microcontroller` dan emulator `avr_uno.rs`](docs/reports/016-phase-3-mcu-trait-and-avr-uno.md).
- **Repository:** https://github.com/appyx123/Nodus-Studio.git — branch `main`, upstream `origin/main`.
- **Langkah berikutnya:** Fase 3 Langkah 4 — perluasan skema manifest `.nodpkg` untuk MCU, paket `mcu-arduino-uno.nodpkg`, dan verifikasi end-to-end (`not-started`).

## Ringkasan roadmap

| Fase | Status | Bukti / catatan |
| --- | --- | --- |
| 1 — Fondasi dan kanvas | `completed` | Scaffold Tauri v2 + SolidJS, loader `.nodpkg` v1, katalog, kanvas pan/zoom/snap/drag, `WireLayer.tsx` Manhattan, dan netlist Rust (`circuit_graph.rs`) lulus 56 tes Rust, 24 tes Node, WebKit DOM smoke, dan native GNOME/Wayland smoke |
| 2 — Mesin simulasi dan Lua | `completed` | `SimTime` half-ns, `Scheduler`, resolusi net digital, sandbox Lua 5.4 (`mlua`), streamer biner `NDFR` 60 Hz, `basic-power-source`/`basic-pushbutton`/`basic-led`, lulus 70 tes Rust, 26 tes Node, WebKit DOM smoke, dan native GNOME/Wayland smoke |
| 3 — Editor, compiler, dan AVR | `in-progress` | Langkah 1–3 selesai: spike backend AVR (`avr-emulator 0.1.2`), `arduino_cli.rs`, `uart.rs`, `CodeEditor.tsx`, `SerialMonitor.tsx`, `traits.rs`, dan `avr_uno.rs` lulus 74 tes Rust dan 26 tes Node |
| 4 — Sensor, bus, dan MQTT | `not-started` | Menunggu Fase 3 selesai dan teruji |
| 5 — Penyimpanan dan distribusi | `not-started` | Menunggu Fase 4 selesai dan teruji |

## Implementasi yang tersedia

- Kontrak subset v1: [`docs/nodpkg-v1.md`](docs/nodpkg-v1.md) dan fixture normalisasi bersama [`docs/nodpkg-v1-led.json`](docs/nodpkg-v1-led.json). Schema/API v1; deklarasi slider dan visual opacity numerik saja.
- Rust sandbox & loader: manifest/loader/validator SVG di `src-tauri/src/sandbox/`, fixture `components-registry/basic-led.nodpkg/` lengkap. **Lua tidak dieksekusi atau dikirim ke frontend pada Fase 1.**
- Rust circuit engine: `src-tauri/src/engine/{circuit_graph.rs,types.rs}` dan IPC `list_components`, `get_circuit`, `edit_circuit` di `src-tauri/src/ipc/`. Menyimpan instance (`node-<UUID>`), kabel (`wire-<UUID>`), warna kabel (`green`, `blue`, `amber`, `red`, `violet`), dan netlist terhubung secara authoritative di backend (maksimal 256 instance, 1.024 kabel). Menghapus instance otomatis menghapus kabel terkait dan memperbarui net.
- Katalog & Kanvas: `src/components/library/ComponentCatalog.tsx`, `src/components/canvas/{Workbench.tsx,NodeRenderer.tsx,WireLayer.tsx,graphic.ts,geometry.ts}`, dan `src/store/circuitStore.ts`. Mendukung beberapa instance dengan SVG ID ter-namespace (`<instance_id>--<original_id>`), pan/zoom (0.2–4), snap-to-grid 25.4 unit (2.54 mm), drag/drop, klik pin A ke pin B, routing kabel Manhattan, ganti warna kabel, seleksi, keyboard nudge, serta hapus kabel/instance.
- Kontrak koordinat: `screen = translation + world * zoom`; posisi wereld pin = `instance_position + local_pin - viewBox_origin`.
- Capability lokal `main` hanya mengizinkan `allow-list-components`, `allow-get-circuit`, dan `allow-edit-circuit`; tanpa argumen path frontend, filesystem/shell plugin, atau remote capability.

## Lingkungan dan verifikasi terakhir

Fedora Workstation 45 x86_64, GNOME/Wayland; Rust/Cargo 1.99.1 dari Fedora, rustfmt 1.10.0, Clippy 0.1.99. Native dependencies tersedia: WebKitGTK 2.54.2, GTK3 3.24.52, OpenSSL 4.0.3, librsvg 2.63.2, AppIndicator, dan libxdo. Node 22.23.1, npm 10.9.8. System Python memiliki `pyatspi` dan `gi` GTK3/WebKit2 4.1 untuk skrip uji.

| Pemeriksaan | Hasil | Cakupan |
| --- | --- | --- |
| `npm test` | passed | 24 tes Node: konfigurasi/permissions, kontrak manifest + circuit topology TypeScript, serta 17 tes geometri/pin/Manhattan |
| `npm run build` | passed | Typecheck TypeScript dan Vite produksi |
| `python3 scripts/webkit-workbench-smoke.py` | passed | 12 checks interaksi/geometri/hit-test/Manhattan wiring/recolor/endpoint move/cascading delete dengan mock IPC + 1 browser fallback |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 56 tes Rust (34 loader/manifest/SVG + 22 circuit netlist & IPC) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Semua target tanpa warning |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --no-bundle --debug` | passed | Native debug build (`src-tauri/target/debug/nodus`) |
| `python3 scripts/desktop-catalog-smoke.py` | passed | Native resource/IPC, dua Add, pin wiring A ke K, netlist merge, ganti warna kabel, zoom/reset, hapus kabel, hapus instance, isolasi paket invalid, Reload dan error recovery |
| Source limits, tautan lokal/anchor dokumentasi, dan `git diff --check` | passed | Semua file kode ≤400 baris |
| Input fisik/trusted pointer capture, touch/trackpad, pixel QA, stress 256 instance / 1.024 kabel | not-run | Tidak diklaim tercakup oleh event sintetik/accessibility |
| Release/RPM/Flatpak dan compiler minimum | not-run | Di luar cakupan Fase 1 |

## Hambatan dan risiko

1. Tidak ada blocker aktif untuk Fase 2. Fase 1 hanya mengelola topologi sesi; belum ada simulasi waktu, eksekusi Lua, atau resolusi listrik.
2. Input fisik drag/drop, pointer release di luar canvas, trusted capture/lost capture, touch/trackpad dan visual piksel tetap tercatat `not-run` untuk pengujian manual.
3. Topologi rangkaian masih bersifat sesi (hilang saat aplikasi ditutup); penyimpanan proyek `.nodus` berada di Fase 5.
4. Loader menolak symlink dan membatasi ukuran baca/payload, dengan final-open `O_NOFOLLOW | O_NONBLOCK` di Unix.

## Tindakan berikutnya

1. Mulai Fase 2 Langkah 1: tetapkan representasi waktu bebas drift untuk $16\text{ MHz}$ ($62.5\text{ ns}$ per siklus) dan kontrak model listrik digital (`HIGH`, `LOW`, `HIGH_Z`, `UNKNOWN`, push-pull, open-drain, input, pull-up/down), lalu implementasikan `scheduler.rs`, `pin.rs`, dan resolusi net di `circuit_graph.rs`.
2. Lanjutkan Fase 2 Langkah 2–4: `lua_host.rs` (`mlua` sandbox), `frame_streamer.rs` (snapshot biner 60 Hz), serta paket `basic-power-source.nodpkg`, `basic-pushbutton.nodpkg`, dan perilaku Lua `basic-led.nodpkg`.
3. Buat laporan `010` di `docs/reports/` menggunakan template dan perbarui `PROGRESS.md`.

## Laporan

- [001 — Fase 1: pemeriksaan lingkungan dan aturan laporan](docs/reports/001-phase-1-environment.md)
- [002 — Fase 1: scaffold aplikasi dan hambatan dependency](docs/reports/002-phase-1-application-scaffold.md)
- [003 — Fase 1: verifikasi prasyarat native dan timeout build](docs/reports/003-phase-1-native-build-attempt.md)
- [004 — Fase 1: build native debug dan smoke test desktop](docs/reports/004-phase-1-native-build-smoke.md)
- [005 — Fase 1: kontrak paket, loader, dan katalog](docs/reports/005-phase-1-component-loader.md)
- [006 — Fase 1: publikasi repository GitHub](docs/reports/006-phase-1-github-publication.md)
- [007 — Fase 1: penyegaran README open source](docs/reports/007-phase-1-readme-refresh.md)
- [008 — Fase 1: kanvas interaktif dan instance komponen](docs/reports/008-phase-1-interactive-workbench.md)
- [009 — Fase 1: kabel Manhattan dan netlist authoritative Rust](docs/reports/009-phase-1-wiring-and-netlist.md)
- [010 — Fase 2: scheduler deterministik dan resolusi net digital](docs/reports/010-phase-2-scheduler-and-net-resolution.md)
- [011 — Fase 2: sandbox Lua 5.4 dan kontrak API host v1](docs/reports/011-phase-2-lua-sandbox-host.md)
- [012 — Fase 2: protokol frame biner dan frame streamer](docs/reports/012-phase-2-binary-frame-streamer.md)
- [013 — Fase 2: komponen bawaan dan runtime simulasi](docs/reports/013-phase-2-builtin-components-and-simulation-runtime.md)
- [014 — Fase 3: spike evaluasi backend emulator AVR](docs/reports/014-phase-3-avr-backend-spike.md)
- [015 — Fase 3: integrasi editor kode, Serial Monitor, dan `arduino_cli.rs`](docs/reports/015-phase-3-editor-serial-and-arduino-cli.md)
- [016 — Fase 3: trait `Microcontroller` dan emulator `avr_uno.rs`](docs/reports/016-phase-3-mcu-trait-and-avr-uno.md)

Template: [`docs/reports/TEMPLATE.md`](docs/reports/TEMPLATE.md). Nomor laporan berikutnya: `017` (periksa ulang sebelum membuat file).
