# 019 — Fase 4: Paket Komponen Sensor & Aktuator Canggih (DHT22, HC-SR04, SG90)

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Antigravity (Coding)
- **Fase dan langkah blueprint:** Fase 4, Langkah 2 (Paket Komponen Sensor & Aktuator Canggih: `sensor-dht22.nodpkg`, `sensor-hcsr04.nodpkg`, dan `actuator-servo-sg90.nodpkg`)
- **Status subtahap:** completed
- **Status fase:** in-progress

## Tujuan dan batas cakupan

Menyelesaikan Fase 4 Langkah 2:
1. Membangun dan meregistrasikan 3 paket komponen `.nodpkg` v1 interaktif baru:
   - `components-registry/sensor-dht22.nodpkg`: Sensor suhu & kelembapan digital 1-wire interaktif (slider suhu -40°C..80°C, kelembapan 0%..100%).
   - `components-registry/sensor-hcsr04.nodpkg`: Sensor jarak ultrasonik trigger/echo (slider jarak objek 2 cm..400 cm, kecepatan suara 58 µs/cm).
   - `components-registry/actuator-servo-sg90.nodpkg`: Motor servo mikro kendali PWM 50 Hz (pulsa 1 ms..2 ms = 0°..180°) dengan visual rotasi horn dinamis.
2. Memperluas skema manifest `.nodpkg` secara backward-compatible dengan dukungan visual property `rotation` (`VisualProperty::Rotation` pada Rust dan `VisualProperty = "opacity" | "rotation"` pada TypeScript).
3. Mengimplementasikan dukungan rotasi grafis SVG dinamis pada frontend (`NodeRenderer.tsx`) dengan perhitungan pivot bounding box otomatis.
4. Memvalidasi seluruh fungsionalitas dengan pengujian unit, integrasi runtime Lua/elektris Rust, WebKit DOM smoke, dan GNOME/Wayland desktop smoke.

## Hasil dan perubahan

- `src-tauri/src/sandbox/manifest.rs` & `src/types/nodpkg.ts`:
  - Menambahkan varian `Rotation` pada enum `VisualProperty` (`#[serde(rename_all = "snake_case")]`).
  - Menyesuaikan validasi range manifest: properti `opacity` dibatasi pada range `0.0..=1.0`, sedangkan properti `rotation` diizinkan pada range sudut `-360.0..=360.0`.
- `src/components/canvas/NodeRenderer.tsx`:
  - Menambahkan penanganan `v.property === "rotation"` yang mengambil bounding box (`getBBox()`) elemen target untuk merotasi horn servo secara presisi terhadap titik pusat porosnya (`rotate(angle cx cy)`).
- `components-registry/sensor-dht22.nodpkg/`:
  - `manifest.toml`: Definisi komponen DHT22 dengan 4 pin (`VCC`, `DATA`, `NC`, `GND`), slider `temperature` (-40°C..80°C) dan `humidity` (0%..100%), serta visual `dyn-activity`.
  - `graphic.svg`: Ilustrasi sensor DHT22 dengan grid ventilasi plastik putih dan pin header 2.54 mm.
  - `logic.lua`: Emulasi protokol 1-wire DHT22: mendeteksi pulsa inisiasi LOW dari master MCU (>= 800 µs), mengirim respon 80 µs LOW + 80 µs HIGH, dilanjutkan 40 bit data (16-bit kelembapan, 16-bit suhu, 8-bit checksum) menggunakan pemanggilan timer bertahap (`dht_step`).
- `components-registry/sensor-hcsr04.nodpkg/`:
  - `manifest.toml`: Definisi sensor jarak HC-SR04 dengan 4 pin (`VCC`, `TRIG`, `ECHO`, `GND`), slider `distance` (2 cm..400 cm), dan visual `dyn-pulse`.
  - `graphic.svg`: Ilustrasi PCB biru dengan dua transduser ultrasonik logam (T & R) dan pin header 2.54 mm.
  - `logic.lua`: Mendeteksi pulsa TRIG master (>= 8 µs), mensimulasikan jeda akustik internal 200 µs, lalu memancarkan pulsa ECHO HIGH dengan durasi `distance_cm * 58` µs sesuai konstanta perambatan gelombang ultrasonik di udara.
- `components-registry/actuator-servo-sg90.nodpkg/`:
  - `manifest.toml`: Definisi motor servo SG90 dengan 3 pin (`PWM`, `VCC`, `GND`) dan visual target `dyn-horn` berproperti `rotation` (0°..180°).
  - `graphic.svg`: Ilustrasi casing biru servo SG90, lubang sekrup mounting, kabel 3-warna (SIG/VCC/GND), dan horn putih bersimetri radial pada `dyn-horn`.
  - `logic.lua`: Mengukur durasi pulsa HIGH pada pin `PWM` (`timestamp_ns - pwm_start`) dan memetakan rentang pulsa 1.0 ms..2.0 ms ke sudut 0.0°..180.0° untuk memperbarui visual horn.
- `src-tauri/src/engine/runtime_tests.rs`:
  - Menambahkan pengujian `sensors_and_actuators_operate_with_expected_timing_and_visuals` yang memvalidasi operasi runtime servo SG90, HC-SR04, dan DHT22.
  - Menyesuaikan jumlah paket katalog terdaftar dari 4 menjadi 7.
- `src-tauri/src/sandbox/pkg_loader_tests.rs` & `scripts/desktop-catalog-smoke.py`:
  - Menyesuaikan ekspektasi pemuatan 7 paket terurut (`actuator-servo-sg90`, `basic-led`, `basic-power-source`, `basic-pushbutton`, `mcu-arduino-uno`, `sensor-dht22`, `sensor-hcsr04`).

## Keputusan teknis

1. **Rotasi Presisi via `getBBox()`**:
   - Memanfaatkan kalkulasi titik tengah `cx = bbox.x + bbox.width / 2` dan `cy = bbox.y + bbox.height / 2` dari elemen SVG `dyn-horn` memungkinkan rotasi poros servo berputar sempurna tanpa memerlukan metadata koordinat manual atau pelanggaran aturan sanitasi SVG.
2. **Penjadwalan Timer Bertahap (Single-Timer Sequencer)**:
   - Protokol 1-wire DHT22 memiliki lebih dari 80 transisi sinyal. Menjadwalkan timer secara berantai (1 timer aktif pada satu saat melalui fungsi callback `dht_step`) mencegah pelanggaran batas aman `MAX_TIMERS_PER_INSTANCE` (64 timer).

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 77 passed, 0 failed (seluruh rangkaian pengujian Rust termasuk verifikasi runtime servo, HC-SR04, dan DHT22) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline -- -D warnings` | passed | Selesai tanpa warning |
| `npm test` | passed | 27 passed, 0 failed (seluruh pengujian kontrak TypeScript dan modul geometri/kanvas) |
| `npm run build` | passed | Kompilasi TypeScript dan bundler Vite produksi selesai tanpa error |
| `/usr/bin/python3 scripts/webkit-workbench-smoke.py` | passed | 13 cek DOM WebKitGTK lolos |
| `/usr/bin/python3 scripts/desktop-catalog-smoke.py` | passed | 20 cek AT-SPI GNOME/Wayland desktop binary lolos (7 paket loaded, sim lifecycle, wiring, reload) |
| `wc -l $(find src-tauri/src -name '*.rs')` | passed | Seluruh 42 file Rust berada di bawah batas 400 baris |

## Hambatan dan risiko

- Tidak ada blocker. Tiga paket komponen baru telah lulus uji statik dan uji dinamis.

## Handoff: langkah berikutnya

1. Langkah 2 Fase 4 telah selesai (`completed`).
2. Melanjutkan ke Fase 4 Langkah 3: Paket Komponen Layar OLED SSD1306 (`display-ssd1306.nodpkg`):
   - Komponen display grafis monokrom $128 \times 64$ piksel via protokol I2C (alamat `0x3C` / `0x3D`).
   - Framebuffer virtual terhubung ke stream visualisasi frontend.
   - Kompatibilitas dengan library Arduino standar (`Adafruit_SSD1306` / `U8g2`).
