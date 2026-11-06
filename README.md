<div align="center">

# Nodus Studio

### An open workbench for connected ideas.

A Linux-first desktop IoT simulator in the making.<br>
Modular components. A Rust-powered foundation. An open ecosystem.

[![Status: Early development](https://img.shields.io/badge/status-early_development-f59e0b?style=flat-square)](#project-status)
[![License: MIT](https://img.shields.io/badge/license-MIT-22c55e?style=flat-square)](LICENSE)
[![Target: Fedora Linux](https://img.shields.io/badge/target-Fedora_Linux-51A2DA?style=flat-square&logo=fedora&logoColor=white)](#getting-started)

[Explore the vision](#why-nodus) · [Get started](#getting-started) · [Roadmap](#roadmap) · [Contribute](#contributing)

</div>

---

> **Early development — not yet a working circuit simulator.** Phase 1 (desktop foundation, validated component loader, catalog, interactive placement canvas, Manhattan wiring, and Rust-owned session netlist) is implemented. Electrical simulation, Lua component execution, and firmware emulation are still on the roadmap. There is no verified release package yet.

## Why Nodus?

**Nodus** is Latin for *knot* or *node*: a connection between circuits, devices, and people.

Nodus Studio aims to bring intuitive circuit building and deeper IoT emulation into an open-source desktop application. The long-term inspiration is the visual approachability of Tinkercad and the technical depth of Wokwi—not a claim of feature parity today.

- **Desktop-native foundation.** Tauri v2 uses the system WebView, with Fedora GNOME/Wayland as the first-class target.
- **An open component format.** Component metadata, graphics, and future behavior live in `.nodpkg` packages, not hard-coded frontend components.
- **Rust owns the hardware.** The planned simulation engine, timing, pin resolution, and MCU emulation belong in the backend. The frontend handles presentation and interaction.
- **Build in the open.** Architecture, acceptance gates, verification evidence, and stage reports live alongside the code.

## Tech stack

[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8D8?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![SolidJS](https://img.shields.io/badge/SolidJS-2C4F7C?style=for-the-badge&logo=solid&logoColor=white)](https://www.solidjs.com/)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-646CFF?style=for-the-badge&logo=vite&logoColor=white)](https://vite.dev/)
[![Tailwind CSS](https://img.shields.io/badge/Tailwind_CSS-v4-06B6D4?style=for-the-badge&logo=tailwindcss&logoColor=white)](https://tailwindcss.com/)

| Layer | Technology | Responsibility |
| --- | --- | --- |
| Desktop shell | Tauri v2 + WebKitGTK | Native window, local IPC, resource discovery |
| Backend | Rust | Package validation and authoritative session netlist; simulation in Phase 2+ |
| Interface | SolidJS + TypeScript | Reactive catalog, interactive canvas, and Manhattan wire layer |
| Tooling and styles | Vite + Tailwind CSS v4 | Frontend builds and dark-first UI |
| Component graphics | Validated SVG | Isolated catalog previews and namespaced inline canvas instances |

**Planned, not integrated:** Lua 5.4 via `mlua`, Monaco Editor, `arduino-cli`, an AVR emulation backend subject to a technical spike, and later virtual buses/MQTT. See the [blueprint](master_project_blueprint_ai_context_specification.md) for the approved architecture.

## Project status

**Phase 1 · Foundation and interactive canvas · `completed`**

### Available today

- A Tauri desktop application with a SolidJS interface.
- A Rust loader for directory-based `.nodpkg` packages.
- Strict manifest v1 validation: versions, IDs, pins, anchors, ranges, and visual declarations.
- A restricted SVG allowlist with normalized output and bounded file/catalog sizes.
- A complete **Red LED** fixture with an isolated catalog preview and multiple namespaced canvas instances.
- Visual placement by Add or drag-and-drop, unbounded panning, cursor-anchored zoom (20–400%), and 2.54 mm snap-to-grid.
- Pin-to-pin Manhattan wiring (`WireLayer.tsx`), five wire colors (`green`, `blue`, `amber`, `red`, `violet`), and an authoritative Rust-owned topology/netlist (`circuit_graph.rs`) supporting up to 256 instances and 1,024 wires.
- Selection, keyboard nudging, and deletion for instances and wires with automatic net updates and cascading wire removal.
- Package-level diagnostics, catalog Reload, and recovery from a missing registry while preserving existing session topology.
- Narrow local IPC commands (`list_components`, `get_circuit`, `edit_circuit`); no frontend-supplied filesystem paths.

### Not available yet

Project persistence, Lua execution, live electrical simulation, scheduler, firmware editing, or MCU emulation.

**Recorded verification:** 56 Rust tests, 24 Node tests, frontend typecheck/build, Rust formatting, Clippy, and native debug build passed. Real WebKitGTK DOM checks cover hit-testing, multi-instance ID isolation, pin geometry, pan/zoom/drag/drop, cancellation, keyboard focus, Manhattan wiring, net counts, wire recoloring, endpoint movement, cascading wire deletion, and browser fallback. Native Fedora GNOME/Wayland accessibility checks cover resource discovery, Add, pin wiring, wire color changes, wire/instance deletion, zoom/reset, and catalog error recovery. See [wiring/netlist report 009](docs/reports/009-phase-1-wiring-and-netlist.md), [canvas report 008](docs/reports/008-phase-1-interactive-workbench.md), and [loader report 005](docs/reports/005-phase-1-component-loader.md).

The WebKit interaction harness mocks only discovery and dispatches synthetic input. Physical drag/drop and trusted pointer capture still need a manual desktop check.

These are recorded local results, **not a CI badge**. Pixel-level visual QA, release packaging, and minimum-compiler compatibility remain unverified. [PROGRESS.md](PROGRESS.md) is the current handoff summary.

## Getting started

### Prerequisites

| Requirement | Notes |
| --- | --- |
| Node.js | `^22.12.0` or `>=24.0.0`; recorded environment: 22.23.1 |
| npm | Project package manager: 10.9.8; use `package-lock.json` |
| Rust + Cargo | Manifest minimum: Rust 1.90, Edition 2021; verified compiler: Fedora 1.99.1 |
| Native libraries | Tauri v2 prerequisites for your operating system |

The minimum Rust compiler has not been tested independently. Keep both lockfiles; use `--locked` for Cargo checks. A Lua runtime, `arduino-cli`, and emulator dependencies are **not required at this stage**.

<details>
<summary><strong>Fedora native dependencies</strong></summary>

The following names were checked against Fedora 45 repository metadata. Install them from your own terminal with administrator access:

```sh
sudo dnf install rust cargo rustfmt clippy gcc-c++ make pkgconf-pkg-config webkit2gtk4.1-devel openssl-devel librsvg2-devel libappindicator-gtk3-devel libxdo-devel
```

This uses Fedora's Rust toolchain. Rustup is an alternative, not an additional requirement. Avoid competing toolchains without deciding which owns your PATH. AppIndicator is a Tauri prerequisite here; the application does not enable a tray feature.

For other environments or changed package availability, follow the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Other operating systems have not been verified for this project.

</details>

### Run the desktop app

```sh
git clone https://github.com/appyx123/Nodus-Studio.git
cd Nodus-Studio
npm ci
npm run tauri -- dev
```

Already in the project workspace? Skip the clone and `cd` commands. The first native build downloads dependencies and may take several minutes.

### Workbench controls

| Action | Control |
| --- | --- |
| Place a component | Click **Add to canvas**, or drag that button onto the canvas |
| Move an instance | Drag its painted body; position snaps to the grid |
| Connect pins | Click pin A then pin B on the canvas (or use the selected instance's **Pin** buttons); press Escape to cancel |
| Change wire color | Click a color button (**Green**, **Blue**, **Amber**, **Red**, **Violet**) before wiring or while a wire is selected |
| Pan | Drag empty canvas space, or middle-drag |
| Zoom | Scroll over the canvas, or use the zoom buttons |
| Nudge | Focus an instance with Tab, then use arrow keys |
| Select / delete | Click or focus an instance or wire; use **Delete selected**, Delete, or Backspace |
| Restore viewport | Click **Reset view**; instances and wires are not removed |

**Topology is session-only.** Closing or reloading the app clears the canvas. Library Reload refreshes discovery without deleting existing instances or wires. No electrical simulation or Lua behavior runs in Phase 1.

### Preview the frontend only

```sh
npm run dev
```

Open `http://127.0.0.1:1420`. Browser preview has no native filesystem access and reports that component discovery requires the desktop app. It is not a substitute for desktop validation.

### Build a native debug binary

```sh
npm run tauri -- build --no-bundle --debug
./src-tauri/target/debug/nodus
```

This runs without a Vite server and does not create distribution packages. The binary is named `nodus`; the application identifier is `org.nodus.Studio`.

> **Keep resources with the application.** On Linux, Tauri finds resources beside the binary in a recognized Cargo output directory, or under `../lib/Nodus Studio/` in an installed-style layout. Copying the binary alone is insufficient; discovery does not depend on the launch working directory.

For a release binary, omit `--debug`. That build remains unverified. RPM/Flatpak distribution is reserved for Phase 5.

## Development checks

```sh
# Frontend configuration, shared contract, types, and build
npm test
npm run build

# Rust formatting, behavior, and linting
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

`npm run build` includes typechecking; `npm run typecheck` is also available separately. Add `--offline` to Cargo checks when dependencies are cached.

Rust tests cover manifests, the shared serialization fixture, SVG rejection/normalization, package isolation, resource limits, symlinks/FIFOs, and resource-path construction. Node tests cover project configuration and positive/negative TypeScript contract checks. Neither replaces a desktop interaction test.

### Desktop smoke checks

After a native debug build, on GNOME/Wayland with system `pyatspi` already available:

```sh
python3 scripts/desktop-catalog-smoke.py
```

The bounded script launches a private relocated binary/resource copy, inspects accessibility by child PID, checks Add/zoom/reset/deletion plus catalog error/recovery paths, and cleans up. It changes no original package files, takes no screenshots, and installs no dependencies. Accessibility evidence is not pixel-level visual QA or a complete CSP audit.

For DOM interaction checks in real WebKitGTK, after `npm run build`, with system Python `gi`, GTK3, and WebKit2 4.1 already available:

```sh
python3 scripts/webkit-workbench-smoke.py
```

This separate, bounded harness serves `dist` on a temporary loopback port and mocks only the discovery boundary. It checks actual SVG screen transforms and hit-testing, but uses synthetic input, not physical mouse/keyboard events or native Tauri IPC. A fresh unmocked WebView also checks the browser-only fallback. The WebView and local server are closed on exit; it is not a production server.

## The `.nodpkg` component format

A package is a **directory**, not a ZIP archive:

```text
components-registry/
└── basic-led.nodpkg/
    ├── manifest.toml   # Metadata, pins, controls, visual declarations
    ├── graphic.svg     # Graphics validated by the Rust loader
    └── logic.lua       # Required text; not executed in Phase 1
```

The current contract supports slider declarations and numeric opacity fields. SVG features deliberately use a small fail-closed subset; rich effects are not silently accepted. Graphics use **1 SVG unit = 0.1 mm**, so standard 2.54 mm pin spacing equals 25.4 units.

Read the [package contract](docs/nodpkg-v1.md), inspect the [LED manifest](components-registry/basic-led.nodpkg/manifest.toml), or use the [shared normalized fixture](docs/nodpkg-v1-led.json).

**Security boundary:** file/payload limits and symlink checks are implemented, including Unix final-open hardening. This is not a race-proof filesystem sandbox: the built-in registry must not have hostile concurrent writers replacing parent directories. Lua is never executed or sent to the UI. The local window has only `allow-list-components`; no filesystem/shell plugin or remote capability is enabled.

## Roadmap

| Phase | Scope | Status |
| --- | --- | --- |
| **1 · Visual foundation** | Desktop shell, package loader, catalog, interactive canvas, wiring, Rust netlist | **Completed** — foundation, loader, catalog, interactive canvas, Manhattan wiring, and Rust netlist verified |
| **2 · Simulation core** | Deterministic scheduler, digital net resolution, Lua sandbox, binary visual frames | Not started |
| **3 · Firmware workflow** | AVR backend spike, Monaco, `arduino-cli`, first MCU integration | Not started |
| **4 · IoT components** | Sensors, displays, actuators, I2C/SPI, opt-in MQTT bridge | Not started |
| **5 · Polish and distribution** | Versioned project files, logic analyzer, RPM/Flatpak packaging | Not started |

**Next milestone:** Phase 2 — deterministic clock scheduler (`scheduler.rs`), digital pin/net resolution (`pin.rs`, `circuit_graph.rs`), isolated Lua component sandbox (`lua_host.rs`), 60 Hz binary visual frame streamer (`frame_streamer.rs`), and interactive power/pushbutton/LED packages.

Phases have explicit acceptance gates. Planned features are not release commitments. The MVP is not a general analog/SPICE solver; ESP32 emulation is outside the current deliverables.

## Repository map

```text
components-registry/   Built-in component packages
docs/                  Package contract, shared fixture, stage reports
scripts/               Configuration/contract tests and desktop smoke test
src/                   SolidJS interface and TypeScript types
src-tauri/             Rust backend, Tauri configuration, native lockfile
AGENTS.md              Repository rules for coding agents
PROGRESS.md            Current state, blockers, and next actions
```

## Contributing

Contributions are welcome. Start with the current milestone rather than skipping roadmap gates.

1. Read the [blueprint](master_project_blueprint_ai_context_specification.md), [repository instructions](AGENTS.md), and [progress summary](PROGRESS.md).
2. For substantial changes, [open an issue](https://github.com/appyx123/Nodus-Studio/issues) to discuss scope before building.
3. Keep changes focused; preserve the approved stack, backend hardware boundary, validation, and lockfiles.
4. Add focused tests and run the relevant checks above. State what you could not verify.
5. Include a new numbered [stage report](docs/reports/TEMPLATE.md) for meaningful development work and update `PROGRESS.md`—including partial or blocked work.
6. Open a [pull request](https://github.com/appyx123/Nodus-Studio/pulls) with the motivation, scope, and verification results.

Bug reports are most useful with your OS/session, tool versions, reproduction steps, and a minimal package fixture. **Do not post credentials, private tokens, or sensitive logs in public issues.**

## Documentation

- [Architecture and acceptance criteria](master_project_blueprint_ai_context_specification.md)
- [Current progress and next actions](PROGRESS.md)
- [Component package contract](docs/nodpkg-v1.md)
- [Wiring and netlist implementation](docs/reports/009-phase-1-wiring-and-netlist.md)
- [Canvas implementation and verification](docs/reports/008-phase-1-interactive-workbench.md)
- [Loader implementation and verification](docs/reports/005-phase-1-component-loader.md)
- [Stage report archive](docs/reports/)
- [Agent collaboration rules](AGENTS.md)

## License

[MIT](LICENSE) © 2026 Muhammad Rafli.

---

<div align="center">

**Connect the circuit. Understand the system. Build in the open.**

</div>
