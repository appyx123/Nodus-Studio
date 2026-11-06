# 004 — Fase 1: Build Native Debug dan Smoke Test Desktop

- **Tanggal:** 2026-11-06 (tanggal sesi agent)
- **Agent / pelaksana:** Zed coding agent
- **Fase dan langkah blueprint:** Fase 1, Langkah 1b — menyelesaikan verifikasi fondasi aplikasi
- **Status subtahap:** `completed` untuk scaffold, native debug build, dan smoke test terbatas
- **Status fase:** `in-progress`; loader, kanvas interaktif, dan netlist belum dibuat

## Tujuan dan batas cakupan

Melanjutkan build dengan batas 15 menit yang disetujui pengguna, memeriksa Rust, dan memverifikasi startup serta konten shell pada desktop Fedora GNOME/Wayland. Tidak mengerjakan loader, simulasi, atau distribusi aplikasi.

## Hasil dan perubahan

- Resolusi dan unduhan dependency pertama selesai; native Tauri debug build berhasil. Cargo melaporkan 8m14s untuk build; durasi ini bukan total semua pemeriksaan sesi.
- `src-tauri/Cargo.lock` dihasilkan oleh Cargo: format v4, 407 entri termasuk paket aplikasi. Tidak ditulis manual dan tidak diabaikan oleh `.gitignore`.
- Binary tersedia di `src-tauri/target/debug/nodus`. Binary debug berukuran 199,925,208 byte saat diperiksa; ini bukan ukuran paket rilis.
- Format, test harness Rust, serta Clippy lulus. Test harness berisi **0 tes Rust**, sehingga tidak membuktikan perilaku loader/mesin yang memang belum diimplementasikan. Tiga tes konfigurasi Node tetap lulus.
- Startup shell diuji sebagai child process dengan waktu tunggu berbatas, kemudian dihentikan hanya pada proses uji tersebut. Tidak menjalankan Vite/watch tanpa batas.
- AT-SPI melalui `pyatspi` yang sudah tersedia di sistem menemukan aplikasi berdasarkan PID child, bukan membaca isi aplikasi pengguna lain. Frame `Nodus Studio`, dokumen WebKit, heading, dan teks SolidJS ditemukan. Tidak mengambil screenshot desktop.
- Pembaruan dokumentasi: `README.md`, `PROGRESS.md`, dan laporan ini. Source aplikasi, versi dependency langsung, CSP, serta privilege tidak diubah. Cache/build output dan schema Tauri yang dihasilkan tetap diabaikan sesuai `.gitignore`.

## Keputusan teknis

- Timeout sebelumnya adalah keterbatasan durasi percobaan awal, bukan alasan mengganti versi atau stack. Cache yang sudah ada digunakan kembali.
- Tetap menggunakan toolchain Fedora Rust/Cargo 1.99.1. Dependency transitif kini terkunci; toolchain aktual dicatat tetapi tidak dipatok melalui rustup. Minimum Rust 1.90 pada manifest belum diuji dengan compiler 1.90.
- Native debug build memakai asset frontend hasil before-build hook; smoke test binary tidak bergantung pada server Vite.
- Build serta keterlihatan konten shell cukup untuk menutup langkah fondasi pada cakupan ini. Verifikasi visual piksel, semua aturan CSP, interaksi kanvas, release build, dan pemaketan tidak dianggap selesai.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `CARGO_NET_RETRY=1 CARGO_HTTP_TIMEOUT=45 npm run tauri -- build --no-bundle --debug` | passed | Timeout tool 900 detik; frontend typecheck/build dan native compilation selesai; menghasilkan binary debug |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust diterima |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | Unit harness lib/bin dan doc-tests selesai, semuanya 0 tes; bukan cakupan perilaku backend |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Tidak ada warning/error Clippy; tidak melakukan unduhan baru |
| `npm test` | passed | 3/3 tes konfigurasi lulus |
| Startup binary dengan Python subprocess, tunggu maksimum 12 detik lalu terminate child | passed | Proses tetap hidup 12 detik; stdout/stderr kosong; pemeriksaan proses ini saja tidak membuktikan UI |
| Startup + traversal AT-SPI berbatas 15 detik, filter berdasarkan PID yang diluncurkan | passed | Frame `Nodus Studio`, document WebKit, heading `A foundation for connected ideas.`, katalog kosong, checkpoint, dan footer SolidJS ditemukan; proses uji ditutup |
| Pemeriksaan layout/atribut teks heading via AT-SPI | passed | Bounds `(498, 359, 576, 44)`, `invisible:false`, size `27pt`, weight `600`, foreground `244,244,245`, family `Inter`; konsisten dengan style heading, bukan pemeriksaan semua piksel |
| Parse lockfile menggunakan Python `tomllib`, hash SHA256, dan ukuran binary | passed | 407 entri; hash lockfile `1b671a4e61cbd0e151ef9107309e7b4390ff70bd2e5c1d0cb6f6e47c5284e11d` pada pemeriksaan tahap ini |
| Pemeriksaan visual manual/screenshot dan audit CSP runtime lengkap | not-run | Bukti terbatas pada startup, accessibility content/layout/atribut; tidak mengklaim pixel-perfect atau seluruh kebijakan CSP teruji |
| Build release/RPM/Flatpak | not-run | Di luar cakupan tahap ini; tidak ada paket distribusi dibuat |

## Hambatan dan risiko

- Tidak ada blocker aktif untuk memulai kontrak/loader Fase 1 Langkah 2 ketika diminta pengguna.
- Belum ada tes perilaku backend, loader, sanitasi paket, atau rangkaian. Tes baru wajib mengikuti implementasi fitur berikutnya.
- Smoke test membuktikan shell berjalan dan kontennya dimuat pada desktop target; bukan pengujian penuh aksesibilitas, semua styling, atau semua interaksi.
- Build debug dan pemeriksaan dengan compiler Fedora 1.99.1 tidak menjamin release build atau kompatibilitas compiler minimum. Fase 1 secara keseluruhan belum selesai.

## Handoff: langkah berikutnya

1. Lanjut Fase 1 Langkah 2 pada permintaan berikutnya: kontrak manifest v1 dan visual LED, model Rust/TypeScript, serta strategi lokasi registry development/resource produksi.
2. Implementasikan loader/sanitasi dengan tes untuk paket valid, metadata/versi/ID/anchor tidak valid, batas ukuran, path traversal/symlink, serta SVG berbahaya. Gunakan Rust sebagai sumber kebenaran paket.
3. Buat fixture lengkap `basic-led.nodpkg`; file Lua wajib ada tetapi belum dieksekusi. Jangan memulai scheduler atau emulasi MCU.
4. Pertahankan Cargo.lock dan jalankan pemeriksaan relevan dengan `--locked`. Lanjutkan smoke/visual checks saat UI bertambah.
5. Buat laporan `005` atau nomor berikutnya yang belum terpakai; perbarui `PROGRESS.md`, bukan hasil historis laporan 001–004.

## Kelengkapan laporan

- [x] Debug build, smoke test terbatas, dan pekerjaan fase berikutnya dibedakan.
- [x] Command, hasil, jumlah tes, serta batas verifikasi dicatat.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
