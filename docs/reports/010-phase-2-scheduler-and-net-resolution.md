# 010 — Fase 2: Scheduler Deterministik dan Resolusi Net Digital

- **Tanggal:** 2026-11-06
- **Agent / pelaksana:** Antigravity coding agent
- **Fase dan langkah blueprint:** Fase 2, Langkah 1 — representasi waktu presisi, `scheduler.rs`, `pin.rs`, dan resolusi state net pada `circuit_graph.rs`
- **Status subtahap:** `completed`
- **Status fase:** `in-progress` (Langkah 1 selesai; Langkah 2–4 menyusul)

## Tujuan dan batas cakupan

Membangun fondasi waktu presisi tanpa drift untuk MCU $16\text{ MHz}$ ($1\text{ siklus} = 62.5\text{ ns}$), scheduler event deterministik dengan siklus hidup (`start`, `pause`, `resume`, `stop`, `reset`), abstraksi mode pin (`Input`, `InputPullup`, `InputPulldown`, `Output`, `OpenDrain`), serta resolusi net digital diskret (`HIGH`, `LOW`, `HIGH_Z`, `UNKNOWN`) di `circuit_graph.rs`. Belum mengeksekusi Lua atau mengirim frame biner.

## Hasil dan perubahan

- `src-tauri/src/engine/scheduler.rs`: tipe `SimTime` berbasis unit setengah-nanodetik (`0.5 ns`, sehingga 1 siklus $16\text{ MHz} = 125\text{ unit}$ tepat tanpa drift), antrean event deterministik (`BTreeMap<(SimTime, u64), ScheduledEvent>`), pembatalan timer individual maupun per-instance, proteksi antrean (maks 4.096 event) dan deteksi loop tanpa kemajuan waktu (maks 1.024 event bertimestamp sama), serta transisi siklus hidup `Stopped`/`Running`/`Paused`.
- `src-tauri/src/engine/pin.rs`: enum `LogicLevel` (`HIGH`, `LOW`, `HIGH_Z`, `UNKNOWN`), `PinMode` (`INPUT`, `INPUT_PULLUP`, `INPUT_PULLDOWN`, `OUTPUT`, `OPEN_DRAIN`), `PinRuntime`, dan fungsi `resolve_net` yang membedakan penggerak kuat, open-drain, pull lemah, floating (`HIGH_Z`), serta konflik output (`UNKNOWN` + daftar driver konflik).
- `src-tauri/src/engine/circuit_graph.rs`: penyimpanan state runtime pin per instance, metode `set_pin_mode`, `set_pin_output`, `release_instance_pins`, `reset_all_pins`, `resolve_nets`, dan `read_pin`.
- `src-tauri/src/engine/simulation_tests.rs`: 4 pengujian komprehensif untuk pembuktian $16.000.000\text{ siklus} = 1\text{ detik}$ virtual tanpa drift (termasuk pelestarian sisa $0.5\text{ ns}$ pada siklus ganjil), urutan stabil FIFO pada timestamp identik, siklus hidup scheduler & deteksi zero-advance loop, serta resolusi net digital (floating, pull-up/down, open-drain, push-pull conflict, dan pelepasan instance).

## Keputusan teknis

- Representasi waktu internal menggunakan integer `u64` dalam satuan $0.5\text{ ns}$ (`SimTime`). Konversi ke `_ns` memakai pembulatan ke bawah (`as_ns_floor()`) hanya saat diekspos ke API nanodetik, tanpa mengubah akumulator internal.
- Net tanpa driver maupun resistor pull menghasilkan `HIGH_Z` (mengambang), bukan `LOW`. Konflik antara driver kuat `HIGH` dan `LOW` menghasilkan `UNKNOWN` dengan `conflict = true`.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust bersih |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 60 tes Rust lulus (56 tes Fase 1 + 4 tes scheduler/net Fase 2) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Bebas warning di seluruh target |

## Hambatan dan risiko

- Tidak ada blocker untuk Langkah 2 (integrasi sandbox Lua).

## Handoff: langkah berikutnya

1. Lanjutkan ke Fase 2 Langkah 2: integrasi sandbox Lua (`src-tauri/src/sandbox/lua_host.rs`) dengan pembatasan memori, budget instruksi, dan kontrak API `nodus.*`.

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
