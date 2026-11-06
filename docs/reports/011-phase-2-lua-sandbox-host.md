# 011 — Fase 2: Sandbox Lua 5.4 dan Kontrak API Host v1

- **Tanggal:** 2026-11-06
- **Agent / pelaksana:** Antigravity coding agent
- **Fase dan langkah blueprint:** Fase 2, Langkah 2 — kontrak API Lua v1 dan sandbox per-instance pada `src-tauri/src/sandbox/lua_host.rs`
- **Status subtahap:** `completed`
- **Status fase:** `in-progress` (Langkah 1–2 selesai; Langkah 3–4 menyusul)

## Tujuan dan batas cakupan

Menyediakan runtime sandbox Lua 5.4 (`mlua`) terisolasi per instance komponen beserta permukaan API host `nodus.*` v1, pembatasan memori & instruksi, pembatasan jumlah timer aktif per instance, serta penanganan kegagalan deterministik tanpa menghentikan aplikasi.

## Hasil dan perubahan

- `src-tauri/Cargo.toml`: menambahkan `mlua = { version = "=0.11.4", features = ["lua54", "vendored"] }` dan `crossbeam-channel = "=0.5.15"`.
- `src-tauri/src/sandbox/pkg_loader.rs` & `src-tauri/src/sandbox/manifest.rs`: menyimpan `lua_source` di `ComponentPackage` dengan atribut `#[serde(skip_serializing)]` sehingga skrip Lua tersedia untuk runtime backend namun tidak pernah diserialisasi ke frontend JSON IPC.
- `src-tauri/src/sandbox/lua_host.rs`: implementasi `LuaSandboxInstance` dan `InstanceDiagnostic`:
  - Hanya mengaktifkan pustaka standar aman (`table`, `string`, `math`, `utf8`) dan menghapus `io`, `os`, `package`, `debug`, `coroutine`, `dofile`, `load`, `loadfile`, `require`, `collectgarbage`, `math.random`, serta `math.randomseed`.
  - Batas memori `256 KiB` per instance, batas `10.000` instruksi per callback (`HookTriggers::every_nth_instruction(250)`), dan batas `64` timer aktif per instance.
  - Permukaan API `nodus.set_pin_mode`, `nodus.set_pin`, `nodus.read_pin`, `nodus.schedule_timer`, `nodus.cancel_timer`, dan `nodus.set_visual_state` dengan pemeriksaan kepemilikan pin/visual, penolakan pin `NC`, serta validasi rentang nilai.
  - Callback host `on_init()`, `on_control_change(control_id, value)`, `on_pin_edge(pin_id, state, timestamp_ns)`, dan callback timer terdaftar.
  - Kegagalan runtime / pelanggaran kuota menonaktifkan instance yang bermasalah, melepaskan seluruh penggerak pin miliknya ke `HIGH_Z`, membatalkan seluruh timer miliknya di `Scheduler`, dan menghasilkan `InstanceDiagnostic` terstruktur.
- `src-tauri/src/sandbox/lua_host_tests.rs`: 3 pengujian unit menyeluruh untuk siklus hidup API `nodus.*`, sanitasi global/RNG terlarang, serta proteksi infinite loop, kehabisan memori, dan banjir timer.

## Keputusan teknis

- Operasi mutasi dari dalam callback Lua dikumpulkan ke antrean transaksi (`HostOp`) dan diterapkan secara atomik setelah callback selesai; bila callback gagal di tengah jalan, tidak ada mutasi parsial yang bocor ke sirkuit atau scheduler, dan pin/timer yang sudah terpasang langsung dibersihkan.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust bersih |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 63 tes Rust lulus (termasuk 3 tes sandbox Lua baru) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Bebas warning di seluruh target |
| `wc -l src-tauri/src/sandbox/*.rs` | passed | `lua_host.rs` 394 baris, `lua_host_tests.rs` 270 baris (<= 400 baris) |

## Hambatan dan risiko

- Tidak ada blocker untuk Langkah 3 (protokol frame biner ber-versi dan streamer 60 Hz).

## Handoff: langkah berikutnya

1. Lanjutkan ke Fase 2 Langkah 3: implementasi `src-tauri/src/ipc/frame_streamer.rs` dan decoder frame biner di `src/store/simulationStore.ts`.

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
