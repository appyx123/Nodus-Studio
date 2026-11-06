# 003 — Fase 1: Verifikasi Prasyarat Native dan Percobaan Build

- **Tanggal:** 2026-11-06 (tanggal sesi agent)
- **Agent / pelaksana:** Zed coding agent
- **Fase dan langkah blueprint:** Fase 1, Langkah 1b — melanjutkan verifikasi fondasi desktop
- **Status subtahap:** `partial`
- **Status fase:** `blocked`; resolusi dependency online dan build desktop belum selesai

## Tujuan dan batas cakupan

Memverifikasi instalasi yang dilaporkan pengguna, kemudian memeriksa source dan mencoba build native. Tidak menambah fitur, mengubah dependency, menjalankan Lua, atau melewati gate desktop untuk membuat loader/kanvas.

## Hasil dan perubahan

- Rust/Cargo dan dependency native yang sebelumnya hilang sekarang terdeteksi. Tidak ada instalasi sistem tambahan dilakukan oleh agent pada tahap ini.
- Rust dan Cargo: `1.99.1`, paket Fedora `1.99.1-2.fc45`; rustfmt `1.10.0`, clippy `0.1.99`.
- pkg-config: WebKitGTK `2.54.2`, GTK3 `3.24.52`, OpenSSL `4.0.3`, librsvg `2.63.2`. RPM juga mengonfirmasi development package AppIndicator dan libxdo.
- Format Rust dan metadata manifest tanpa dependency lolos. Frontend production build/typecheck dan 3 tes konfigurasi kembali lolos.
- Pemeriksaan Rust online dan build Tauri debug masing-masing timeout setelah 120 detik saat mengambil indeks crates.io. Tidak teramati error source Rust; ini **bukan** bukti source sudah dikompilasi atau lolos.
- Pada pemeriksaan akhir belum ada `src-tauri/Cargo.lock`, arsip crate di cache, atau binary `nodus`. Cache indeks parsial terbentuk; tidak ada proses Cargo yang tertinggal.
- Memperbarui `README.md` dan `PROGRESS.md` agar tidak lagi menyatakan Rust/dependency sistem belum dipasang. Menambahkan laporan ini; laporan 001–002 tetap dipertahankan sebagai riwayat.
- Tidak mengubah source aplikasi, manifest versi dependency, atau konfigurasi keamanan.

## Keputusan teknis

- Tetap menggunakan Rust dari Fedora. Peringatan `rustup` pada Tauri CLI bukan kegagalan instalasi Rust, dan tidak menjadi alasan memasang toolchain kedua.
- Mempertahankan versi dependency; timeout unduhan belum membuktikan ketidakcocokan versi atau bug source.
- Percobaan build debug memakai `CARGO_NET_RETRY=0` dan `CARGO_HTTP_TIMEOUT=30` hanya pada command tersebut agar retry jaringan tidak memperpanjang setiap permintaan tanpa batas. Tidak mengubah konfigurasi Cargo global.
- Setelah timeout, kelanjutan dengan batas runtime lebih panjang diserahkan pada keputusan pengguna. Cache indeks yang sudah diperoleh dapat dipakai pada percobaan berikutnya; jangan menghapusnya.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `rustc --version`, `cargo --version`, `rustfmt --version`, `cargo clippy --version` | passed | Toolchain tersedia dengan versi di atas; command versi Clippy bukan eksekusi lint |
| `pkg-config --modversion webkit2gtk-4.1 gtk+-3.0 openssl librsvg-2.0` | passed | Semua empat modul ditemukan |
| `rpm -q rust cargo webkit2gtk4.1-devel openssl-devel librsvg2-devel libappindicator-gtk3-devel libxdo-devel` | passed | Semua paket yang ditanyakan terpasang |
| `npm run build && npm test` | passed | Typecheck dan Vite build berhasil; 3/3 tes konfigurasi; JS 11.73 kB, CSS 9.09 kB sebelum gzip |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format source diterima; dijalankan ulang terpisah setelah percobaan gabungan timeout |
| `cargo check --manifest-path src-tauri/Cargo.toml` (setelah fmt dalam command gabungan) | blocked | Timeout tool 120 detik pada indeks; log terakhir `Fetch 50 complete; 4 pending`; belum ada hasil kompilasi |
| `cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps --offline --format-version 1` | passed | Manifest/target dikenali; tidak memvalidasi resolusi atau kompilasi dependency |
| `curl -I --max-time 15 https://index.crates.io/config.json` | passed | HTTP 200; bukan bukti seluruh dependency selesai diunduh |
| `cargo generate-lockfile --manifest-path src-tauri/Cargo.toml --offline` | failed | Exit 101: cache indeks belum memiliki `reqwest` yang dibutuhkan Tauri; belum menghasilkan lockfile |
| GET indeks `https://index.crates.io/re/qw/reqwest` via curl dengan batas 15 detik | passed | HTTP 200, sekitar 6.69 detik; tidak membuktikan akar masalah transport tertentu |
| `npm run tauri -- info` | passed | Rust/Cargo/native terdeteksi; masih ada peringatan rustup yang memang tidak digunakan |
| `CARGO_NET_RETRY=0 CARGO_HTTP_TIMEOUT=30 npm run tauri -- build --no-bundle --debug` | blocked | Timeout tool 120 detik saat indeks; frontend hook lulus, log terakhir `Fetch 139 complete; 18 pending` |
| Pemeriksaan keberadaan lockfile/binary/cache dan `pgrep` untuk Cargo | passed | Lockfile/binary belum ada, 0 arsip crate, tidak ada proses Cargo tertinggal |
| Native binary, Clippy aktual, dan tes Rust | not-run | Dependency online belum terselesaikan; tidak boleh diklaim lulus |
| Jendela GNOME/Wayland, rendering CSS, dan CSP WebKitGTK | not-run | Belum ada binary native yang bisa diuji |

## Hambatan dan risiko

- Hambatan lama berupa Rust/paket sistem yang hilang sudah teratasi.
- Hambatan saat ini adalah pengambilan dependency awal belum selesai dalam jatah runtime tool. HTTP dapat diakses, tetapi belum ada bukti yang cukup untuk menyimpulkan gangguan spesifik jaringan, transport, atau registry.
- Tidak ada bukti build desktop atau acceptance gate jendela lulus; Fase 1 tetap belum selesai.
- Source Rust, dependency transitif, dan runtime WebKitGTK masih dapat memperlihatkan error setelah proses dependency selesai. Jangan menyamakan keberhasilan metadata/fmt/frontend dengan native build.

## Handoff: langkah berikutnya

1. Tawarkan kelanjutan build dengan batas waktu lebih panjang, misalnya 600–900 detik, atau pengguna menjalankan `npm run tauri -- build --no-bundle --debug` pada terminal host.
2. Setelah resolusi selesai, simpan Cargo.lock asli, catat dependency/toolchain, lalu jalankan pemeriksaan Rust/build/test/Clippy relevan. Gunakan `--locked` setelah lockfile tersedia.
3. Jika binary berhasil dibuat, verifikasi jendela desktop beserta tampilan CSS dan error CSP. Jangan menjalankan server/watch tanpa batas melalui tool agent.
4. Baru setelah fondasi desktop lolos, lanjut kontrak dan loader `.nodpkg` pada Fase 1 Langkah 2.
5. Catat kelanjutan dalam laporan `004` dan perbarui `PROGRESS.md`; jangan mengubah laporan ini seolah build sudah berhasil pada sesi ini.

## Kelengkapan laporan

- [x] Prasyarat terpasang dipisahkan dari build/runtime yang belum selesai.
- [x] Command, timeout, hasil gagal offline, serta batas bukti dicatat.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
