# 002 — Fase 1: Scaffold Aplikasi dan Hambatan Dependency

- **Tanggal:** 2026-11-06 (tanggal sesi agent)
- **Agent / pelaksana:** Zed coding agent
- **Fase dan langkah blueprint:** Fase 1, Langkah 1b — persiapan dependency dan fondasi aplikasi
- **Status subtahap:** `partial`
- **Status fase:** `blocked`; build native dan smoke test desktop belum lolos

## Tujuan dan batas cakupan

Melanjutkan persiapan lingkungan atas izin pengguna, membuat scaffold stack yang ditetapkan, dan memvalidasi pemeriksaan yang dapat dijalankan. Tidak melanjutkan ke loader, kanvas, netlist, Lua, emulasi MCU, atau distribusi RPM/Flatpak.

## Hasil dan perubahan

- Memeriksa ulang dokumen handoff serta roadmap; memeriksa prerequisite dan konfigurasi pada dokumentasi resmi Tauri v2.
- Mengonfirmasi paket yang belum terpasang menggunakan RPM, lalu memverifikasi ketersediaan nama paket utama melalui metadata lokal DNF Fedora 45.
- Mencoba instalasi dependency dengan sudo; tidak berhasil karena autentikasi interaktif diperlukan. Tidak ada perubahan paket sistem yang berhasil dilakukan.
- Memasang dependency npm proyek dan menghasilkan `package-lock.json`; npm melaporkan 0 vulnerability pada saat instalasi, bukan jaminan keamanan permanen.
- Membuat shell statis dark-mode dengan area katalog kosong dan penjelasan tahap aktif. Tidak ada komponen hardware hard-coded atau tombol fitur palsu.

### File

| Area | File |
| --- | --- |
| Konfigurasi frontend | `package.json`, `package-lock.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `.gitignore` |
| Shell SolidJS | `src/index.tsx`, `src/App.tsx`, `src/styles.css` |
| Fondasi Rust/Tauri | `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/error.rs` |
| Ikon aplikasi | `src-tauri/icons/icon.png` — PNG RGBA 128×128, dibuat lokal |
| Pemeriksaan scaffold | `scripts/scaffold.test.mjs` — 3 tes Node bawaan, tanpa framework tambahan |
| Dokumentasi | `README.md`, pembaruan `PROGRESS.md`, dan laporan ini |

`node_modules/` dan `dist/` dihasilkan lokal serta diabaikan melalui `.gitignore`. Tidak membuat commit, branch, staging, atau direktori root kedua.

## Keputusan teknis

- npm `10.9.8` dengan versi dependency langsung yang eksplisit dan lockfile. Versi registry diperiksa sebelum dipilih; Node `22.23.1` memenuhi engine Vite yang dipilih.
- SolidJS `1.9.18`, TypeScript `7.0.2`, Vite `8.3.6`, vite-plugin-solid `2.11.15`, Tailwind CSS dan plugin Vite `4.3.4`, CLI Tauri `2.12.4`.
- Crate Rust diverifikasi terpisah dari versi CLI: tauri `2.12.5`, tauri-build `2.7.4`, thiserror `2.0.21`. Manifest memilih Edition 2021 dan minimum Rust `1.90`. Toolchain aktual serta dependency transitif Rust **belum terkunci** karena Cargo belum tersedia.
- Jalur instalasi yang diusulkan memakai paket Rust Fedora. Rustup bukan tambahan wajib; jangan mencampur keduanya tanpa keputusan PATH yang jelas.
- App ID `org.nodus.Studio`, product name `Nodus Studio`, binary `nodus`; error startup memakai `Result` dan `thiserror`, tanpa `.unwrap()`/`.expect()` pada source produksi yang ditulis.
- Vite memakai `127.0.0.1:1420` dengan strict port. Tauri menunjuk URL yang sama dan `../dist` untuk build produksi.
- CSP produksi aktif tanpa `unsafe-eval`/`unsafe-inline`; CSP dev mengizinkan inline style Vite dan koneksi HMR lokal. Tidak mengaktifkan global API, asset protocol, native plugin, remote capability, atau akses filesystem frontend. Runtime CSP belum diuji di WebKitGTK.
- Tidak menambahkan `@tauri-apps/api`, Lua, Tokio, atau emulator sebelum ada kebutuhan implementasi pada langkahnya. CLI info akan menampilkan API JS belum terpasang; shell ini tidak memakai IPC.
- HTML drag-and-drop tidak diblokir handler native (`dragDropEnabled: false`) untuk kebutuhan kanvas mendatang; fitur drag belum diimplementasikan. Packaging dimatikan sampai tahap distribusi yang sesuai.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `sudo -n true` di sandbox | blocked | `no new privileges` mencegah elevasi |
| `rpm -q rust cargo webkit2gtk4.1-devel openssl-devel librsvg2-devel libappindicator-gtk3-devel libxdo-devel gcc-c++ make` | passed | Pemeriksaan membedakan paket yang belum ada dari GCC C++ dan make yang terpasang; exit query nonzero karena paket hilang |
| `sudo -n dnf install -y rust cargo rustfmt clippy webkit2gtk4.1-devel openssl-devel librsvg2-devel libappindicator-gtk3-devel libxdo-devel` di luar sandbox | blocked | Exit 1: `sudo: a password is required`; tidak meminta/merekam kata sandi |
| `dnf --cacheonly list --available rust cargo rustfmt clippy webkit2gtk4.1-devel openssl-devel librsvg2-devel libappindicator-gtk3-devel libxdo-devel` | passed | Semua nama paket ditemukan di metadata Fedora 45 lokal; Rust/Cargo tersedia sebagai `1.99.1-2.fc45`, belum diinstal |
| `npm view ...` dan query API crates.io | passed | Versi dan kebutuhan minimum diperiksa; ini bukan bukti instalasi Rust |
| `npm install --no-fund` | passed | Dependency lokal dan lockfile dibuat; audit saat itu melaporkan 0 vulnerability |
| `npm run build` | passed | TypeScript typecheck dan Vite production build selesai; JS 11.73 kB, CSS 9.06 kB sebelum gzip |
| `npm test` | passed | 3/3 tes konfigurasi lolos; bukan tes perilaku browser atau Rust |
| `npm run tauri -- info` | passed | Diagnostik mengenali SolidJS/Vite dan konfigurasi; tetap melaporkan prasyarat native/Rust hilang, bukan kelulusan lingkungan |
| `npm run tauri -- build --no-bundle` | failed | Berhenti saat `cargo metadata`: Cargo tidak ditemukan, OS error 2; source Rust belum dikompilasi |
| Diagnostics editor | passed | Tidak ada error/warning yang dilaporkan; bukan pengganti compiler Rust |
| Review read-only scaffold oleh subagent | passed | Tidak menemukan masalah actionable; tetap menyatakan native runtime belum terverifikasi |
| Pemeriksaan Python JSON/TOML, lockfile, tautan/fence dokumen, urutan laporan, panjang source, dan larangan unwrap | passed | Identitas konsisten, dependency langsung cocok dengan lockfile, tautan lokal valid, laporan 001–002 berurutan, source di bawah 400 baris; bukan kompilasi Rust |
| Smoke test jendela desktop / CSP WebKitGTK | not-run | Toolchain dan dependency native belum tersedia |

## Hambatan dan risiko

- Persetujuan pengguna sudah ada, tetapi agent tidak memiliki autentikasi sudo. Pengguna perlu menjalankan command instalasi dalam `README.md` pada terminal host interaktif.
- Belum ada `src-tauri/Cargo.lock`; jangan mengklaim build Rust reproducible atau kompatibilitas runtime sebelum Cargo benar-benar berjalan.
- Tiga tes yang lulus adalah guard konfigurasi, bukan bukti jendela Tauri, ikon, atau CSS tampil benar. Variabel sesi GNOME/Wayland juga bukan bukti visual aplikasi.
- Belum ada acceptance criterion keseluruhan Fase 1 yang dinyatakan lengkap; jangan membuka pekerjaan Fase 2.

## Handoff: langkah berikutnya

1. Instal prasyarat dari terminal host interaktif sesuai `README.md`; jangan kirim kata sandi kepada agent.
2. Periksa versi `rustc`/`cargo` dan modul native, lalu jalankan Cargo untuk menghasilkan lockfile asli. Catat toolchain aktual dan evaluasi pematokannya tanpa membuat instalasi lain otomatis.
3. Jalankan pemeriksaan Rust yang relevan dan `npm run tauri -- build --no-bundle`. Perbaiki hanya masalah fondasi yang teramati.
4. Jalankan `npm run tauri -- dev` secara interaktif, verifikasi shell di jendela GNOME/Wayland, lihat error console/CSP bila ada, dan catat hasil aktual. Server/watch tidak dijalankan tanpa batas oleh tool agent.
5. Setelah gate desktop lulus, lanjut kontrak dan loader `.nodpkg` sesuai blueprint. Buat laporan `003` untuk kelanjutan; jangan menulis ulang hasil historis laporan ini atau `001`.

## Kelengkapan laporan

- [x] Pekerjaan selesai dibedakan dari instalasi yang gagal dan pemeriksaan yang belum dilakukan.
- [x] File, command, bukti, keputusan, serta batas validasi dicatat.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
