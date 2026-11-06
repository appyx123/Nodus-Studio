# 012 — Fase 2: Protokol Frame Biner dan Frame Streamer 60 Hz

- **Tanggal:** 2026-11-06
- **Agent / pelaksana:** Antigravity coding agent
- **Fase dan langkah blueprint:** Fase 2, Langkah 3 — protokol frame biner ber-versi dan `frame_streamer.rs`
- **Status subtahap:** `partial` (kode dan uji unit selesai; uji transport nyata Tauri/WebKitGTK `not-run`)
- **Status fase:** `in-progress`

## Cakupan

Protokol frame biner `NDFR` v1 (little-endian), encoder/decoder Rust, antrean berbatas (kapasitas 1, latest-state-wins, tidak memblokir thread simulasi), command Tauri `get_simulation_frame` (binary `tauri::ipc::Response`), dan decoder TypeScript.

## Perubahan

- `src-tauri/src/ipc/frame_streamer.rs`: header 32 byte (magic, versi, run_state, flags conflict/fault, seq u64, sim_time half-ns u64, speed_ratio f32, jumlah instance/net u16), payload visual per instance dan net. Batas: 256 instance, 64 visual/instance, 1024 net, 512 endpoint/net, ID <= 128 byte. Decoder menolak magic/versi salah, truncation, flag tidak valid, nilai non-finite, dan byte sisa. `FrameStreamer` menyimpan cache frame terakhir untuk resync.
- `src-tauri/src/ipc/commands.rs`, `lib.rs`: command `get_simulation_frame`, `FrameStreamer` di-manage Tauri.
- `src/store/frameDecoder.ts`: decoder TS murni (diuji Node). `src/store/simulationStore.ts`: polling 60 Hz, hanya menerapkan state visual dari Rust.
- `scripts/frame-decoder.test.mjs`: uji decoder terhadap layout Rust.
- `frame_streamer.rs` memakai `#![allow(dead_code)]` sementara; produsen frame (thread simulasi) dihubungkan di Langkah 4.

## Validasi

| Pemeriksaan | Hasil | Catatan |
| --- | --- | --- |
| `cargo fmt --check` | passed | |
| `cargo test --locked --offline` | passed | 67 tes (4 baru frame_streamer) |
| `cargo clippy --all-targets -D warnings` | passed | |
| `npm test` | passed | 26 tes termasuk 2 uji decoder |
| `npm run typecheck` | passed | |
| Transport nyata di Tauri/WebKitGTK | not-run | ditunda ke verifikasi Langkah 4 |

## Batasan dan langkah berikut

- Polling 60 Hz dari frontend (bukan push event); keputusan sederhana, dapat diubah bila WebKitGTK bermasalah.
- Langkah 4: thread simulasi menghasilkan frame, komponen registry baru, kontrol start/pause/resume/stop/reset, smoke test desktop.
