# 008 — Fase 1: Kanvas Interaktif dan Instance Komponen

- **Tanggal:** 2026-11-06 (tanggal sesi agent)
- **Agent / pelaksana:** Zed coding agent
- **Fase dan langkah blueprint:** Fase 1, Langkah 3 — katalog dan kanvas interaktif
- **Status subtahap:** `completed` untuk implementasi kanvas dan cakupan verifikasi di bawah
- **Status fase:** `in-progress`; kabel dan netlist Rust belum dibuat

## Tujuan dan batas cakupan

Menghubungkan katalog yang sudah ada ke penempatan beberapa instance LED, pan/zoom, grid fisik, dan drag-and-drop. Menjaga anchor SVG serta identitas instance saat viewport dan posisi berubah. Penempatan masih lokal selama sesi; belum ada topologi rangkaian, penyimpanan proyek, Lua, scheduler, atau simulasi hardware.

## Hasil dan perubahan

- `src/components/canvas/geometry.ts`: transform screen/world, grid 25.4 unit, zoom berjangkar pada pointer dengan batas 0.2–4, dan konversi delta drag ke posisi dunia yang di-snap.
- `src/components/canvas/graphic.ts`: parse hanya SVG normalisasi Rust, import DOM tanpa `innerHTML`, namespace seluruh ID dengan `node-<UUID>--`, serta penyesuaian ukuran nested SVG ke extent viewBox. Penulisan ulang referensi lokal tidak mengubah allowlist paket Rust.
- `src/components/canvas/NodeRenderer.tsx`: SVG inline per instance dengan identitas, posisi, seleksi, fokus keyboard, dan event handler. Memo paket/ID mencegah parsing ulang graphic saat hanya posisi berubah.
- `src/components/canvas/Workbench.tsx`: canvas/grid, Add/drop dari katalog, pan tanpa batas dunia yang ditetapkan, zoom pointer/buttons, reset viewport, snap, drag, seleksi, arrow-key nudge, Delete/Backspace, serta tombol hapus. Pointer capture untuk input trusted; cancel/lost capture mengembalikan gesture awal. Maksimal 256 instance.
- `src/components/library/ComponentCatalog.tsx`: menerima resource bersama dari App; tombol Add juga menjadi sumber drag HTML dengan MIME `application/x-nodus-component`. Drop hanya menerima ID paket yang ada di katalog aktif, bukan SVG/path dari drag payload.
- `src/App.tsx`, `src/styles.css`: resource discovery bersama, layout sidebar/toolbar/canvas/status, empty state, serta style seleksi/fokus. Key list memakai instance ID stabil agar pergerakan tidak mengganti DOM/fokus komponen.
- `scripts/workbench-geometry.test.mjs`: 15 tes matematika menggunakan compiler TypeScript yang sudah terpasang dan direktori uji privat yang dibersihkan.
- `scripts/webkit-workbench-smoke.py`, `scripts/workbench-dom-checks.js`: harness WebKitGTK dengan discovery mock, input DOM sintetik, geometri/hit-test nyata, serta halaman fallback browser tanpa mock. Server loopback sementara dan WebView dibersihkan saat keluar.
- `scripts/desktop-catalog-smoke.py`: memperluas tes native PID-scoped dengan Add dua instance, identitas aksesibilitas, zoom/reset, delete, dan mempertahankan placement saat registry gagal. Tes isolasi paket dan resource relocation tetap dipakai.
- `README.md`, `docs/nodpkg-v1.md`, `PROGRESS.md`: status fitur, kontrol, kontrak koordinat, command validasi, keterbatasan, serta next action diperbarui.
- Tidak menambahkan dependency, mengubah lockfile/permission/CSP, menjalankan Lua, memasang paket sistem, atau melakukan staging/commit/push.

## Keputusan teknis

- `screen = viewport_translation + world * zoom`; koordinat screen relatif terhadap SVG canvas sesudah dikurangi bounding rectangle. SVG canvas tidak memakai viewBox, sehingga unit lokalnya mengikuti CSS pixel; nested SVG mempertahankan skala dunia paket. Satu unit paket = 0.1 mm; grid 25.4 unit = 2.54 mm.
- Posisi instance adalah origin viewport nested SVG. Add menempatkan origin dekat pusat tampilan dengan offset bertahap; drop menempatkannya di pointer lalu snap. Viewport awal `(60,40)`, zoom 2; Reset hanya mereset viewport, bukan menghapus instance.
- Instance memakai `node-<UUID v4>`, berbeda dari ID jenis paket. Instance menyimpan snapshot paket yang dipakai saat penempatan; Reload discovery tidak menghapus atau mengganti instance yang sudah ada.
- Canvas state saat ini hanya state presentasi frontend. Langkah 4 harus menambahkan registrasi instance dan koneksi yang authoritative di Rust; ini bukan implementasi netlist di JavaScript.
- Registry tetap sumber SVG yang tervalidasi. `DOMParser`/namespacing frontend bukan pengganti sanitizer Rust atau alasan menerima input SVG arbitrer.
- Canvas diuji dengan unit tests, WebKit DOM checks, dan native accessibility actions yang berbeda cakupannya; hasil sintetik tidak diklaim sebagai input fisik native.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `node --test scripts/workbench-geometry.test.mjs` terhadap stub awal, dijalankan subagent | failed → passed | Dilaporkan 11 kegagalan perilaku sebelum implementasi, lalu 15 lulus; hasil integrated suite diverifikasi ulang oleh parent |
| Percobaan awal harness pada UI sebelum kanvas | failed | Tombol Add belum tersedia; fallback browser sudah berfungsi |
| Regresi hit-test body SVG sebelum perbaikan | failed | Parent menjalankan harness: `elementFromPoint()` mengenai rect background, bukan LED; atribut `pointer-events="none"` pada SVG import menyebabkan masalah |
| Harness setelah menghapus atribut penyebab dan rebuild | passed | Body SVG menerima hit-test; pointerdown dari hasil hit-test dapat menggerakkan instance. Ditambah cek cancel dan fokus keyboard tetap bertahan |
| `npm test` setelah source akhir | passed | 20 tes Node: 5 konfigurasi/kontrak sebelumnya + 15 geometri |
| `npm run build` | passed | Typecheck TypeScript dan Vite production build |
| `python3 scripts/webkit-workbench-smoke.py` setelah build source akhir | passed | 12 checks mocked-discovery: readiness/layout, dua UUID dan namespace ID, hit-test, cancel gesture, wheel anchor, drag, pan, zoom bounds/reset, MIME/drop/unknown ID, keyboard/fokus/delete, delete toolbar; 1 check browser fallback tanpa mock |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Tidak ada perubahan source Rust |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 34 tes Rust loader/manifest/SVG tetap lulus |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Semua target tanpa warning |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --no-bundle --debug` setelah perbaikan hit-test | passed | Native binary dengan asset akhir; compilation 10.69 detik |
| `python3 scripts/desktop-catalog-smoke.py` setelah native build akhir | passed | Tauri IPC/resource discovery sebenarnya, SVG preview, dua Add dan identitas berbeda, zoom/reset, hapus satu instance, isolasi paket invalid, Reload, registry hilang dan restorasi tanpa kehilangan instance tersisa |
| `git diff --check`, pemeriksaan tautan lokal/anchor dan batas source | passed | Dokumentasi valid secara statis; source ≤400 baris; direktori uji sementara dibersihkan |
| Drag/drop fisik, trusted pointer capture/lost capture, touch/trackpad, audit visual piksel | not-run | Input WebKit dipicu sintetik; native checks memakai action aksesibilitas. Hit-test/geometri nyata membantu cakupan, tetapi tidak menggantikan pemeriksaan input fisik |
| Stress test 256 instance dan SVG kompleks maksimum | not-run | Guard batas ada, tetapi belum benchmark performa atau batas threshold secara runtime |
| Release/RPM/Flatpak dan compiler minimum Rust 1.90 | not-run | Di luar cakupan; compiler aktual tetap Fedora Rust/Cargo 1.99.1 |

## Hambatan dan risiko

- Tidak ada blocker aktif untuk Langkah 4. Fase 1 belum memenuhi acceptance kabel/netlist dan belum boleh dianggap selesai.
- Penempatan hilang saat app ditutup/reload halaman; library Reload berbeda dan mempertahankan instance. Jangan menyebutnya penyimpanan proyek.
- Manual desktop check untuk drag/drop fisik dan pointer capture tetap diperlukan. Tes DOM sintetik tidak menghasilkan native pointer capture meskipun source menanganinya untuk event trusted.
- Tidak ada performa 60 FPS yang dijanjikan. Batas 256 instance bukan hasil benchmark; simulasi dan frame streaming belum diimplementasikan.
- Seluruh batas threat model filesystem/SVG di kontrak paket dan laporan 005 tetap berlaku. Dukungan referensi/efek SVG kaya masih ditolak validator Rust.
- Tampilan GitHub/README, desktop pixel QA, dan distribusi belum divalidasi pada tahap ini. Perubahan masih lokal, belum commit/push.

## Handoff: langkah berikutnya

1. Jika tersedia, lakukan manual desktop check: Add dua LED, drag body/pin dekat tepi viewport, lepas pointer di luar canvas, pan, zoom sambil hover, drag katalog ke canvas, lalu ulangi pada touchpad. Catat hasil nyata tanpa mengklaim tes sintetik sebagai input fisik.
2. Lanjut Fase 1 Langkah 4: tetapkan kontrak instance/endpoint/topologi authoritative Rust sebelum menambahkan command IPC. Identitas komponen harus berbeda dari identitas instance; backend memvalidasi jenis paket dan pin sebelum koneksi.
3. Tambahkan WireLayer dengan koneksi klik pin A ke B, routing Manhattan, warna kabel, serta penghapusan kabel/instance yang memperbarui graf Rust. Pertahankan anchor world/screen dari nested SVG dan namespace per instance; uji round-trip setelah pan/zoom/drag.
4. Lengkapi tes netlist Rust dan interaksi native sebelum menilai seluruh acceptance Fase 1. Jangan menjalankan Lua/scheduler/resolusi listrik pada fase ini.
5. Simpan laporan 009 atau nomor global berikutnya yang belum digunakan dan perbarui PROGRESS. Jangan commit/push tanpa instruksi pengguna.

## Kelengkapan laporan

- [x] Implementasi Langkah 3 dibedakan dari sisa acceptance Fase 1.
- [x] Perubahan/file, keputusan, command, hasil aktual, dan batas bukti dicatat.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
