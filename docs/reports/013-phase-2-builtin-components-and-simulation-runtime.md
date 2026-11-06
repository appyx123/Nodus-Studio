# 013 — Fase 2: Komponen Bawaan, Runtime Simulasi, dan Integrasi Kontrol End-to-End

- **Tanggal:** 2026-11-06
- **Agent / pelaksana:** Antigravity coding agent
- **Fase dan langkah blueprint:** Fase 2, Langkah 4 — `basic-power-source.nodpkg`, `basic-pushbutton.nodpkg`, aktivasi Lua `basic-led.nodpkg`, thread simulasi, dan kontrol UI
- **Status subtahap:** `completed`
- **Status fase:** `completed`

## Cakupan

Menyelesaikan Fase 2: menambahkan paket `basic-power-source.nodpkg` dan `basic-pushbutton.nodpkg`, mengaktifkan logika Lua `basic-led.nodpkg` (duty cycle / state net), mengintegrasikan `SimulationRuntime` dan `SimulationController` pada dedicated OS thread Rust, serta menghubungkan kontrol simulasi (`start`/`pause`/`resume`/`stop`/`reset`) dan visual 60 Hz di frontend.

## Perubahan

- `components-registry/basic-power-source.nodpkg/`: terminal `VCC` (HIGH) dan `GND` (LOW).
- `components-registry/basic-pushbutton.nodpkg/`: pin `IN`/`OUT`, kontrol `pressed`, visual `cap`, dan penerusan level `IN` ke `OUT` saat ditekan.
- `components-registry/basic-led.nodpkg/logic.lua`: menghitung kecerahan (`0.15..1.0`) dari state net (`A == HIGH` dan `K == LOW`) dan duty cycle jendela 5 ms dengan timer settle.
- `src-tauri/src/engine/{runtime.rs,runtime_tests.rs,circuit_graph.rs}`: `SimulationRuntime` memanggil `on_init`, mendeteksi transisi net (`InputEdge`), mengeksekusi timer/callback Lua, dan mereset pin saat shutdown.
- `src-tauri/src/ipc/simulation.rs`, `commands.rs`, `build.rs`, `capabilities/main.json`: `SimulationController` mengelola thread `nodus-sim`, mempublikasikan `SimulationFrame` ke `FrameStreamer`, dan menyediakan command `control_simulation` serta `set_control_value`. Edit topologi ditolak saat simulasi aktif.
- `src/store/simulationStore.ts`, `src/components/canvas/{SimulationBar.tsx,NodeRenderer.tsx,Workbench.tsx}`: tombol Start/Pause/Resume/Stop/Reset, slider/toggle kontrol komponen, dan pembaruan atribut visual SVG dari frame biner.
- `scripts/{desktop-catalog-smoke.py,scaffold.test.mjs}`: memperbarui uji izin capability dan memverifikasi siklus Start/Pause/Resume/Reset serta streaming biner pada aplikasi native GNOME/Wayland.

## Validasi

| Pemeriksaan | Hasil | Bukti / cakupan |
| --- | --- | --- |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust bersih |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 70 tes Rust lulus (termasuk rangkaian sumber–tombol–LED, PWM duty cycle, konflik pin, dan lifecycle thread) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Tanpa warning di semua target |
| `npm run typecheck && npm test` | passed | 26 tes Node lulus |
| `npm run tauri -- build --debug --no-bundle -- --locked --offline` | passed | Binary debug `src-tauri/target/debug/nodus` terbangun |
| `/usr/bin/python3 scripts/webkit-workbench-smoke.py` | passed | 12 DOM checks + 1 browser fallback |
| `/usr/bin/python3 scripts/desktop-catalog-smoke.py` | passed | 3 paket dimuat, wiring, Start/Pause/Resume/Reset simulasi native, isolasi & recovery registry |

## Batasan dan langkah berikut

- Input fisik manual dan QA piksel tetap `not-run`.
- Siap masuk ke Fase 3 (integrasi editor kode Monaco, `arduino-cli`, dan emulasi MCU AVR pertama).
