# Nodus Studio — Progress & Handoff

## Mulai dari sini

Baca `AGENTS.md`, blueprint, lalu laporan terbaru sebelum melanjutkan. Blueprint tetap menjadi sumber kebenaran persyaratan; file ini hanya merangkum keadaan implementasi.

- **Fase aktif:** Fase 1 — Fondasi Aplikasi & Kanvas Interaktif.
- **Subtahap:** Langkah 2 — kontrak/loader `.nodpkg`, fixture LED, dan integrasi katalog desktop.
- **Status subtahap:** `completed`; loader/katalog dan smoke test terbatas terverifikasi pada laporan 005.
- **Status kesiapan build:** frontend dan native debug `completed`; release build `not-started`.
- **Status Fase 1:** `in-progress`; kanvas interaktif, kabel, dan netlist belum dikerjakan.
- **Laporan terbaru:** [007 — Penyegaran README open source](docs/reports/007-phase-1-readme-refresh.md). Pembaruan dokumentasi `completed`; implementasi/tes Langkah 2 tetap pada [laporan 005](docs/reports/005-phase-1-component-loader.md).
- **Repository:** https://github.com/appyx123/Nodus-Studio.git — branch `main`, upstream `origin/main`; publikasi implementasi `10dd73b` terverifikasi.
- **Langkah berikutnya:** Fase 1 Langkah 3 — renderer/kanvas interaktif (`not-started` untuk kanvas; katalog sudah tersedia).

## Ringkasan roadmap

| Fase | Status | Bukti / catatan |
| --- | --- | --- |
| 1 — Fondasi dan kanvas | `in-progress` | Fondasi, loader, fixture LED, dan katalog lulus tes/build/desktop smoke; kanvas, kabel, dan netlist belum dibuat |
| 2 — Mesin simulasi dan Lua | `not-started` | Menunggu seluruh acceptance Fase 1 selesai dan teruji |
| 3 — Editor, compiler, dan AVR | `not-started` | Menunggu Fase 2 selesai dan teruji |
| 4 — Sensor, bus, dan MQTT | `not-started` | Menunggu Fase 3 selesai dan teruji |
| 5 — Penyimpanan dan distribusi | `not-started` | Menunggu Fase 4 selesai dan teruji |

## Implementasi yang tersedia

- Kontrak subset v1: [`docs/nodpkg-v1.md`](docs/nodpkg-v1.md), dengan fixture normalisasi bersama [`docs/nodpkg-v1-led.json`](docs/nodpkg-v1-led.json). Schema/API v1; kontrol slider dan visual opacity numerik saja.
- Rust: manifest/loader/validator SVG di `src-tauri/src/sandbox/`, IPC `list_components` di `src-tauri/src/ipc/`, error terpusat, serta tes terpisah. Paket gagal menghasilkan diagnostik tanpa menggagalkan sibling valid.
- Registry: `components-registry/basic-led.nodpkg/` lengkap. Lua hanya dicek sebagai teks UTF-8 nonkosong; **tidak dieksekusi atau dikirim ke frontend**.
- UI: `src/types/nodpkg.ts`, `src/components/library/ComponentCatalog.tsx`, dan shell App. Preview memakai `<img>` SVG terisolasi; tersedia Reload dan error recovery. Belum ada placement atau perilaku pin.
- Capability lokal jendela `main` hanya mengizinkan discovery; tanpa argumen path frontend, filesystem/shell plugin, atau remote capability. CSP tetap aktif.
- Registry development memakai `CARGO_MANIFEST_DIR`; native build memakai resource directory Tauri. Pada Linux relocation, gunakan `bin/nodus` dan `lib/Nodus Studio/components-registry/`, bukan menyalin binary saja atau bergantung CWD.

## Lingkungan dan verifikasi terakhir

Fedora Workstation 45 x86_64, GNOME/Wayland; Rust/Cargo 1.99.1 dari Fedora, rustfmt 1.10.0, Clippy 0.1.99. Native dependencies tersedia: WebKitGTK 2.54.2, GTK3 3.24.52, OpenSSL 4.0.3, librsvg 2.63.2, AppIndicator, dan libxdo. Node 22.23.1 dan npm 10.9.8. Tidak perlu mengulang instalasi atau menghapus cache.

Stack tetap SolidJS 1.9.18, TypeScript 7.0.2, Vite 8.3.6, Tailwind 4.3.4, CLI/API Tauri 2.12.4, Rust tauri 2.12.5, dan tauri-build 2.7.4. Edition 2021, minimum manifest Rust 1.90; compiler minimum belum diuji tersendiri. Kedua lockfile tersedia; Cargo.lock berisi 407 entri. Binary debug: `src-tauri/target/debug/nodus`.

| Pemeriksaan | Hasil | Cakupan |
| --- | --- | --- |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 34 tes Rust kontrak, loader, SVG, batas resource, symlink/FIFO, dan resource-path helper |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Semua target tanpa warning |
| `npm test` | passed | 5 tes konfigurasi dan fixture TypeScript positif/negatif |
| `npm run build` | passed | Typecheck dan produksi Vite |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --no-bundle --debug` | passed | Native debug build dengan frontend terkini |
| `python3 scripts/desktop-catalog-smoke.py` | passed | Resource Linux relocated/CWD independen, LED dan konten SVG, isolasi paket invalid, Reload, error registry hilang dan pemulihan; PID-scoped, child/salinan privat dibersihkan |
| Source limits dan lockfile alignment | passed | Source ≤400 baris, tanpa unwrap/expect Rust produksi; dependency langsung cocok lockfile |
| Browser fallback runtime, pixel QA, release/RPM/Flatpak, compiler minimum | not-run | Tidak diklaim terverifikasi |

## Hambatan dan risiko

1. Tidak ada blocker aktif untuk kanvas. Seluruh Fase 1 belum selesai; Lua/scheduler/emulasi tetap di luar cakupan.
2. Loader menolak symlink dan membatasi read/payload. Unix final-open menolak symlink/FIFO replacement, tetapi traversal parent belum race-proof terhadap hostile concurrent writers. Registry bawaan diasumsikan dikendalikan pemilik aplikasi. Jangan memperluas ke registry tak tepercaya tanpa meninjau batas ini.
3. SVG sengaja merupakan subset ketat, bukan sanitizer umum. Gradient/filter/transform dan reference belum didukung. Inline renderer mendatang wajib namespace ID per instance; isolasi image katalog saja belum memenuhi acceptance multi-instance kanvas.
4. Desktop smoke berbasis accessibility membuktikan konten dan pemulihan error, bukan tampilan piksel, seluruh CSP, atau interaksi kanvas. Script memerlukan GNOME/Wayland dan `pyatspi` yang sudah tersedia; tidak memasang paket sendiri.
5. Repository Git diinisialisasi dan dipublikasikan atas permintaan pengguna pada tahap 006. Riwayat remote dan LICENSE MIT dipertahankan; push implementasi fast-forward tanpa force-push. Jangan menginisialisasi ulang atau membuat commit/push berikutnya tanpa instruksi pengguna.
6. Rust aktual 1.99.1 terverifikasi, bukan dipatok melalui rustup. Minimum 1.90 dan paket distribusi belum diuji.

## Tindakan berikutnya

1. Pada kelanjutan yang diminta pengguna, kerjakan Fase 1 Langkah 3: kontrak posisi/instance, transform screen/world, `Workbench.tsx`, dan `NodeRenderer.tsx`. Gunakan katalog yang ada sebagai sumber placement.
2. Tambahkan infinite pan/zoom, snap 25.4 unit SVG (2.54 mm), drag-and-drop, dan namespace ID/referensi SVG. Uji beberapa LED identik serta anchor saat viewport/posisi berubah.
3. Verifikasi interaksi pada desktop target; jangan mengklaim acceptance kanvas dari typecheck saja.
4. Setelah Langkah 3 selesai, Langkah 4 menambah `WireLayer.tsx` dan netlist Rust: endpoint tervalidasi, routing Manhattan, warna kabel, serta hapus instance/kabel yang memperbarui graf. Belum ada resolusi listrik.
5. Simpan laporan `008` atau nomor berikutnya yang belum terpakai dan perbarui file ini. Jangan mengubah laporan lama untuk memasukkan hasil baru.

## Laporan

- [001 — Fase 1: pemeriksaan lingkungan dan aturan laporan](docs/reports/001-phase-1-environment.md)
- [002 — Fase 1: scaffold aplikasi dan hambatan dependency](docs/reports/002-phase-1-application-scaffold.md)
- [003 — Fase 1: verifikasi prasyarat native dan timeout build](docs/reports/003-phase-1-native-build-attempt.md)
- [004 — Fase 1: build native debug dan smoke test desktop](docs/reports/004-phase-1-native-build-smoke.md)
- [005 — Fase 1: kontrak paket, loader, dan katalog](docs/reports/005-phase-1-component-loader.md)

- [006 — Fase 1: publikasi repository GitHub](docs/reports/006-phase-1-github-publication.md)

- [007 — Fase 1: penyegaran README open source](docs/reports/007-phase-1-readme-refresh.md)

Template: [`docs/reports/TEMPLATE.md`](docs/reports/TEMPLATE.md). Nomor laporan berikutnya: `008` (periksa ulang sebelum membuat file).
