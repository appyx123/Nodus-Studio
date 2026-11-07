# 025 — Rilis v0.1.0, CI/CD GitHub Actions, dan Distribusi Linux

- **Tanggal:** 2026-11-07
- **Agent / pelaksana:** Coding agent
- **Fase dan langkah blueprint:** Pasca-Fase 5 / Rilis Publik v0.1.0
- **Status subtahap:** completed
- **Status fase:** completed

## Tujuan dan batas cakupan

- Mempublikasikan rilis GitHub resmi `v0.1.0` untuk tag yang telah dibuat.
- Mengonfigurasi otomatisasi CI/CD GitHub Actions untuk pengujian otomatis (lint, test, build) serta otomatisasi rilis multi-distribusi.
- Membuat paket distribusi portabel Linux x86_64 (`.tar.gz`) beserta skrip instalasi zero-dependency (`install.sh`/`uninstall.sh`), serta mengunggahnya ke GitHub Release.

## Hasil dan perubahan

- `.github/workflows/ci.yml`: Workflow CI GitHub Actions untuk pengujian otomatis frontend (Node 22, unit tests, Vite build) dan backend Rust (formatting, 89 unit tests, Clippy `-D warnings`).
- `.github/workflows/release.yml`: Workflow otomatisasi rilis pada push tag `v*` atau manual dispatch untuk membuild tarball portabel Linux dan paket RPM Fedora via kontainer Fedora.
- `scripts/package-release.sh`: Skrip pemaketan portabel Linux x86_64 mandiri yang memaketkan binary `nodus`, registry komponen bawaan, desktop entry, mime type, AppStream metadata, ikon hicolor, skrip instalasi user `~/.local` / `/usr/local`, serta verifikasi checksum SHA256.
- GitHub Release `v0.1.0`: Terpublikasi di `https://github.com/appyx123/Nodus-Studio/releases/tag/v0.1.0` dengan aset `nodus-studio-0.1.0-linux-x86_64.tar.gz` (4.5 MB) dan file `.sha256`.
- `.gitignore`: Menambahkan folder build `dist-release/` dan `build-rpm/`.

## Keputusan teknis

- Menggunakan workflow terpisah `ci.yml` dan `release.yml` dengan permissions minimal (`contents: write` hanya pada job rilis).
- Pemaketan portabel Linux menyediakan skrip instalasi `install.sh` yang mendukung instalasi tanpa hak root ke `$HOME/.local` maupun dengan sudo ke `/usr/local`.

## Validasi

| Command / pemeriksaan aktual | Hasil | Bukti ringkas / batas cakupan |
| --- | --- | --- |
| `gh release create v0.1.0 ...` | passed | Release v0.1.0 aktif di GitHub |
| `npm run tauri -- build --no-bundle` | passed | Binary release terkompilasi di `src-tauri/target/release/nodus` (15 MB) |
| `./scripts/package-release.sh 0.1.0` | passed | Tarball `dist-release/nodus-studio-0.1.0-linux-x86_64.tar.gz` (4.5 MB) & SHA256 dibuat |
| `gh release upload v0.1.0 ...` | passed | Tarball dan sha256 terunggah dan diverifikasi via `gh release view v0.1.0` |
| `npm test` | passed | 28 tes unit Node passed |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | passed | 89 tes unit Rust passed |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | passed | Nol warning Clippy |

## Hambatan dan risiko

- Lingkungan lokal host Fedora belum memiliki paket sistem `rpm-build` / `flatpak-builder`, namun otomatisasi pembangunan RPM Fedora telah dikonfigurasikan di workflow `.github/workflows/release.yml` menggunakan kontainer `fedora:41`.

## Handoff: langkah berikutnya

1. Lakukan git push untuk commit workflow CI/CD dan skrip rilis ke branch `main`.
2. Lanjutkan ke roadmap pasca-MVP (komponen periferal baru seperti LCD1602 I2C, potensiometer, atau fitur UX kanvas seperti Undo/Redo dan seleksi kotak).

## Kelengkapan laporan

- [x] Status didukung bukti; pekerjaan direncanakan tidak ditulis sebagai selesai.
- [x] Path, command, hasil, dan batas validasi diperiksa.
- [x] `PROGRESS.md` diperbarui dan menautkan laporan ini.
