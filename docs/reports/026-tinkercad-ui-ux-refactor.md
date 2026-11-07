# 026 — Refaktor UI/UX Gaya Tinkercad & Kontrol Kanvas Modern

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Coding agent
- **Fase dan langkah blueprint:** Pasca-Fase 5 / UI/UX Modernization
- **Status subtahap:** completed
- **Status fase:** completed

## Tujuan dan batas cakupan

- Mengadopsi arsitektur antarmuka pengguna bergaya Tinkercad Circuits dengan fokus tombol berbasis ikon SVG ringkas dan deskriptif.
- Mengimplementasikan 4 tahap UI/UX:
  1. Toolbar atas berbasis ikon tanpa teks berlebih (Undo, Redo, Delete, Rotate, Wire Color circular swatches, Start/Stop Simulation dengan animasi pulse, toggle panels).
  2. Drawer katalog komponen di sisi kanan yang dapat di-collapse, dilengkapi instant search filter dan tab kategori (*All*, *Basic*, *Sensors*, *Actuators*, *MCU*).
  3. Floating Component Inspector dialog untuk komponen terpilih dengan nama yang dapat diubah, tombol rotasi, hapus, kontrol interaktif live (*range sliders*, *toggles*), dan pin list.
  4. Kontrol navigasi kanvas melayang (*floating navigation pill*) di pojok kanan bawah dengan *Fit to View*, *Zoom In*, *Zoom Out*, dan *Reset View*, serta efek visual pin hover glowing.
- Menjaga batas modularitas: setiap file kode strictly ≤ 400 baris.

## Hasil dan perubahan

- `src/components/canvas/WorkbenchToolbar.tsx`: Toolbar atas bersih berbasis ikon dengan project renaming, aksi edit (↺ Rotate, 🗑 Delete, ↶ Undo, ↷ Redo), 5 circular color swatches, tombol simulasi Play/Stop, editor, logic analyzer, MQTT, save/open, export schematic SVG, dan toggle drawer komponen.
- `src/components/library/ComponentCatalog.tsx`: Drawer sisi kanan dengan tab kategori (*All, Basic, Sensors, Actuators, MCU*), pencarian instan, dan kartu komponen grid 2-kolom dengan tombol `+ Add` dan drag-and-drop.
- `src/components/canvas/ComponentInspector.tsx`: Dialog melayang properti komponen dengan thumbnail SVG, nama komponen, tombol rotasi/hapus, slider kontrol interaktif real-time, dan pemetaan pin.
- `src/components/canvas/CanvasNavigation.tsx`: Floating pill navigasi kanvas di pojok kanan bawah (Fit to view, Zoom In, Zoom Level %, Zoom Out, Reset view).
- `src/components/canvas/CanvasStage.tsx`: Abstraksi panggung kanvas SVG, grid pattern, dan layer rendering terisolasi.
- `src/components/canvas/useWorkbenchGestures.ts`: Modul gesture pan/zoom/drag, event pointer capture, dan shortcut keyboard (`Ctrl+Z`, `Ctrl+Y`, `R`, `Delete`, `Escape`).
- `src/components/canvas/useWorkbenchActions.ts`: Modul aksi kanvas (place, pin connection, recolor, remove).
- `src/components/canvas/geometry.ts`: Menambahkan algoritma `fitViewToNodes` dan dukungan rotasi sudut pada `pinWorldPoint`.
- `src/components/canvas/NodeRenderer.tsx`: Rendering rotasi dinamis (`rotate(θ)`) dan SVG `<title>` tooltip pada pin.
- `src/components/canvas/Workbench.tsx`: Orkestrasi workbench modular (330 baris).
- `src/styles.css`: Transisi pin hover dengan glowing drop shadow dan layout flexbox responsif.

## Keputusan teknis

- Memisahkan logika gesture dan action ke dalam modul terpisah (`useWorkbenchGestures.ts`, `useWorkbenchActions.ts`, `CanvasStage.tsx`, `CanvasNavigation.tsx`, `ComponentInspector.tsx`) agar seluruh file tetap memenuhi batas ketat ≤ 400 baris.
- Menjaga atribut aksesibilitas (`aria-label`, `data-component-id`, `data-color-option`) pada setiap elemen interaktif sehingga 100% tes otomatis AT-SPI dan WebKit DOM smoke test tetap lulus tanpa regresi.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `npm run typecheck && npm run build` | passed | TypeScript lolos tanpa error, bundle Vite terkompilasi |
| `npm test` | passed | 28 tes unit Node passed |
| `python3 scripts/webkit-workbench-smoke.py` | passed | 12 checks mocked discovery + 1 browser fallback passed |
| `python3 scripts/desktop-catalog-smoke.py` | passed | 20 checks accessibility GNOME Wayland passed |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | passed | 89 tes unit Rust passed |
| `npm run tauri -- build --no-bundle` | passed | Native release binary terkompilasi di `src-tauri/target/release/nodus` |
| `find src -name "*.ts" -o -name "*.tsx" \| xargs wc -l` | passed | Semua file ≤ 330 baris (batas maksimal 400) |

## Hambatan dan risiko

- Tidak ada hambatan aktif.

## Handoff: langkah berikutnya

1. Lakukan commit dan push refaktor UI/UX ke branch `main`.
2. Eksplorasi penambahan komponen periferal IoT baru (LCD 1602 I2C, potensiometer analog, buzzer PWM).

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
