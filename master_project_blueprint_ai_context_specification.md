# MASTER BLUEPRINT & AI DIRECTIVE: NODUS STUDIO (OPEN-SOURCE IoT SIMULATOR)

> **PERINGATAN KERAS UNTUK AI ASSISTANT (BACA SEBELUM MENULIS KODE):**
> Dokumen ini adalah **Single Source of Truth (SSOT)** untuk proyek **Nodus Studio**. Sebagai AI Engineer/Assistant, kamu **DILARANG KERAS**:
>
> 1. Mengubah tech stack yang sudah ditetapkan di bawah (misal: mengganti SolidJS ke React, atau mengganti Tauri ke Electron).
>
> 2. Menulis logika simulasi perangkat keras atau emulasi pin di sisi Frontend (JavaScript/TypeScript). Semua logika simulasi, *clock tick*, dan protokol **WAJIB** berjalan di Backend (Rust).
>
> 3. Mengirimkan perubahan *state* pin per-*tick* mikrokontroler ($16\text{ MHz}$) secara langsung melalui JSON IPC standar. Komunikasi *state* visual dari Rust ke Frontend **WAJIB** di-*batch* menggunakan *binary buffer* tersinkronisasi pada kecepatan *frame rate* layar ($60\text{ FPS}$).
>
> 4. Membuat komponen IoT secara *hard-coded* di dalam kode sumber utama. Semua komponen (sensor, aktuator, layar, MCU) **WAJIB** mengikuti standar modular `.nodpkg` yang didefinisikan di dokumen ini.
>
> 5. Melompati fase pengerjaan. Kerjakan hanya modul yang diminta oleh pengguna sesuai urutan *Roadmap*.

## 1. Visi, Filosofi & Identitas Proyek

* **Nama Proyek (Core Engine):** **Nodus**
* **Nama Aplikasi Desktop:** **Nodus Studio**
* **Filosofi Nama:** Berasal dari bahasa Latin *Nodus* yang berarti **"simpul"** (*node* / *knot*). Nama ini merepresentasikan tiga pilar utama aplikasi:
  1. **Circuit Node:** Titik simpul pertemuan arus dan komponen elektronika di atas kanvas.
  2. **IoT Network Node:** Perangkat pintar yang saling terhubung dalam jaringan protokol IoT (MQTT, HTTP, WebSocket).
  3. **Community Node:** Titik temu kolaborasi komunitas *open-source* yang bersama-sama membangun pustaka perangkat keras virtual terlengkap di dunia.
* **Identitas Teknis & Ekosistem Linux:**
  * **Repository / Root Directory (nama kanonis):** `nodus-studio`. Jika workspace lokal menggunakan nama lain, gunakan root yang sudah ada; jangan membuat root `nodus-studio/` tambahan atau mengganti nama direktori tanpa persetujuan pengguna.
  * **Binary & CLI Command:** `nodus`
  * **App ID (GNOME / Wayland / Flatpak):** `org.nodus.Studio`
  * **Format Paket Komponen:** `.nodpkg` (*Nodus Component Package*)
  * **Format Penyimpanan Proyek:** `.nodus` (*Nodus Project Workspace*)
* **Target Platform Utama (Tahap 1):** Linux Desktop — **Fedora Workstation (Wayland / GNOME)** sebagai *first-class citizen*. Arsitektur harus tetap *cross-platform ready* untuk ekspansi ke distro Linux lain, Windows, dan macOS di masa depan.
* **Tujuan Utama:** Membangun aplikasi desktop simulator IoT *open-source* yang menggabungkan **kualitas visual estetis & intuitif sekelas Tinkercad** dengan **kedalaman emulasi teknis sekelas Wokwi**, ditambah **ekosistem komponen modular terbuka (`.nodpkg`)** yang memudahkan komunitas menambahkan ratusan sensor/modul IoT baru tanpa menyentuh *core engine*.

## 2. Spesifikasi Tech Stack (STRICT & NON-NEGOTIABLE)

| Layer | Teknologi yang Digunakan | Alasan & Aturan Penggunaan | 
| ----- | ----- | ----- | 
| **App Framework** | **Tauri v2** | Menggunakan `webkit2gtk4.1` native di Fedora. Ukuran binari kecil, akses penuh ke sistem operasi & multithreading Rust. | 
| **Backend / Core Engine** | **Rust (Edition 2021/2024)** | Menangani *simulation loop*, penjadwal waktu (*scheduler*), emulasi MCU, bus protokol (I2C, SPI, UART), dan jaringan virtual (MQTT/HTTP). | 
| **Async & Threading** | `tokio` + `crossbeam-channel` | `std::thread` khusus dengan *spin-sleeper* presisi tinggi untuk *Hardware Simulation Thread*, dan `tokio` untuk I/O jaringan & kompilasi. | 
| **Component Scripting** | **Lua 5.4 via `mlua` (Rust)** | Setiap komponen IoT menggunakan skrip Lua terisolasi untuk logika perilakunya. Mudah ditulis kontributor tanpa perlu kompilasi ulang. | 
| **MCU Emulation Core** | **AVR (kandidat: wrapper `simavr`) & ESP32 Bridge** | Target MVP adalah emulasi ATmega328P (Arduino Uno) dengan ketepatan siklus yang diverifikasi. `avr-device` menyediakan definisi akses perangkat, **bukan emulator CPU**, sehingga tidak dapat digunakan sendirian untuk menjalankan firmware. Backend AVR harus lolos spike teknis sebelum integrasi penuh Fase 3; arsitektur tetap modular untuk ESP32/QEMU di masa depan. | 
| **Firmware Compiler** | **`arduino-cli` (Subprocess)** | Dijalankan secara *headless* oleh Rust di latar belakang untuk mengompilasi kode `.ino`/`.cpp` pengguna menjadi `.elf`/`.hex`. | 
| **Frontend Framework** | **SolidJS + TypeScript + Vite** | Reaktivitas *fine-grained* tanpa *Virtual DOM overhead*. Sangat krusial untuk memperbarui ratusan status pin/LED pada $60\text{ FPS}$. | 
| **Styling UI** | **Tailwind CSS v4** | Desain antarmuka *dark-mode first*, bersih, modern, dan menyatu dengan estetika GNOME/Fedora. | 
| **Canvas & Rendering** | **Hybrid SVG + HTML5 Canvas** | Komponen dirender sebagai elemen DOM SVG yang tajam saat di-*zoom*; efek *glow*, partikel, dan analisis sinyal dirender di layer Canvas. | 
| **Code Editor** | **Monaco Editor** | Mendukung *syntax highlighting* C++/Arduino, *auto-complete*, dan *error squiggles* dari output `arduino-cli`. | 

### Dependensi Sistem Dasar (Fedora Linux)

Verifikasi terlebih dahulu versi Fedora, Rust/Cargo, Node.js, package manager frontend, dan dependensi native Tauri v2. Pilih satu package manager frontend, simpan lockfile, dan catat versi toolchain yang digunakan. Gunakan Rust Edition 2021 atau 2024 sesuai versi Rust yang dipilih, lalu tetapkan secara eksplisit di `Cargo.toml`.

Daftar berikut adalah titik awal dependensi Fedora, **bukan jaminan semua nama paket tersedia di setiap versi Fedora**. Cocokkan dengan dokumentasi Tauri v2 dan repositori Fedora yang digunakan. Instalasi paket sistem harus mendapat persetujuan pengguna:

```
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel clang llvm-devel arduino-cli

```

`arduino-cli` baru wajib untuk Fase 3. Jika tidak tersedia dari repositori Fedora yang aktif, gunakan metode instalasi resmi dan catat versinya; jangan mengganti toolchain secara diam-diam. Penambahan dependency native backend AVR dilakukan setelah spike teknis Fase 3.

## 3. Arsitektur Sistem & Alur Data

Sistem **Nodus Studio** dibagi menjadi **3 domain eksekusi**: frontend WebView, dedicated OS thread simulasi Rust, serta worker I/O asinkron. Ini bukan kewajiban membuat tepat tiga proses atau tiga thread; Tauri/WebView dan Tokio dapat memiliki thread/proses internal tambahan. Pekerjaan simulasi dan kompilasi tidak boleh memblokir UI:

```
+-----------------------------------------------------------------------+
|                NODUS STUDIO FRONTEND (SolidJS WebView)                |
|  [Komponen Sidebar]   [Interactive SVG Canvas]   [Monaco Code Editor] |
|          |                       ^                        |           |
+----------|-----------------------|------------------------|-----------+
           | 1. User Event         | 3. Frame Buffer (60Hz) | 4. Compile
           v (Slider/Button)       | (Binary State Array)   v (.ino code)
+-----------------------------------------------------------------------+
|                        TAURI v2 IPC LAYER (Rust)                      |
+-----------------------------------------------------------------------+
           |                       ^                        |
           v                       |                        v
+-------------------------------------+    +----------------------------+
|   NODUS SIMULATION THREAD (Rust)    |    |   TOOLCHAIN & I/O WORKER   |
| - Deterministic Clock Scheduler     |    |         (Tokio Async)      |
| - MCU Emulator Core (AVR / ESP32)   |    | - arduino-cli Runner       |
| - Virtual Bus (GPIO, I2C, SPI, ADC) |    | - Virtual WiFi / MQTT      |
| - Lua Component Sandbox (`mlua`)    |    | - File System (.nodpkg)    |
+-------------------------------------+    +----------------------------+

```

### Aturan Sinkronisasi Waktu & IPC (KRUSIAL):

1. **Hardware Clock & Presisi:** Simulasi berjalan di *dedicated OS thread* Rust. Waktu simulasi harus dibedakan dari waktu nyata (*wall clock*). Untuk MCU $16\text{ MHz}$, $1\text{ siklus} = 62.5\text{ ns}$; **dilarang membulatkan setiap siklus ke nanodetik integer** karena menimbulkan drift. Tetapkan satu representasi waktu internal sebelum Fase 2: fixed-point yang dapat merepresentasikan setengah nanodetik, atau konversi siklus rasional dengan akumulasi sisa. API berakhiran `_ns` menggunakan nanodetik; aturan konversi dan pembulatannya harus terdokumentasi dan tidak boleh mengubah akumulasi waktu internal.

2. **Determinisme & Pacing:** Scheduler berbasis event dengan urutan stabil untuk event bertimestamp sama. Input pengguna dan hasil I/O masuk melalui antrean dan diberi timestamp pada batas pemrosesan yang terdefinisi. Pacing waktu nyata menggunakan *sleep/spin-sleeper* secara terukur, bukan menunggu satu siklus MCU per *sleep*. Jika host tidak mampu mengikuti kecepatan target, laporkan rasio kecepatan simulasi; jangan mengubah waktu virtual atau menghilangkan event hardware agar tampak real-time. Pengujian deterministik menggunakan input bertimestamp yang sama dan tidak bergantung pada broker/jaringan nyata.

3. **State Aggregation:** Rust mencatat perubahan visual (misal: kecerahan LED dari duty cycle PWM, framebuffer OLED, sudut servo) ke `SharedVisualState`. Event hardware tetap diproses oleh mesin; hanya representasi visualnya yang diagregasi. Jangan menjalankan callback Lua setiap siklus MCU bila tidak diperlukan; gunakan perubahan pin dan timer.

4. **60 FPS Frame Sync:** Rust mengirim snapshot biner pada target maksimum sekitar $60\text{ Hz}$, terpisah dari clock hardware. Gunakan transport biner Tauri v2 yang diverifikasi pada WebKitGTK target, bukan array byte yang diserialisasikan menjadi JSON. Pengiriman tidak boleh memblokir thread simulasi. Terapkan antrean visual berbatas dengan kebijakan *latest state wins*: frame visual lama boleh diganti, event hardware tidak boleh dibuang. Frontend menerapkan pembaruan melalui `requestAnimationFrame`.

5. **Kontrak IPC:** JSON boleh digunakan untuk metadata, command, dan pesan error berfrekuensi rendah. Format frame biner wajib memiliki versi protokol, nomor urut frame, waktu simulasi, serta pemetaan instance komponen dan field visual. Definisikan endianness, tipe/panjang field, dan validasi batas buffer sebelum implementasi Fase 2. Frame pertama dan sinkronisasi ulang harus dapat membentuk state visual lengkap.

6. **Batas Tanggung Jawab:** Frontend mengelola viewport, seleksi, drag, routing visual kabel, dan tampilan state dari Rust. Backend menjadi sumber kebenaran topologi rangkaian, resolusi pin, dan perilaku simulasi. Setiap instance di kanvas memiliki `instance_id` unik, berbeda dari `component.id` yang mengidentifikasi jenis paket. Akses WebView ke worker/simulasi melalui command dan antrean yang eksplisit.

### Batas Model Kelistrikan MVP

* MVP menggunakan model net digital diskret: `HIGH`, `LOW`, `HIGH_Z` (tidak didorong), serta `UNKNOWN` untuk keadaan tak terdefinisi/konflik. Bedakan output push-pull, open-drain, input, dan pull-up/pull-down dalam kontrak pin sebelum Fase 2.
* Net tanpa penggerak maupun pull harus dianggap mengambang, bukan otomatis `LOW`. Penggerak kuat `HIGH` dan `LOW` pada net yang sama menghasilkan konflik yang dilaporkan tanpa membuat aplikasi crash.
* Sumber tegangan dan ground direpresentasikan secara eksplisit. Metadata rentang tegangan digunakan untuk validasi/peringatan kompatibilitas; bukan bukti bahwa arus atau tegangan analog telah dihitung.
* LED pada Fase 2 menunjukkan state logika/duty cycle melalui Lua; **belum** menjadi model arus LED yang akurat. Resistor tidak boleh dimodelkan sebagai kabel ideal dan diklaim memiliki perilaku listrik realistis. Model resistansi/arus dan penyelesaian rangkaian analog harus dispesifikasikan serta diuji tersendiri sebelum diaktifkan.
* Abstraksi ADC/PWM disiapkan, tetapi nilai analog dari kontrol sensor bukan penyelesaian rangkaian analog umum. Nodus MVP bukan pengganti SPICE.

## 4. Standar Paket Komponen Modular (`.nodpkg`)

Keunggulan utama **Nodus Studio** adalah kemudahan menambah komponen. Untuk Fase 1–4, format wajib adalah **direktori** `components-registry/<component-id>.nodpkg/` dengan **3 file wajib**. Format arsip adalah perluasan mendatang yang membutuhkan spesifikasi terpisah (format, batas ukuran, dan ekstraksi aman); jangan mengasumsikan `.nodpkg` adalah ZIP.

```
components-registry/
└── sensor-dht22.nodpkg/
    ├── manifest.toml   # Metadata, definisi pin, dan kontrol UI interaktif
    ├── graphic.svg     # Vektor visual berstandar tinggi (Tinkercad-style)
    └── logic.lua       # Logika pemrosesan sinyal & protokol komunikasi

```

### 4.1. Spesifikasi `manifest.toml`

```
# Versi format paket, berbeda dari versi komponen.
schema_version = 1

[compatibility]
# Versi API host Nodus yang dibutuhkan paket.
engine_api = 1

[component]
id = "sensor-dht22"
name = "DHT22 Temperature & Humidity Sensor"
category = "Sensors"
author = "Nodus Community"
version = "1.0.0"
description = "Sensor suhu dan kelembaban digital presisi tinggi."

[electrical]
operating_voltage_min = 3.3
operating_voltage_max = 6.0

# ID pin digunakan oleh Lua; svg_anchor menunjuk ID elemen di graphic.svg.
[[pins]]
id = "VCC"
name = "VCC (+)"
type = "power"
svg_anchor = "pin-vcc"

[[pins]]
id = "SDA"
name = "DATA"
type = "bidirectional"
svg_anchor = "pin-data"

[[pins]]
id = "NC"
name = "NC"
type = "nc"
svg_anchor = "pin-nc"

[[pins]]
id = "GND"
name = "GND (-)"
type = "ground"
svg_anchor = "pin-gnd"

# Kontrol Interaktif yang muncul saat komponen diklik di Canvas Nodus Studio
[[controls]]
id = "temperature"
label = "Temperature (°C)"
type = "slider"
min = -40.0
max = 80.0
step = 0.5
default = 25.0

[[controls]]
id = "humidity"
label = "Humidity (%)"
type = "slider"
min = 0.0
max = 100.0
step = 1.0
default = 60.0

```

### 4.2. Validasi dan Evolusi Kontrak Paket

Kontrak minimal v1 yang diimplementasikan pada Fase 1 dijabarkan di [`docs/nodpkg-v1.md`](docs/nodpkg-v1.md), dengan fixture normalisasi bersama `docs/nodpkg-v1-led.json`. Subset awal hanya mendukung kontrol slider dan field visual angka untuk opacity; boolean/buffer, efek SVG kaya, serta deklarasi backend MCU memerlukan perluasan teruji pada fase terkait. Blueprint ini tetap SSOT untuk arsitektur dan urutan fase.

* Contoh manifest di atas bukan daftar lengkap seluruh jenis komponen. Sebelum loader diimplementasikan, tetapkan skema v1 beserta tipe data, field wajib/opsional, enum pin/kontrol, nilai default, dan aturan penolakan field yang tidak dikenal. Skema Rust dan tipe TypeScript harus konsisten serta diuji dengan fixture yang sama.
* `schema_version` mengatur format paket; `component.version` menggunakan SemVer untuk versi komponen; `compatibility.engine_api` menyatakan versi kontrak host. Tolak versi skema/API yang tidak didukung dengan pesan yang jelas.
* `component.id` harus cocok dengan nama direktori tanpa akhiran `.nodpkg`. ID komponen, pin, kontrol, serta field visual harus unik dalam lingkup masing-masing. `pins[].svg_anchor` harus merujuk tepat satu elemen SVG; **ID pin tidak wajib sama dengan ID SVG**.
* Validasi nilai numerik yang bukan NaN atau tak hingga (*finite*), rentang tegangan, `min <= default <= max`, serta `step > 0`. Berikan error berisi path paket dan field terkait. Paket tidak valid tidak boleh menjatuhkan aplikasi atau membuat paket valid lainnya gagal dimuat.
* Definisikan deklarasi field visual bertipe (boolean, angka dengan rentang, atau buffer berdimensi terbatas), target `dyn-*`, dan operasi rendering yang diizinkan. Lua hanya mengirim nilai melalui field yang dideklarasikan; tidak boleh mengirim JavaScript, HTML, CSS bebas, atau instruksi DOM. Kontrak visual minimal LED ditetapkan pada Fase 1; encoding biner dan API runtime ditetapkan pada Fase 2.
* Semua MCU tetap memiliki paket `.nodpkg`. Sebelum Fase 3, tambahkan deklarasi backend MCU yang terversi untuk memilih implementasi host yang telah terdaftar melalui trait `Microcontroller`. Paket tidak boleh menunjuk executable atau pustaka native arbitrer. Emulasi CPU berada di Rust/backend emulator, bukan di frontend atau skrip Lua per siklus.
* Pada aplikasi terpaket, registry bawaan harus ditemukan melalui mekanisme resource Tauri, bukan bergantung pada current working directory atau path absolut mesin pengembang. Fase 1 harus menetapkan strategi path pengembangan dan resource produksi. Lokasi paket tambahan pengguna dapat ditambahkan kemudian dengan kebijakan validasi yang sama.

### 4.3. Aturan Desain dan Keamanan `graphic.svg`

* Harus menggunakan `viewBox` dengan skala $1\text{ unit SVG} = 0.1\text{ mm}$ ukuran fisik nyata agar proporsi semua komponen di kanvas akurat (misal: jarak antar pin *header* standar $2.54\text{ mm} = 25.4\text{ unit SVG}$).

* Setiap titik koneksi pin wajib memiliki atribut `id="pin-<nama>"` (contoh: `<circle id="pin-vcc" cx="25.4" cy="100" r="5" />`).

* Elemen visual yang bisa berubah dinamis (seperti LED menyala atau jarum servo) wajib memiliki `id="dyn-<nama>"` agar dapat dimanipulasi oleh Frontend berdasarkan state visual tervalidasi dari Rust.

* SVG dari paket dianggap tidak tepercaya. Tetapkan allowlist elemen/atribut dan sanitasi sebelum ditampilkan. Tolak skrip, atribut event `on*`, `foreignObject`, DTD/entity eksternal, referensi file/jaringan eksternal, serta URL/CSS yang dapat memuat konten di luar paket. Efek SVG seperti gradient/filter hanya diizinkan dalam subset lokal yang telah divalidasi. Jangan memasukkan SVG mentah melalui `innerHTML` tanpa sanitasi.

* Batasi ukuran file, jumlah elemen, kompleksitas referensi, dan dimensi buffer. Saat satu paket dipasang berkali-kali, namespace seluruh ID SVG beserta referensi lokalnya per `instance_id` agar pin, gradient, filter, dan elemen dinamis tidak bertabrakan.

### 4.4. Kontrak API `logic.lua`

Skrip Lua dijalankan di dalam sandbox Rust dan hanya boleh berkomunikasi menggunakan API `nodus` yang disediakan host Rust. `nodus` adalah nama kanonis; alias tidak diperlukan untuk API v1.

* Setiap instance komponen memiliki state Lua terisolasi. Jangan mengeksekusi `logic.lua` saat memindai paket di Fase 1; eksekusi dimulai pada Fase 2 ketika instance simulasi diinisialisasi.
* API v1 mencakup `set_pin_mode`, `set_pin`, `read_pin`, `schedule_timer`, `cancel_timer`, dan `set_visual_state`. Sebelum Fase 2, tetapkan signature, tipe nilai, validasi kepemilikan pin/field, serta perilaku error. Timer menggunakan waktu simulasi, bukan waktu OS; `schedule_timer` mengembalikan handle dan hanya menerima callback Lua milik instance yang sama. Pin penggerak dan hasil resolusi net harus dibedakan; `read_pin` membaca hasil resolusi net.
* Callback mencakup `on_init`, `on_control_change`, `on_pin_edge`, dan callback timer terdaftar. Callback edge dapat menerima `HIGH`, `LOW`, `HIGH_Z`, atau `UNKNOWN`; jangan menganggap semua state bersifat biner. Timestamp `_ns` merupakan konversi waktu internal sesuai aturan presisi di bagian 3. Tetapkan urutan inisialisasi, input, propagasi edge, reset, dan pelepasan instance agar hasil dapat direproduksi.
* Nonaktifkan akses `io`, `os`, `package`, `debug`, pemuatan modul/file/native, serta eksekusi kode dinamis yang tidak diperlukan. Ekspos hanya pustaka standar yang diizinkan. Paket tidak mendapat akses filesystem, jaringan, proses, atau FFI melalui Lua.
* Terapkan batas memori, budget instruksi per callback, jumlah timer/event tertunda, dan jumlah event tanpa kemajuan waktu. Infinite loop maupun event berulang pada timestamp sama harus dihentikan secara terkendali. RNG harus dihilangkan atau diberi seed deterministik.
* Error atau pelanggaran budget menonaktifkan instance bermasalah, membatalkan timer miliknya, dan melepas penggerak pin ke `HIGH_Z`; tampilkan diagnostik berisi instance/paket/callback. Jangan meninggalkan output aktif secara diam-diam, dan jangan membuat proses aplikasi crash. Sandbox ini membatasi skrip, bukan pengganti isolasi proses untuk native code tidak tepercaya.

```
-- Illustrative DHT22 callback skeleton; not a complete sensor implementation.
local temp = 25.0
local hum = 60.0

function on_init()
    nodus.set_pin_mode("SDA", "INPUT_PULLUP")
end

-- Called by the host when a validated UI control value changes.
function on_control_change(control_id, value)
    if control_id == "temperature" then temp = value end
    if control_id == "humidity" then hum = value end
end

-- Called by Rust when the resolved pin state changes.
function on_pin_edge(pin_id, state, timestamp_ns)
    if pin_id == "SDA" and state == "LOW" then
        -- Implement the DHT22 single-wire protocol using Rust scheduler timers.
        -- This is not the Dallas/Maxim 1-Wire protocol.
        -- nodus.schedule_timer(delay_ns, "callback_name")
    end
end

```

## 5. Struktur Direktori Proyek (Monorepo)

AI **WAJIB** mematuhi pembagian tanggung jawab pada struktur direktori berikut saat membuat file baru. Nama root mengikuti workspace yang ada. Struktur ini adalah **peta target**, bukan instruksi membuat seluruh modul sekaligus: buat hanya file yang dibutuhkan fase aktif. File konfigurasi, resource, error, dan pengujian tambahan boleh dibuat sesuai kebutuhan toolchain dan tanggung jawabnya.

```
nodus-studio/
├── master_project_blueprint_ai_context_specification.md # SSOT proyek ini
├── package.json                  # Konfigurasi Frontend (name: "nodus-studio")
├── tsconfig.json
├── vite.config.ts
├── src/                          # FRONTEND SOURCE (SolidJS + TypeScript)
│   ├── index.tsx                 # Entry point UI
│   ├── App.tsx                   # Layout utama Nodus Studio (Sidebar, Canvas, Editor)
│   ├── store/                    # Global reactive state (Solid Stores)
│   │   ├── circuitStore.ts       # State posisi komponen & jalur kabel (wires)
│   │   ├── simulationStore.ts    # Penerima buffer 60FPS dari Rust
│   │   └── editorStore.ts        # State kode firmware & log kompilasi
│   ├── components/
│   │   ├── canvas/               # Mesin render kanvas
│   │   │   ├── Workbench.tsx     # Pan, zoom, grid SVG/Canvas
│   │   │   ├── WireLayer.tsx     # Manhattan/Bezier wire routing & animasi arus
│   │   │   └── NodeRenderer.tsx  # Pemuat SVG komponen & pop-up kontrol sensor
│   │   ├── editor/
│   │   │   ├── CodeEditor.tsx    # Wrapper Monaco Editor
│   │   │   └── SerialMonitor.tsx # Terminal UART real-time & Plotter
│   │   └── library/
│   │       └── ComponentCatalog.tsx # Daftar komponen dari components-registry
│   └── types/
│       └── nodpkg.ts             # TypeScript interfaces untuk manifest .nodpkg & IPC
│
├── src-tauri/                    # BACKEND SOURCE (Rust + Tauri v2)
│   ├── Cargo.toml                # Package name: "nodus-studio", bin: "nodus"
│   ├── tauri.conf.json           # Identifier: "org.nodus.Studio", productName: "Nodus Studio"
│   └── src/
│       ├── main.rs               # Entry point aplikasi
│       ├── lib.rs                # Registrasi Tauri commands & state
│       ├── ipc/                  # Komunikasi data antara Rust & SolidJS
│       │   ├── commands.rs       # Start/Stop sim, Load komponen, Compile
│       │   └── frame_streamer.rs # Pengirim binary state 60FPS
│       ├── engine/               # INTI MESIN SIMULASI NODUS
│       │   ├── scheduler.rs      # Event queue deterministik, tanpa drift siklus
│       │   ├── circuit_graph.rs  # Netlist & resolusi state net digital MVP
│       │   ├── pin.rs            # Abstraksi Digital, Analog (ADC), dan PWM
│       │   └── bus/              # Protokol komunikasi perangkat keras
│       │       ├── i2c.rs
│       │       ├── spi.rs
│       │       └── uart.rs
│       ├── mcu/                  # EMULATOR MIKROKONTROLER
│       │   ├── traits.rs         # Trait `Microcontroller` standar
│       │   ├── avr_uno.rs        # Implementasi ATmega328P (Arduino Uno)
│       │   └── esp32_bridge.rs   # Ekstensi masa depan; bukan deliverable MVP
│       ├── sandbox/              # LUA COMPONENT RUNNER
│       │   ├── pkg_loader.rs     # Parser manifest.toml & validasi .nodpkg
│       │   └── lua_host.rs       # Konteks `mlua` dan API `nodus.*`
│       ├── compiler/             # INTEGRASI TOOLCHAIN
│       │   └── arduino_cli.rs    # Eksekutor `arduino-cli` & parser error
│       └── network/              # JARINGAN IoT VIRTUAL
│           └── mqtt_bridge.rs    # Klien MQTT & HTTP virtual untuk komponen WiFi
│
└── components-registry/          # KOLEKSI KOMPONEN (.nodpkg)
    ├── mcu-arduino-uno.nodpkg/
    ├── basic-led.nodpkg/
    ├── basic-pushbutton.nodpkg/
    ├── basic-power-source.nodpkg/
    ├── basic-resistor.nodpkg/    # Perilaku listrik menunggu spesifikasi analog
    ├── sensor-dht22.nodpkg/
    ├── sensor-hcsr04.nodpkg/
    ├── display-ssd1306-oled.nodpkg/
    └── actuator-servo-sg90.nodpkg/

```

## 6. Peta Jalan Eksekusi (Step-by-Step Roadmap)

Pengembangan **Nodus Studio** harus dilakukan secara bertahap. **AI tidak boleh mengerjakan Fase** $N+1$ **sebelum Fase** $N$ **selesai dan teruji.**

Sebelum implementasi setiap fase, catat keputusan kontrak yang masih terbuka, cakupan pekerjaan, dan pengujian penerimaan. Setelah implementasi, laporkan command validasi beserta hasil sebenarnya; jangan menyatakan lolos jika build/test tidak dijalankan. Verifikasi desktop yang belum dapat dilakukan harus ditandai tertunda dan tidak boleh dianggap lulus. Perubahan stack, perluasan cakupan, atau perpindahan fase membutuhkan persetujuan pengguna.

### Aturan Laporan Tahap & Handoff Agent (WAJIB)

* Sebelum bekerja, baca `AGENTS.md`, `PROGRESS.md`, laporan bernomor terbaru yang ditautkan di sana, dan laporan lain yang relevan. Verifikasi keadaan file/tooling saat ini; jangan menganggap laporan lama selalu mencerminkan lingkungan yang aktif.
* Setiap subtahap pengembangan yang bermakna wajib menghasilkan laporan Markdown di `docs/reports/`, termasuk ketika pekerjaan baru selesai sebagian atau terhambat. Gunakan `docs/reports/TEMPLATE.md` dan nomor global berikutnya: `NNN-phase-N-short-topic.md`, dimulai dari `001`. Jangan menimpa laporan terdahulu untuk menggunakan ulang nomornya.
* Sebelum mengakhiri sesi, catat cakupan, perubahan/file aktual, keputusan, command dan hasil validasi, keterbatasan, hambatan, serta tindakan berikutnya. Pisahkan pekerjaan yang benar-benar selesai dari rencana. Untuk pekerjaan lintas sesi, simpan laporan parsial dan lanjutkan riwayat pada laporan berikutnya.
* Perbarui `PROGRESS.md` agar berisi fase/subtahap aktif, status roadmap, tautan laporan terbaru, hambatan, dan langkah konkret berikutnya. Gunakan status `not-started`, `in-progress`, `partial`, `blocked`, atau `completed`; hasil verifikasi memakai `not-run`, `failed`, `passed`, atau `blocked` beserta cakupannya.
* Fase hanya boleh ditandai `completed` setelah seluruh acceptance criteria fase memiliki bukti. Kelulusan pemeriksaan lingkungan atau scaffold bukan kelulusan Fase 1.
* Diskusi rutin read-only atau edit trivial tidak memerlukan laporan terpisah; bila relevan, masukkan ke laporan tahap aktif. Jangan memasukkan rahasia, kredensial, atau log sensitif. Agent yang tidak otomatis membaca `AGENTS.md` harus diarahkan untuk membacanya.

### Fase 1: Fondasi Aplikasi & Kanvas Interaktif (Visual First)

* **Target:** Aplikasi **Nodus Studio** berjalan di Fedora Linux, mampu memuat paket `.nodpkg`, menampilkannya di kanvas dengan proporsi akurat, dan menghubungkan pin dengan kabel.

* **Deliverables:**

  1. **Verifikasi lingkungan & scaffold:** Periksa toolchain/dependensi Fedora, tetapkan versi dan lockfile, lalu inisialisasi Tauri v2 (`org.nodus.Studio`, binary `nodus`) + SolidJS + TypeScript + Vite + Tailwind CSS v4. Pastikan jendela desktop dapat dibuka sebelum menambah fitur.

  2. **Kontrak paket & loader:** Tetapkan skema manifest v1 dan kontrak visual minimal LED. Buat model Rust, tipe `src/types/nodpkg.ts`, serta `pkg_loader.rs` untuk menemukan registry, memvalidasi paket, dan mengirim metadata beserta SVG yang sudah disanitasi ke frontend. Buat `basic-led.nodpkg` lengkap sebagai fixture; file Lua wajib ada tetapi **tidak dieksekusi** pada fase ini.

  3. **Katalog & kanvas:** Buat `ComponentCatalog.tsx`, `NodeRenderer.tsx`, dan `Workbench.tsx` dengan *Infinite Pan & Zoom*, *Snap-to-Grid* ($2.54\text{ mm} = 25.4$ unit SVG), serta *Drag-and-Drop*. Gunakan transformasi koordinat yang konsisten agar anchor pin tetap akurat saat pan/zoom dan perpindahan komponen.

  4. **Kabel & netlist:** Buat `WireLayer.tsx`: klik pin A ke pin B, routing *orthogonal/Manhattan*, dan perubahan warna kabel. Backend memvalidasi endpoint dan menyimpan graf koneksi; saat komponen/kabel dihapus, netlist ikut diperbarui. Fase ini membentuk topologi saja, belum menjalankan scheduler, Lua, atau resolusi listrik.

* **Kriteria lulus Fase 1:**
  * Typecheck/build frontend, pengujian loader/netlist Rust, dan build aplikasi desktop berhasil dengan command yang dicatat.
  * Paket valid tampil di katalog; paket rusak/versi tidak didukung/anchor hilang/SVG berbahaya ditolak dengan diagnostik tanpa menjatuhkan aplikasi. Loader membatasi ukuran file dan menolak path traversal/symlink yang keluar dari direktori paket.
  * Beberapa instance LED yang sama dapat ditampilkan tanpa benturan ID SVG. Pan/zoom, snap, drag, serta sambung/hapus kabel mempertahankan endpoint dan netlist yang benar.
  * Smoke test di Fedora Wayland/GNOME dilakukan dan hasilnya dicatat. Tidak ada logika simulasi hardware di frontend.

### Fase 2: Mesin Simulasi Inti & Sandbox Komponen Lua

* **Target:** Simulasi waktu nyata berjalan di Rust dan komponen pasif/sederhana dapat berinteraksi melalui skrip Lua.

* **Deliverables:**

  1. Tetapkan representasi waktu presisi dan aturan resolusi net digital, lalu implementasikan `scheduler.rs`, `pin.rs`, dan resolusi state pada `circuit_graph.rs` yang sudah dibangun di Fase 1. Tambahkan start/pause/resume/stop/reset yang jelas; reset harus menginisialisasi ulang instance dan membatalkan event lama.

  2. Finalisasi kontrak API Lua v1 dan integrasi `mlua` di `lua_host.rs`, termasuk semua pembatasan sandbox dan pelaporan error di bagian 4.4.

  3. Tetapkan protokol frame biner dan implementasikan `frame_streamer.rs` dengan target $60\text{ Hz}$, antrean berbatas, serta pengujian transport pada Tauri/WebKitGTK. Frontend hanya menerapkan state visual dari Rust.

  4. Buat `basic-power-source.nodpkg` (terminal $5\text{V}$ dan GND) serta `basic-pushbutton.nodpkg`; aktifkan perilaku Lua pada `basic-led.nodpkg` yang sudah tersedia sejak Fase 1. LED menyala/glow berdasarkan state net dan duty cycle, bukan klaim perhitungan arus analog.

* **Kriteria lulus Fase 2:**
  * Pengujian waktu membuktikan $16{,}000{,}000$ siklus pada $16\text{ MHz}$ setara tepat satu detik virtual tanpa drift; event bertimestamp sama memiliki urutan stabil.
  * Uji net mencakup floating, pull, open-drain, dan konflik output. Rangkaian sumber–tombol–LED merespons input melalui Rust dan Lua; pin konflik dilaporkan.
  * Infinite loop Lua, kehabisan budget/memori, banjir timer, dan error callback ditangani tanpa crash atau hang aplikasi.
  * Decoder menolak frame terpotong/versi tidak dikenal; antrean visual tetap berbatas saat frontend lambat. UI tetap responsif selama smoke test dengan jumlah komponen dan perangkat host yang dicatat.
  * Start/pause/resume/stop/reset tidak meninggalkan thread, timer, atau state instance lama.

### Fase 3: Integrasi Editor Kode, `arduino-cli`, & Emulasi MCU Pertama

* **Target:** Pengguna dapat menulis kode `setup()` dan `loop()` Arduino di dalam **Nodus Studio**, mengompilasinya di Fedora, dan melihat hasilnya menggerakkan komponen di kanvas.

* **Deliverables:**

  1. **Spike backend AVR lebih dahulu:** Evaluasi wrapper `simavr` untuk load `.elf`/`.hex`, stepping siklus, GPIO, timer/PWM, UART, reset, dan integrasi waktu dengan scheduler. Verifikasi lisensi/redistribusi, dependensi native Fedora, keamanan FFI, serta kemampuan build. Catat keputusan dan keterbatasan; jika kandidat gagal, minta persetujuan sebelum mengganti backend.

  2. Integrasi Monaco Editor dan panel Serial Monitor. Buat `arduino_cli.rs` dengan argumen subprocess terstruktur (tanpa interpolasi shell), workspace sementara terisolasi, timeout/pembatalan, dan pemetaan error kompilasi ke editor. Catat versi `arduino-cli` dan Arduino AVR core; instalasi board core/library membutuhkan jaringan dan tidak boleh dilakukan diam-diam.

  3. Implementasi trait `Microcontroller` dan `avr_uno.rs` berdasarkan spike yang lolos, lalu hubungkan ke scheduler dan GPIO/UART/PWM. Jangan menjalankan emulator per siklus melalui IPC atau callback Lua.

  4. Tambahkan deklarasi backend MCU ke kontrak paket tanpa merusak paket v1 yang sudah valid, lalu buat `mcu-arduino-uno.nodpkg`. Lua menangani perilaku pendukung paket bila diperlukan, bukan emulasi CPU.

* **Kriteria lulus Fase 3:**
  * Sketch Blink dapat dikompilasi, dimuat, dan mengubah LED pada periode virtual yang diharapkan; uji PWM, input tombol, serta UART/Serial Monitor tersedia.
  * Error kompilasi tampil di editor; toolchain hilang, timeout, dan pembatalan menghasilkan error yang dapat ditindaklanjuti.
  * Ketepatan siklus/periferal yang didukung diuji terhadap fixture yang terdokumentasi. Jangan mengklaim seluruh ATmega328P siklus-akurat bila cakupan uji belum membuktikannya.
  * **ESP32/QEMU bukan deliverable fase ini**; integrasinya memerlukan roadmap dan pengujian tersendiri.

### Fase 4: Ekosistem Sensor IoT, Bus Protokol (I2C/SPI), & Jaringan Virtual

* **Target:** Menjalankan kumpulan kasus penggunaan IoT yang terdefinisi dengan sensor, layar, aktuator, dan MQTT. Paritas umum dengan Wokwi/Tinkercad adalah aspirasi jangka panjang, **bukan kriteria kelulusan fase ini**.

* **Deliverables:**

  1. Implementasi bus protokol `i2c.rs` dan `spi.rs` di mesin Rust.

  2. Pembuatan komponen IoT esensial:

     * `sensor-dht22.nodpkg` (dengan *slider* suhu & kelembaban interaktif).

     * `sensor-hcsr04.nodpkg` (sensor jarak ultrasonik dengan *slider* jarak objek).

     * `display-ssd1306-oled.nodpkg` (layar OLED $128 \times 64$ piksel via I2C yang merender *bitmap buffer* ke kanvas).

     * `actuator-servo-sg90.nodpkg` (motor servo yang berputar mulus berdasarkan sinyal pulsa PWM $50\text{ Hz}$).

  3. Implementasi `mqtt_bridge.rs` agar simulasi dapat mempublikasikan dan menerima pesan MQTT ke broker nyata melalui worker Tokio. Tanpa emulator WiFi/ESP32, gunakan bridge host yang dikonfigurasi eksplisit; jangan mengklaim firmware Arduino Uno memiliki WiFi atau stack TCP/IP native. Pemanggilan bridge oleh komponen hanya melalui API host terbatas, bukan akses jaringan langsung dari Lua. Jika perlu perluasan API, versikan kontraknya.

* **Kriteria lulus Fase 4:**
  * Tersedia rangkaian/sketch fixture untuk DHT22, HC-SR04, OLED I2C, servo, serta fixture bus SPI. Verifikasi timing/data protokol, framebuffer OLED, dan pemetaan pulsa ke sudut servo.
  * MQTT diuji dengan broker lokal terkendali: publish/subscribe, putus-sambung, batas antrean, dan pelaporan error. Akses jaringan harus opt-in; jelaskan endpoint serta data yang keluar dari aplikasi. Kredensial tidak boleh ditanam dalam kode, paket komponen, atau log.
  * I/O jaringan tidak memblokir simulasi. Rekam input jaringan bertimestamp jika replay deterministik dibutuhkan; broker nyata tidak dijadikan prasyarat unit test.
  * Keterbatasan model sensor, bus, dan jalur jaringan dicatat; ESP32 tetap di luar cakupan kecuali roadmap disetujui ulang.

### Fase 5: Pemolesan UX, Penyimpanan Proyek, & Pemaketan Linux (Flatpak/RPM)

* **Target:** Siap dirilis ke publik sebagai proyek *open-source* kebanggaan komunitas.

* **Deliverables:**

  1. Tetapkan format `.nodus` terversi sebelum implementasi Simpan & Muat: rangkaian, instance/koneksi, kontrol, firmware, serta referensi ID dan versi paket. Jelaskan apakah paket di-embed atau direferensikan, aturan migrasi, dan perilaku jika paket hilang/berbeda versi. Gunakan penulisan atomik, validasi ukuran/path, dan jangan menyimpan kredensial broker di proyek.

  2. *Built-in Component Inspector / Logic Analyzer* sederhana untuk melihat gelombang sinyal digital/PWM.

  3. Skrip build paket `.rpm`, konfigurasi sumber/build untuk Fedora COPR, dan manifest Flatpak (`org.nodus.Studio.yml`). Verifikasi distribusi registry bawaan, lisensi/dependensi emulator, serta strategi toolchain `arduino-cli` di sandbox Flatpak. Akses subprocess host, filesystem, dan jaringan harus dirancang eksplisit dengan izin minimum; jangan menganggap semua executable host tersedia dari Flatpak.

* **Kriteria lulus Fase 5:**
  * Uji round-trip `.nodus` mempertahankan rangkaian dan firmware; file rusak, versi tak didukung, serta paket hilang ditangani tanpa kehilangan proyek yang sedang terbuka.
  * Logic analyzer membaca data sinyal dari Rust dengan buffer berbatas, bukan menyimpulkan gelombang dari frame visual 60 Hz yang dapat kehilangan pulsa singkat.
  * RPM dan Flatpak dipasang serta diuji pada lingkungan Fedora target yang bersih; registry, kompilasi, simulasi, dan izin berjalan sesuai dokumentasi.
  * Dokumentasi build/penggunaan, lisensi, batas fitur, contoh proyek, dan hasil validasi rilis tersedia.

## 7. Standar Penulisan Kode (Coding Conventions untuk AI)

1. **Bahasa Kode & Komentar:** Semua nama variabel, fungsi, tipe data, dan komentar teknis di dalam kode **WAJIB** menggunakan Bahasa Inggris standar industri. Diskusi dengan pengguna tetap menggunakan Bahasa Indonesia.

2. **Rust Safety & Error Handling:**

   * **DILARANG** menggunakan `.unwrap()` atau `.expect()` pada kode produksi di `src-tauri/`. Gunakan `Result<T, NodusError>` dengan *crate* `thiserror` agar aplikasi tidak pernah *crash* tiba-tiba saat pengguna salah merangkai kabel atau menulis skrip Lua yang *error*.

3. **SolidJS Reactivity:**

   * Jangan pernah melakukan *destructuring* pada `props` di komponen SolidJS (misal: `const { pins } = props` adalah **SALAH** karena menghilangkan reaktivitas; gunakan `props.pins`).

4. **Modularitas File:**

   * Jangan membuat satu file kode lebih dari $400$ baris. Pecah menjadi sub-modul yang rapi sesuai tanggung jawabnya (*Single Responsibility Principle*).

## 8. Panduan Prompt Awal untuk Memulai Bersama AI

Saat memberikan file `master_project_blueprint_ai_context_specification.md` ini ke AI Assistant, gunakan prompt berikut untuk memulai eksekusi tanpa melenceng. Blueprint ini tetap menjadi satu SSOT; jangan membuat salinan `PROJECT_BLUEPRINT.md` yang berpotensi berbeda isinya:

> *"Baca `master_project_blueprint_ai_context_specification.md` sebagai SSOT Nodus Studio. Kerjakan hanya Fase 1, Langkah 1 & 2: periksa lingkungan dan file yang sudah ada, laporkan dependency yang kurang, tetapkan kontrak manifest v1 serta visual LED, kemudian buat fondasi Tauri v2 + SolidJS + TypeScript + Vite + Tailwind CSS v4, model Rust dan tipe TypeScript, loader/sanitasi paket beserta pengujiannya, serta contoh lengkap `basic-led.nodpkg`. Gunakan root workspace yang sudah ada. Jangan mengeksekusi Lua, menjalankan simulasi, atau mengerjakan emulator. Validasi build/typecheck/test yang tersedia dan laporkan hasil serta pemeriksaan desktop yang masih tertunda. Minta persetujuan sebelum instalasi paket sistem atau perubahan cakupan."*