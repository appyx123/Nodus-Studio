# 005 — Fase 1: Kontrak Paket, Loader, dan Katalog Komponen

- **Tanggal:** 2026-11-06 (tanggal sesi agent)
- **Agent / pelaksana:** Zed coding agent
- **Fase dan langkah blueprint:** Fase 1, Langkah 2 — kontrak/loader `.nodpkg`, dengan integrasi katalog sebagai konsumen IPC
- **Status subtahap:** `completed`
- **Status fase:** `in-progress`; kanvas interaktif, kabel, dan netlist belum dibuat

## Tujuan dan batas cakupan

Memuat paket direktori `.nodpkg` melalui Rust, memvalidasi metadata dan SVG sebelum dikirim ke frontend, serta menampilkan fixture LED dan diagnostik pada aplikasi desktop. Tidak menjalankan Lua, scheduler, simulasi listrik, atau emulasi MCU. Katalog sudah tersedia; ini bukan penyelesaian seluruh Langkah 3 maupun Fase 1.

## Hasil dan perubahan

- `src-tauri/src/sandbox/{mod.rs,manifest.rs,pkg_loader.rs,svg.rs}`: kontrak manifest v1, loader berbatas, dan validator SVG allowlist yang membangun ulang XML. Tes terpisah pada `manifest_tests.rs`, `pkg_loader_tests.rs`, dan `svg_tests.rs`.
- `src-tauri/src/ipc/{mod.rs,commands.rs}`, `src-tauri/src/{lib.rs,error.rs}`: command async `list_components` tanpa argumen path dari frontend, scanning melalui blocking worker, dan error terpusat yang dapat diserialisasikan.
- `src-tauri/build.rs`, `src-tauri/capabilities/main.json`, `src-tauri/tauri.conf.json`: permission app-command eksplisit, capability hanya jendela lokal `main`, serta pemetaan resource registry. Tidak menambahkan filesystem/shell plugin atau melonggarkan CSP.
- `src-tauri/Cargo.toml` dan `Cargo.lock`: dependency langsung serde, serde_json, toml, semver, quick-xml, serta libc khusus Unix; menggunakan versi yang tersedia di cache. Lockfile tetap dihasilkan/dikelola Cargo.
- `components-registry/basic-led.nodpkg/{manifest.toml,graphic.svg,logic.lua}`: fixture LED lengkap, pin A/K, jarak 25.4 unit SVG, dan deklarasi opacity `brightness`. Lua hanya placeholder teks; tidak dieksekusi atau dikirim ke UI.
- `src/types/nodpkg.ts`, `src/components/library/ComponentCatalog.tsx`, `src/App.tsx`: tipe IPC, loading/error/empty states, preview SVG melalui `<img>` data URL, metadata, diagnostik paket, serta Reload. Guard error resource Solid mencegah pembacaan resource gagal menjatuhkan shell.
- `package.json`, `package-lock.json`: API resmi `@tauri-apps/api` 2.12.4. `scripts/nodpkg-contract.test.mjs` memeriksa fixture bersama dengan TypeScript; `scripts/scaffold.test.mjs` memeriksa permission/resource/CSP. `.gitignore` mengabaikan direktori sementara pengujian kontrak.
- `scripts/desktop-catalog-smoke.py`: smoke test berbatas, PID-scoped, menggunakan hardlink binary dan salinan registry privat; membersihkan proses/file uji tanpa mengubah registry asli.
- `docs/nodpkg-v1.md`, `docs/nodpkg-v1-led.json`: kontrak subset yang benar-benar diimplementasikan dan fixture normalisasi bersama Rust/TypeScript. Bagian 4.2 blueprint menautkan kontrak dan menjelaskan batas subset awal.
- `README.md`, `PROGRESS.md`, dan laporan ini diperbarui. Tidak memasang paket sistem atau membuat commit/branch/staging.

## Keputusan teknis

- Manifest menolak field tidak dikenal pada semua kedalaman; schema/API v1, SemVer komponen, ID direktori, pin/anchor, rentang finite, slider, dan binding opacity divalidasi. Kontrol/visual opsional dinormalisasi menjadi array kosong.
- Batas file: manifest 64 KiB, SVG 256 KiB, Lua 128 KiB. Registry maksimal 512 entri langsung, 128 direktori paket yang diproses, payload paket JSON 14 MiB, serta diagnostik berbatas agar total katalog di bawah 16 MiB. Paket gagal tidak menggagalkan sibling valid.
- Menolak symlink root/paket/file, memeriksa containment canonical, lalu melakukan bounded read. Unix final open memakai `O_NOFOLLOW | O_NONBLOCK` dan pemeriksaan tipe descriptor untuk menolak penggantian symlink/FIFO sesudah pengecekan path.
- SVG memakai subset ketat, bukan sanitizer SVG umum: tanpa script/event/CSS/URL/reference/transform/gradient/filter. ID, geometri, viewBox, kedalaman, dan jumlah elemen dibatasi. XML dinormalisasi; SVG mentah tidak diteruskan.
- Preview berupa image document terisolasi, bukan `innerHTML`. Namespace ID per instance tetap wajib saat renderer inline kanvas dibangun nanti.
- Development memakai `cfg(dev)` dan `CARGO_MANIFEST_DIR`; native build memakai resource directory Tauri, bukan CWD. Pada Linux, layout relocation uji adalah `bin/nodus` dan `lib/Nodus Studio/components-registry/` sesuai resolver Tauri. Menyalin binary saja tidak cukup.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| Regresi final-open symlink sebelum hardening, lalu sesudah perbaikan | failed → passed | Tes gagal dengan `File::open` lama lalu lulus setelah nofollow/nonblocking; bukti red/green tercatat pada sesi implementasi |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format diterima |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 34 tes Rust; manifest, fixture, SVG berbahaya/rusak, isolasi paket, batas resource, symlink/FIFO, dan resource-path helper |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Tidak ada warning/error |
| `npm test` | passed | 5 tes: konfigurasi dan fixture TypeScript positif/negatif |
| `npm run build` | passed | Typecheck dan Vite production build |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --no-bundle --debug` | passed | Native debug binary di `src-tauri/target/debug/nodus`; percobaan akhir compilation 9.67 detik |
| Percobaan awal `python3 scripts/desktop-catalog-smoke.py` | failed | Asumsi skrip salah: resource ditempatkan di sebelah binary relocation dan role SVG diasumsikan `image`; bukti accessibility menunjukkan fallback `/usr/lib/Nodus Studio` dan SVG sebagai panel/section. Skrip diperbaiki berdasarkan resolver dependency dan konten PID uji, bukan melonggarkan permission |
| `python3 scripts/desktop-catalog-smoke.py` setelah build akhir | passed | Resource relocation dan CWD tak terkait; LED + konten SVG; paket invalid terisolasi; Reload menghapus diagnostik; registry hilang menampilkan error tanpa crash; restorasi registry pulih. Child dan salinan privat dibersihkan |
| Pemeriksaan Python batas baris, unwrap/expect produksi, dan alignment lockfile langsung | passed | Source aplikasi/skrip ≤400 baris; tidak ada unwrap/expect Rust produksi; dependency npm dan versi Rust langsung cocok dengan lockfile; 407 entri Cargo |
| Pemeriksaan tautan lokal dokumentasi yang berubah | passed | Target tautan README, PROGRESS, kontrak, blueprint, dan laporan tersedia |
| `git --no-optional-locks status --short` | failed | Workspace tidak memiliki repository Git; tidak menginisialisasi Git atau mengubah staging |
| Browser-only fallback runtime dan pixel-level visual QA | not-run | Guard fallback ada dan build lulus; belum diuji dalam browser. Accessibility bukan pemeriksaan piksel atau audit seluruh CSP |
| Release build, RPM/Flatpak, dan compiler minimum Rust 1.90 | not-run | Di luar cakupan; verifikasi aktual menggunakan Fedora Rust/Cargo 1.99.1 |

## Hambatan dan risiko

- Tidak ada blocker aktif untuk Langkah 3. Registry bawaan diasumsikan dikuasai pemilik aplikasi dan tidak dimodifikasi penyerang secara konkuren.
- Hardening final-open bukan sandbox filesystem race-proof: penggantian direktori parent secara konkuren belum ditangani dengan descriptor-relative traversal penuh. Kebijakan harus diperkuat sebelum mendukung lokasi paket dengan hostile concurrent writers.
- Subset SVG sengaja menolak fitur visual kaya. Ekstensi kontrak harus ditambah bersama tes, bukan melewati validator.
- Smoke test memastikan konten SVG tersedia lewat accessibility, bukan kualitas visual seluruh piksel atau seluruh aturan CSP. Distribusi resource RPM/Flatpak masih harus diverifikasi pada Fase 5.
- Belum ada kanvas multi-instance, wiring, netlist, Lua runtime, ataupun simulasi. Jangan menandai Fase 1 selesai.

## Handoff: langkah berikutnya

1. Baca AGENTS, blueprint, PROGRESS, laporan ini, dan kontrak paket sebelum melanjutkan.
2. Lanjut Fase 1 Langkah 3: `Workbench.tsx` dan `NodeRenderer.tsx`, placement dari katalog yang sudah ada, pan/zoom, snap 25.4 unit SVG, serta drag-and-drop. Tetapkan transform koordinat dan identitas instance sebelum implementasi; uji anchor tetap benar saat viewport/posisi berubah.
3. Namespace seluruh ID/referensi SVG per `instance_id` sebelum renderer inline. Uji beberapa instance LED yang sama tanpa tabrakan. Jangan menaruh simulasi hardware di frontend.
4. Sesudah kanvas teruji, Langkah 4 menambah WireLayer dan topologi/netlist Rust dengan validasi endpoint serta penghapusan konsisten. Jangan memulai Lua/scheduler sebelum seluruh acceptance Fase 1 terpenuhi.
5. Jalankan focused tests, build, dan desktop interaction checks sesuai perubahan. Simpan laporan `006` atau nomor berikutnya yang belum digunakan, lalu perbarui PROGRESS; jangan menulis ulang hasil historis laporan ini.

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
