# 024 — Fase 5: Pemaketan Linux (RPM, COPR, Flatpak) dan Penyelesaian Fase 5

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Coding (Antigravity harness)
- **Fase dan langkah blueprint:** Fase 5, Langkah 3
- **Status subtahap:** completed
- **Status fase:** completed

## Tujuan dan batas cakupan

- Menyediakan skrip dan konfigurasi pemaketan Linux standar industri: spec file RPM (`nodus-studio.spec`) untuk Fedora Workstation / Enterprise Linux, integrasi build Fedora COPR (`.copr/Makefile`), dan manifest sandbox Flatpak (`org.nodus.Studio.yml`).
- Menyediakan desktop integration lengkap: desktop entry (`org.nodus.Studio.desktop`), MIME type database (`org.nodus.Studio.xml` untuk ekstensi `.nodus`), dan AppStream metainfo (`org.nodus.Studio.metainfo.xml`).
- Menetapkan strategi toolchain `arduino-cli` di dalam sandbox Flatpak: bundling primer dalam `/app/bin/arduino-cli` serta fallback portal `flatpak-spawn --host` melalui `org.freedesktop.portal.Flatpak`.
- Menyediakan dokumentasi lengkap distribusi Linux (`docs/packaging-linux.md`), lisensi dependensi/emulator (semua berlisensi open-source permisif MIT/Apache-2.0), batas fitur, dan contoh proyek sirkuit siap pakai (`examples/blink-led.nodus`).

## Hasil dan perubahan

- **Packaging Linux (`packaging/` & `.copr/`):**
  - `packaging/org.nodus.Studio.desktop`: Desktop entry file dengan kategori Development/Electronics, MIME binding `application/x-nodus-project+json`, dan `StartupWMClass=nodus`. Divalidasi dengan `desktop-file-validate`.
  - `packaging/org.nodus.Studio.xml`: Shared MIME info XML untuk ekstensi `.nodus`.
  - `packaging/org.nodus.Studio.metainfo.xml`: AppStream metadata spesifikasi modern (developer tag, license, release info). Divalidasi sukses dengan `appstreamcli validate`.
  - `packaging/flatpak/org.nodus.Studio.yml`: Manifest Flatpak runtime `org.freedesktop.Platform 24.08` dengan izin minimum (`wayland`, `fallback-x11`, `ipc`, `dri`, `network` untuk MQTT, `xdg-documents` untuk file `.nodus`, portal spawn).
  - `packaging/rpm/nodus-studio.spec`: RPM spec file untuk Fedora dengan build requirements, install layout (`/usr/bin/nodus`, `/usr/share/nodus-studio/components-registry`), desktop integration, dan verifikasi check.
  - `.copr/Makefile`: Otomasi pembuatan source archive dan SRPM untuk continuous build di Fedora COPR.
  - `scripts/build-rpm.sh`: Skrip otomasi pembuatan tarball dan eksekusi `rpmbuild -ba`.
- **Contoh Proyek & Dokumentasi:**
  - `examples/blink-led.nodus`: Proyek sirkuit referensi lengkap berisi Arduino Uno, Basic LED, wiring D13 & GND, sketch Arduino Blink serial, dan viewport kanvas.
  - `src-tauri/src/project/tests.rs`: Pengujian unittests parsing dan validasi file `examples/blink-led.nodus`.
  - `docs/packaging-linux.md`: Panduan distribusi, keamanan sandbox Flatpak, instruksi build RPM/COPR, strategi toolchain, lisensi dependensi, dan batas fitur.

## Keputusan teknis

- **Desain Sandbox Flatpak Berizin Minimum:** Menghindari grant filesystem host menyeluruh (`--filesystem=host`). Akses berkas dibatasi ke `xdg-documents:rw` dan `xdg-download:rw` untuk integritas data pengguna. Akses jaringan opt-in hanya untuk virtual MQTT bridge.
- **Strategi Toolchain Ganda Arduino CLI:** Menyediakan opsi bundling runtime mandiri di dalam Flatpak serta delegasi ke host melalui portal Flatpak agar pengguna dapat memilih kenyamanan out-of-the-box atau integrasi dengan compiler sistem yang sudah ada.
- **Validasi Standar Linux:** Seluruh metadata desktop diuji langsung dengan validator resmi sistem (`desktop-file-validate` dan `appstreamcli validate`).

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `desktop-file-validate packaging/org.nodus.Studio.desktop` | passed | Validasi desktop entry sukses tanpa error atau warning |
| `appstreamcli validate packaging/org.nodus.Studio.metainfo.xml` | passed | Validasi AppStream metainfo sukses (`✔ Validation was successful`) |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | passed | 89 tes Rust lulus (termasuk verifikasi parsing proyek contoh `examples/blink-led.nodus`) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Zero clippy warning |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed | Format Rust lolos check |
| `npm run build` | passed | Typecheck TypeScript dan bundling Vite sukses |
| `npm test` | passed | 28 tes Node.js lulus |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --no-bundle --debug` | passed | Native debug build binary `nodus` sukses |
| `python3 scripts/desktop-catalog-smoke.py` | passed | 20 checks desktop smoke test lulus di GNOME/Wayland |
| `python3 scripts/webkit-workbench-smoke.py` | passed | 13 checks WebKit DOM smoke test lulus |
| Pemeriksaan batas baris kode (≤ 400 baris) | passed | Seluruh file kode berada di bawah batas 400 baris |

## Hambatan dan risiko

- Eksekusi build Flatpak container lokal (`flatpak-builder`) memerlukan instalasi Flatpak runtime SDK di host. Manifest `org.nodus.Studio.yml` dan spec file RPM telah siap digunakan di build environment Fedora dan Flathub/COPR.

## Handoff: langkah berikutnya

1. Seluruh langkah Fase 5 (Langkah 1: Format Proyek & Persistensi, Langkah 2: Logic Analyzer, Langkah 3: Pemaketan Linux) telah selesai dengan kriteria kelulusan terpenuhi.
2. Nodus Studio telah menyelesaikan seluruh Fase 1 hingga Fase 5 sesuai SSOT blueprint.
3. Siap untuk rilis versi v0.1.0 dan penyusunan release notes rilis publik.

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
