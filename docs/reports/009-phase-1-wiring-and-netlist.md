# 009 — Fase 1: Kabel Manhattan dan Netlist Authoritative Rust

- **Tanggal:** 2026-11-06
- **Agent / pelaksana:** Antigravity coding agent
- **Fase dan langkah blueprint:** Fase 1, Langkah 4 — kabel dan netlist
- **Status subtahap:** `completed`
- **Status fase:** `completed` (seluruh deliverable dan kriteria lulus otomatis Fase 1 terpenuhi; pemeriksaan input fisik manual tetap dicatat terpisah sebagai `not-run`)

## Tujuan dan batas cakupan

Menyelesaikan Langkah 4 dan menutup Fase 1 dengan menghubungkan kanvas interaktif ke graf topologi authoritative di backend Rust (`CircuitState`), merender kabel *orthogonal/Manhattan* di `WireLayer.tsx`, mendukung penyambungan pin A ke pin B, perubahan warna kabel, serta penghapusan kabel/instance yang menjaga konsistensi netlist. Tidak ada eksekusi Lua, scheduler, atau resolusi listrik di Fase 1.

## Hasil dan perubahan

- `src-tauri/src/engine/{mod.rs,types.rs,circuit_graph.rs}`: state topologi sesi authoritative di Rust (`CircuitState`, `CircuitEdit`, `CircuitSnapshot`, `Wire`, `Net`, `Endpoint`, `WireColor`), validasi UUID kanonis (`node-<UUID>`, `wire-<UUID>`), penolakan pin `nc` dan koneksi duplikat/identik, batas 256 instance dan 1.024 kabel, kalkulasi net terhubung secara deterministik, serta penghapusan kabel otomatis saat instance dihapus.
- `src-tauri/src/engine/{contract_tests.rs,graph_tests.rs,validation_tests.rs,test_support.rs}` & `src-tauri/tests/fixtures/`: 16 tes unit mesin netlist untuk kontrak JSON, transitivitas net, siklus, penghapusan jembatan, isolasi reload katalog, dan batas kapasitas.
- `src-tauri/src/ipc/{circuit.rs,circuit_tests.rs,commands.rs,mod.rs}`, `src-tauri/src/{lib.rs,error.rs}`, `src-tauri/build.rs`, `src-tauri/capabilities/main.json`: command IPC `get_circuit` dan `edit_circuit`, sinkronisasi `register_scan` saat `list_components`, penanganan mutex poisoned, serta 5 tes konkurensi/state IPC.
- `src/types/nodpkg.ts`, `src/store/circuitStore.ts`: kontrak TypeScript (`WireColor`, `Endpoint`, `CircuitEdit`, `CircuitSnapshot`, `Wire`, `Net`) dan helper IPC `getCircuit`/`editCircuit` beserta palet 5 warna kabel (`green`, `blue`, `amber`, `red`, `violet`).
- `src/components/canvas/geometry.ts`: fungsi `pinWorldPoint` (memperhitungkan posisi instance dan origin `viewBox`), `manhattanRoute`, dan `manhattanPath`.
- `src/components/canvas/graphic.ts`: ekstraksi koordinat pusat elemen anchor SVG (`pin-*`), atribut `data-pin-id`/`data-pin-type`, dan aksesibilitas pin.
- `src/components/canvas/WireLayer.tsx`: rendering kabel Manhattan dengan keying stabil per `wire_id`, preview garis putus-putus saat menyambung pin, seleksi kabel, dan hapus kabel via keyboard/toolbar.
- `src/components/canvas/{NodeRenderer.tsx,Workbench.tsx}`, `src/styles.css`: alur klik pin A ke pin B (serta tombol pin aksesibel di panel status), pembatalan via Escape, pemilih warna kabel, sinkronisasi tambah/hapus instance & kabel dengan Rust, serta tampilan jumlah instance/wire/net. Semua file sumber dijaga ≤400 baris (`Workbench.tsx` 375 baris).
- `scripts/{scaffold.test.mjs,nodpkg-contract.test.mjs,workbench-geometry.test.mjs,webkit-workbench-smoke.py,workbench-dom-checks.js,desktop-catalog-smoke.py}`: penambahan tes kontrak TypeScript `CircuitEdit`/`CircuitSnapshot`, tes matematika `pinWorldPoint` dan `manhattanRoute`, tes DOM WebKitGTK untuk kabel/recolor/pergerakan endpoint/cascading delete, serta smoke test native GNOME/Wayland untuk penyambungan pin, warna kabel, dan hapus kabel/instance.
- `docs/nodpkg-v1.md`, `README.md`, `PROGRESS.md`: pembaruan dokumentasi kontrak IPC, kontrol workbench, dan status kelulusan Fase 1.

## Keputusan teknis

- Backend Rust (`CircuitState`) adalah satu-satunya sumber kebenaran untuk instance tervalidasi, koneksi kabel, dan pengelompokan net. Frontend hanya memegang posisi koordinat dunia `(x, y)` dan state interaksi viewport/seleksi.
- Koordinat dunia pin dihitung dengan `instance_position + local_pin - viewBox_origin`, sehingga tetap akurat saat pan, zoom, maupun saat komponen digeser atau di-nudge.
- Routing Manhattan menggunakan segmen ortogonal (`M start L midX startY L midX endY L end`) yang dihitung ulang secara reaktif saat posisi instance berubah.
- Karena aturan ARIA `role="button"` pada grup SVG menyembunyikan child SVG dari AT-SPI di WebKitGTK, workbench menyediakan dua jalur setara untuk menyambung pin: klik langsung pada anchor `<circle data-pin-id="...">` di kanvas SVG dan tombol HTML `Pin <ID>` saat instance dipilih.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `npm test` | passed | 24 tes Node lulus: scaffold, kontrak manifest + circuit topology TypeScript, serta 17 tes geometri/pin/Manhattan |
| `npm run build` | passed | `tsc --noEmit` dan Vite production build berhasil |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Seluruh file Rust sesuai format |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 56 tes Rust lulus (34 loader/manifest/SVG + 22 circuit graph & IPC) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Semua target bebas warning |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --no-bundle --debug` | passed | Binary native `src-tauri/target/debug/nodus` berhasil dibangun |
| `python3 scripts/webkit-workbench-smoke.py` | passed | 12 pemeriksaan DOM WebKitGTK (termasuk klik pin A ke B, Manhattan path, net count, recolor, nudge endpoint, dan cascading delete) + 1 browser fallback lulus |
| `python3 scripts/desktop-catalog-smoke.py` | passed | Smoke test native Fedora GNOME/Wayland lulus: discovery, Add 2 LED, sambung kabel pin A ke K, netlist 3 net (1 terhubung), ubah warna kabel ke Amber, zoom/reset, hapus kabel, hapus instance, dan pemulihan registry |
| Batas baris source (`wc -l`) & `git diff --check` | passed | Semua file kode ≤400 baris (`Workbench.tsx` 375 baris); tidak ada whitespace error |
| Input fisik mouse/touchpad, trusted pointer capture, pixel QA | not-run | Diuji melalui event DOM sintetik di WebKitGTK dan aksi aksesibilitas AT-SPI native; bukan gerakan mouse fisik |

## Hambatan dan risiko

- Tidak ada blocker untuk memulai Fase 2.
- Topologi rangkaian masih bersifat sesi (hilang saat aplikasi ditutup); penyimpanan file `.nodus` dijadwalkan pada Fase 5 sesuai blueprint.
- Belum ada simulasi waktu, eksekusi Lua, atau resolusi tegangan/logika pada Fase 1.

## Handoff: langkah berikutnya

1. Mulai **Fase 2 (Mesin Simulasi Inti & Sandbox Komponen Lua)** sesuai urutan blueprint:
   - Langkah 1: Tetapkan representasi waktu presisi tanpa drift ($16\text{ MHz} \Rightarrow 62.5\text{ ns/siklus}$) dan aturan resolusi net digital (`HIGH`, `LOW`, `HIGH_Z`, `UNKNOWN`) di `scheduler.rs`, `pin.rs`, dan `circuit_graph.rs`, lengkap dengan kontrol `start`/`pause`/`resume`/`stop`/`reset`.
   - Langkah 2: Integrasikan `mlua` (Lua 5.4) di `lua_host.rs` dengan sandbox ketat, batas memori/instruksi, dan API `nodus.*`.
   - Langkah 3: Definisikan protokol frame biner ber-versi dan implementasikan `frame_streamer.rs` ($60\text{ Hz}$, *latest state wins*).
   - Langkah 4: Buat paket `basic-power-source.nodpkg` dan `basic-pushbutton.nodpkg`, serta aktifkan `logic.lua` pada `basic-led.nodpkg`.
2. Simpan laporan tahap berikutnya pada `docs/reports/010-phase-2-...` dan perbarui `PROGRESS.md`.

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
