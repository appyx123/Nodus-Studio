# 020 — Fase 4: Display OLED SSD1306 (I2C) dan Emulasi GDDRAM

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Antigravity (Coding)
- **Fase dan langkah blueprint:** Fase 4, Langkah 3 (Display OLED SSD1306 I2C 128x64, emulasi command set, addressing mode, dan GDDRAM)
- **Status subtahap:** completed
- **Status fase:** in-progress

## Tujuan dan batas cakupan

Menyelesaikan Fase 4 Langkah 3:
1. Mengimplementasikan emulator perangkat keras SSD1306 controller di backend Rust (`src-tauri/src/engine/bus/ssd1306.rs`):
   - Dukungan protokol I2C slave (`I2cDevice`) pada alamat `0x3C` / `0x3D`.
   - Command set penuh: Display ON/OFF (`0xAF`/`0xAE`), Contrast (`0x81`), Normal/Inverse (`0xA6`/`0xA7`), addressing modes (Horizontal `0x00`, Vertical `0x01`, Page `0x02`), column bounds (`0x21`), page bounds (`0x22`), dan direct page/column address select (`0xB0`..`0xB7`, `0x00`..`0x1F`).
   - GDDRAM 1024 bytes ($128 \times 64$ bit, 8 page $\times$ 128 kolom) dengan translasi penulisan byte otomatis dan fungsi akses individual pixel `get_pixel(x, y)`.
2. Membangun paket komponen `components-registry/display-ssd1306.nodpkg`:
   - `manifest.toml`: Komponen display grafis OLED, 4 pin grid-aligned (`GND`, `VCC`, `SCL`, `SDA`), visual `screen`.
   - `graphic.svg`: Layout visual modul OLED 0.96" dengan area display $128 \times 64$ dan header pin I2C berjarak 2.54 mm.
   - `logic.lua`: Manajemen level pin I2C dan visualisasi aktif layar.
3. Memverifikasi kompatibilitas streaming perintah/data I2C dan integritas eksekusi command set.

## Hasil dan perubahan

- `src-tauri/src/engine/bus/ssd1306.rs`:
  - `Ssd1306Device`: Struktur emulator display SSD1306 dengan buffer GDDRAM 1024-byte, state machine multi-byte command parsing (`CmdState`), serta pelacakan kursor kolom/page dinamis.
  - Penanganan kontrol I2C: deteksi byte kontrol (Co/D-C#) untuk memisahkan aliran command (`control_byte & 0x40 == 0`) dan data GDDRAM (`control_byte & 0x40 != 0`).
  - Unit test komprehensif menguji command ON/OFF, set contrast, set addressing mode, penulisan bit GDDRAM, dan integrasi transfer bus I2C master/slave.
- `src-tauri/src/engine/bus/mod.rs`:
  - Mengekspor modul `ssd1306` dan konstanta `Ssd1306Device`, `SSD1306_WIDTH` (128), `SSD1306_HEIGHT` (64), `SSD1306_BUFFER_SIZE` (1024).
- `components-registry/display-ssd1306.nodpkg/`:
  - `manifest.toml`: Metadata modul SSD1306, rating tegangan 3.3V–5.5V, 4 pin (`GND`, `VCC`, `SCL`, `SDA`), visual `screen`.
  - `graphic.svg`: Ilustrasi board OLED hitam dengan pin header 2.54 mm dan layar kaca OLED.
  - `logic.lua`: Skrip Lua mendengarkan aktivitas bus I2C dan menyalakan display.
- `src-tauri/src/engine/runtime_tests.rs`:
  - Menyesuaikan jumlah paket katalog bawaan dari 7 menjadi 8.
- `src-tauri/src/sandbox/pkg_loader_tests.rs` & `scripts/desktop-catalog-smoke.py`:
  - Menyesuaikan daftar paket katalog terurut (`actuator-servo-sg90`, `basic-led`, `basic-power-source`, `basic-pushbutton`, `display-ssd1306`, `mcu-arduino-uno`, `sensor-dht22`, `sensor-hcsr04`).

## Keputusan teknis

1. **Emulasi Multi-Mode GDDRAM untuk Kompatibilitas Library Arduino**:
   - `Adafruit_SSD1306` menggunakan Horizontal Addressing Mode (`0x00`) dengan burst byte streaming penuh 1024 byte, sedangkan `U8g2` sering memanfaatkan Page Addressing Mode (`0x02`) per baris strip 8 piksel. Mengimplementasikan translasi kursor kolom/page untuk kedua mode menjamin interoperabilitas library tanpa modifikasi firmware.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 79 passed, 0 failed (termasuk unit test emulator SSD1306 dan integrasi bus I2C) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline -- -D warnings` | passed | Selesai tanpa warning |
| `npm test` | passed | 27 passed, 0 failed (kontrak nodpkg dan geometri/kanvas) |
| `npm run build` | passed | TypeScript dan Vite bundle selesai tanpa error |
| `/usr/bin/python3 scripts/desktop-catalog-smoke.py` | passed | 20 cek desktop GNOME/Wayland AT-SPI lolos dengan 8 paket terdaftar |
| `wc -l $(find src-tauri/src -name '*.rs')` | passed | Seluruh 43 file Rust berada di bawah batas 400 baris |

## Hambatan dan risiko

- Tidak ada blocker. Emulator SSD1306 dan paket komponen siap pakai.

## Handoff: langkah berikutnya

1. Langkah 3 Fase 4 telah selesai (`completed`).
2. Melanjutkan ke Fase 4 Langkah 4: Jembatan IoT Virtual (Virtual MQTT Bridge):
   - Klien MQTT internal di backend Rust (`rumqttc`).
   - Pemetaan pin MCU/net atau channel serial ke topik MQTT publik/lokal (`nodus/workbench/<id>/...`).
   - Panel UI konfigurasi koneksi MQTT broker (Host, Port, Topic Sub/Pub, QoS).
