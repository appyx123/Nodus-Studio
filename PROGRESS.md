# Nodus Studio — Progress & Handoff

## Mulai dari sini

Baca `AGENTS.md`, blueprint, lalu laporan terbaru sebelum melanjutkan. Blueprint tetap menjadi sumber kebenaran persyaratan; file ini merangkum keadaan implementasi.

- **Fase aktif:** Fase 1 — Fondasi Aplikasi & Kanvas Interaktif.
- **Subtahap:** Langkah 3 — katalog dan kanvas interaktif.
- **Status subtahap:** `completed` untuk implementasi dan verifikasi terbatas laporan 008; pemeriksaan input fisik/manual masih `not-run`.
- **Status kesiapan build:** frontend dan native debug `completed`; release build `not-started`.
- **Status Fase 1:** `in-progress`; kabel dan netlist Rust belum dibuat.
- **Laporan terbaru:** [008 — Kanvas interaktif dan instance komponen](docs/reports/008-phase-1-interactive-workbench.md).
- **Repository:** https://github.com/appyx123/Nodus-Studio.git — branch `main`, upstream `origin/main`; README sebelumnya sudah dipublikasikan pada `4f686db`. Perubahan kanvas tahap 008 masih lokal, belum commit/push.
- **Langkah berikutnya:** Fase 1 Langkah 4 — kabel dan netlist authoritative Rust (`not-started`), dengan manual check input fisik kanvas yang masih perlu dicatat.

## Ringkasan roadmap

| Fase | Status | Bukti / catatan |
| --- | --- | --- |
| 1 — Fondasi dan kanvas | `in-progress` | Fondasi, loader, katalog, dan kanvas visual tersedia; tes/build/WebKit/native accessibility lulus; kabel/netlist serta acceptance akhir belum selesai |
| 2 — Mesin simulasi dan Lua | `not-started` | Menunggu seluruh acceptance Fase 1 selesai dan teruji |
| 3 — Editor, compiler, dan AVR | `not-started` | Menunggu Fase 2 selesai dan teruji |
| 4 — Sensor, bus, dan MQTT | `not-started` | Menunggu Fase 3 selesai dan teruji |
| 5 — Penyimpanan dan distribusi | `not-started` | Menunggu Fase 4 selesai dan teruji |

## Implementasi yang tersedia

- Kontrak subset v1: [`docs/nodpkg-v1.md`](docs/nodpkg-v1.md) dan fixture normalisasi bersama [`docs/nodpkg-v1-led.json`](docs/nodpkg-v1-led.json). Schema/API v1; deklarasi slider dan visual opacity numerik saja.
- Rust: manifest/loader/validator SVG di `src-tauri/src/sandbox/`, IPC `list_components` di `src-tauri/src/ipc/`, error terpusat, tes berbatas resource serta isolasi paket invalid. Fixture `components-registry/basic-led.nodpkg/` lengkap. **Lua tidak dieksekusi atau dikirim ke frontend.**
- Katalog: discovery resource bersama dari App, preview `<img>` terisolasi, loading/error/empty/diagnostics, Reload, tombol Add dan sumber drag HTML.
- Kanvas: `src/components/canvas/{Workbench.tsx,NodeRenderer.tsx,graphic.ts,geometry.ts}`. Beberapa instance, namespace SVG, pan/zoom, grid 25.4 unit, drag/drop, seleksi, keyboard nudge, dan delete. Layout dan fokus di `src/styles.css`.
- Kontrak koordinat: `screen = translation + world * zoom`; screen dikurangi origin bounding rectangle canvas. Posisi instance adalah top-left viewport nested SVG. Zoom 0.2–4; awal `(60,40,2)`; pan tanpa batas dunia eksplisit. Setiap instance `node-<UUID v4>` memiliki paket snapshot dan ID SVG `<instance_id>--<original_id>`.
- State penempatan masih presentasi frontend selama sesi, maksimal 256 instance. Reload katalog mempertahankan placement; tutup app/reload halaman menghapusnya. Ini bukan netlist atau persistensi proyek.
- Capability lokal `main` tetap hanya discovery; tanpa argumen path frontend, filesystem/shell plugin, atau remote capability. CSP/dependency/lockfile tidak berubah pada tahap 008.
- Registry development memakai `CARGO_MANIFEST_DIR`; native build memakai resource directory Tauri. Pada Linux relocation: `bin/nodus` dan `lib/Nodus Studio/components-registry/`, bukan binary saja atau CWD.

## Lingkungan dan verifikasi terakhir

Fedora Workstation 45 x86_64, GNOME/Wayland; Rust/Cargo 1.99.1 dari Fedora, rustfmt 1.10.0, Clippy 0.1.99. Native dependencies tersedia: WebKitGTK 2.54.2, GTK3 3.24.52, OpenSSL 4.0.3, librsvg 2.63.2, AppIndicator, dan libxdo. Node 22.23.1, npm 10.9.8. System Python memiliki `pyatspi` dan `gi` GTK3/WebKit2 4.1 untuk skrip uji. Tidak memasang paket sistem baru.

Stack tetap SolidJS 1.9.18, TypeScript 7.0.2, Vite 8.3.6, Tailwind 4.3.4, CLI/API Tauri 2.12.4, Rust tauri 2.12.5, dan tauri-build 2.7.4. Edition 2021, minimum manifest Rust 1.90; compiler minimum belum diuji tersendiri. Kedua lockfile tersedia; Cargo.lock berisi 407 entri. Binary debug terbaru: `src-tauri/target/debug/nodus`.

| Pemeriksaan | Hasil | Cakupan |
| --- | --- | --- |
| `npm test` | passed | 20 tes Node: konfigurasi/kontrak + 15 geometri viewport/snap/drag |
| `npm run build` | passed | Typecheck dan Vite produksi |
| `python3 scripts/webkit-workbench-smoke.py` | passed | 12 checks interaksi/geometri/hit-test nyata dengan discovery mock dan event sintetik; 1 fallback browser tanpa mock. Bukan native Tauri IPC atau input fisik |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 34 tes Rust loader/manifest/SVG; belum ada netlist |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Semua target tanpa warning |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --no-bundle --debug` | passed | Native debug build setelah perbaikan hit-test SVG |
| `python3 scripts/desktop-catalog-smoke.py` | passed | Native resource/IPC, dua Add/identitas, zoom/reset/delete, isolasi paket, Reload dan error recovery. PID-scoped; child/salinan privat dibersihkan |
| Source limits, tautan lokal/anchor dokumentasi, dan `git diff --check` | passed | Source ≤400 baris; direktori uji sementara dibersihkan |
| Input fisik/trusted pointer capture, touch/trackpad, pixel QA, stress 256 instance | not-run | Tidak diklaim tercakup oleh event sintetik/accessibility |
| Release/RPM/Flatpak dan compiler minimum | not-run | Tetap di luar cakupan tahap ini |

## Hambatan dan risiko

1. Tidak ada blocker aktif untuk Langkah 4. Kanvas belum menyimpan topologi di backend. Seluruh Fase 1 belum selesai; Lua/scheduler/resolusi listrik/emulasi tetap dilarang pada fase ini.
2. Input fisik drag/drop, pointer release di luar canvas, trusted capture/lost capture, touch/trackpad dan visual piksel perlu pemeriksaan desktop manual. WebKit hit-test dan event sintetik lulus, tetapi bukan bukti seluruh alur input native.
3. Penempatan hanya selama sesi; guard 256 instance bukan benchmark. Performa pada batas instance/SVG kompleks belum diuji; jangan menjanjikan 60 FPS.
4. Loader menolak symlink dan membatasi read/payload. Unix final-open menolak symlink/FIFO replacement, tetapi traversal parent belum race-proof terhadap hostile concurrent writers. Registry bawaan diasumsikan dikendalikan pemilik aplikasi.
5. SVG tetap subset ketat. Renderer inline hanya menerima hasil Rust, bukan SVG arbitrer. Gradient/filter/transform/reference paket belum didukung meskipun helper dapat menulis ulang referensi ID lokal.
6. Repository lokal tersedia dan riwayat remote/LICENSE dipertahankan. Perubahan tahap ini belum staging/commit/push; izin publikasi sebelumnya bukan izin permanen.
7. Rust aktual 1.99.1 terverifikasi, bukan dipatok lewat rustup. Minimum 1.90 dan distribusi belum diuji.

## Tindakan berikutnya

1. Lakukan manual check input fisik kanvas jika lingkungan memungkinkan dan catat hasil/batasnya; panduan kontrol tersedia di README dan laporan 008.
2. Pada kelanjutan pengguna, tetapkan kontrak instance, endpoint pin, dan graf authoritative Rust untuk Fase 1 Langkah 4. Backend harus memvalidasi paket/pin; jangan memakai graf frontend sebagai sumber kebenaran.
3. Tambahkan `WireLayer.tsx`, klik pin A ke B, routing Manhattan, warna kabel, serta hapus kabel/instance yang konsisten dengan graf. Tetapkan pemetaan anchor nested SVG ke world/screen termasuk origin viewBox.
4. Tambahkan tes Rust netlist, tes anchor/kabel saat pan/zoom/drag, serta smoke test native. Nilai seluruh acceptance Fase 1 hanya setelah kabel/netlist teruji; belum ada resolusi listrik.
5. Simpan laporan `009` atau nomor berikutnya yang belum terpakai dan perbarui file ini. Jangan mengubah laporan lama untuk memasukkan hasil baru atau commit/push tanpa instruksi.

## Laporan

- [001 — Fase 1: pemeriksaan lingkungan dan aturan laporan](docs/reports/001-phase-1-environment.md)
- [002 — Fase 1: scaffold aplikasi dan hambatan dependency](docs/reports/002-phase-1-application-scaffold.md)
- [003 — Fase 1: verifikasi prasyarat native dan timeout build](docs/reports/003-phase-1-native-build-attempt.md)
- [004 — Fase 1: build native debug dan smoke test desktop](docs/reports/004-phase-1-native-build-smoke.md)
- [005 — Fase 1: kontrak paket, loader, dan katalog](docs/reports/005-phase-1-component-loader.md)
- [006 — Fase 1: publikasi repository GitHub](docs/reports/006-phase-1-github-publication.md)
- [007 — Fase 1: penyegaran README open source](docs/reports/007-phase-1-readme-refresh.md)
- [008 — Fase 1: kanvas interaktif dan instance komponen](docs/reports/008-phase-1-interactive-workbench.md)

Template: [`docs/reports/TEMPLATE.md`](docs/reports/TEMPLATE.md). Nomor laporan berikutnya: `009` (periksa ulang sebelum membuat file).
