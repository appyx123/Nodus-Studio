# Nodus Studio: Linux Packaging & Distribution Guide

This document outlines the packaging, distribution, and sandbox security architecture for Nodus Studio on Linux (specifically Fedora RPM, COPR, and Flatpak).

## 1. Application Identifiers & Standards

- **App ID (reverse-DNS):** `org.nodus.Studio`
- **Binary executable:** `nodus`
- **Default window class:** `nodus`
- **MIME type:** `application/x-nodus-project+json` (`*.nodus`)
- **Desktop entry:** `packaging/org.nodus.Studio.desktop`
- **MIME database:** `packaging/org.nodus.Studio.xml`
- **AppStream Metainfo:** `packaging/org.nodus.Studio.metainfo.xml`

---

## 2. Flatpak Packaging (`org.nodus.Studio.yml`)

The Flatpak package runs in a confined container sandbox adhering to the principle of least privilege.

### Sandbox Permissions (`finish-args`)

| Argument | Reason |
|---|---|
| `--socket=wayland` | Native Wayland desktop rendering under GNOME / KDE |
| `--socket=fallback-x11` | X11 fallback for legacy display servers |
| `--share=ipc` | Shared memory IPC for WebKitGTK and GPU buffers |
| `--device=dri` | Hardware-accelerated OpenGL/Vulkan rendering |
| `--share=network` | Opt-in network access strictly for the Virtual MQTT Bridge client |
| `--filesystem=xdg-documents:rw` | Access to user Documents for `.nodus` project persistence |
| `--filesystem=xdg-download:rw` | Access to Downloads for exporting project files |
| `--talk-name=org.freedesktop.portal.Flatpak` | Allows spawning external commands via Flatpak Portal |

### Built-in Package Registry Distribution

Built-in component packages (`.nodpkg`) are installed into `/app/share/nodus-studio/components-registry/`. At runtime, `list_components` in Rust checks the bundle resource path (`resource_dir`) before falling back to local search paths.

### Arduino CLI Strategy in Flatpak Sandbox

Microcontroller compilation in Nodus Studio relies on `arduino-cli`. Because host binaries cannot be arbitrarily executed inside a Flatpak container:
1. **Bundled Sandbox CLI (Primary):** The Flatpak manifest bundles a statically-linked `arduino-cli` binary in `/app/bin/arduino-cli` along with the `arduino:avr` core package.
2. **Host Spawn Portal (Fallback):** If the bundled compiler is unavailable or custom user toolchains are needed, Nodus Studio requests execution via `org.freedesktop.portal.Flatpak` Subprocess Spawn (`flatpak-spawn --host arduino-cli`).

---

## 3. Fedora RPM Packaging (`nodus-studio.spec`)

The RPM package provides a native package for Fedora Workstation 40/41/42/43/45 and enterprise Linux.

### Spec File Configuration

- Spec path: `packaging/rpm/nodus-studio.spec`
- System installation paths:
  - Binary: `/usr/bin/nodus`
  - Component registry: `/usr/share/nodus-studio/components-registry/`
  - Desktop file: `/usr/share/applications/org.nodus.Studio.desktop`
  - AppStream: `/usr/share/metainfo/org.nodus.Studio.metainfo.xml`
  - MIME database: `/usr/share/mime/packages/org.nodus.Studio.xml`
  - Icon: `/usr/share/icons/hicolor/128x128/apps/org.nodus.Studio.png`

### Building the RPM Locally

```bash
# Install build prerequisites
sudo dnf install -y cargo rust nodejs npm webkit2gtk4.1-devel gtk3-devel openssl-devel desktop-file-utils libappstream-glib rpm-build

# Run automated packaging script
./scripts/build-rpm.sh
```

### Fedora COPR Continuous Build

Fedora COPR builds are configured using `.copr/Makefile`:
```makefile
outdir ?= .

srpm:
	mkdir -p $(outdir)
	tar --exclude-vcs --exclude='./target' --exclude='./src-tauri/target' --exclude='./node_modules' \
		--transform 's,^\.,nodus-studio-0.1.0,' -czf $(outdir)/nodus-studio-0.1.0.tar.gz .
	rpmbuild -bs \
		--define "_sourcedir $(outdir)" \
		--define "_srcrpmdir $(outdir)" \
		packaging/rpm/nodus-studio.spec
```

---

## 4. Emulator & Dependencies Licensing

All software components bundled with or used by Nodus Studio conform to permissive open-source licenses:
- **Nodus Studio:** MIT License
- **avr-emulator (Rust crate):** MIT License
- **mlua (Lua 5.4 runtime):** MIT License
- **Tauri v2:** MIT / Apache-2.0
- **SolidJS:** MIT License
- **Monaco Editor:** MIT License
- **quick-xml / serde / crossbeam:** MIT / Apache-2.0

---

## 5. Sample Project & Feature Limits

An example project demonstrating a complete end-to-end circuit is located at `examples/blink-led.nodus`:
- **Components:** Arduino Uno (`mcu-arduino-uno`) connected to Basic LED (`basic-led`).
- **Connections:** Pin 13 to LED Anode (`A`), GND to LED Cathode (`K`).
- **Firmware:** Blink sketch with serial output at 9600 baud.
- **Verification:** Open Nodus Studio, click **Open .nodus**, choose `examples/blink-led.nodus`, and press **Start**. The LED toggles synchronously with the internal timer, and the Digital Logic Analyzer records the 1 Hz / 500 ms square wave.
