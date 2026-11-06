# Nodus Studio

An open-source desktop IoT workbench built with Tauri v2, Rust, SolidJS, TypeScript, Vite, and Tailwind CSS v4.

## Current status

Phase 1, step 2 is complete: a Rust `.nodpkg` loader validates manifests and a restricted SVG subset, then exposes the built-in LED package through a local-only Tauri command. The catalog shows isolated SVG previews, package diagnostics, and Reload. Lua is required as package text but is never executed or sent to the frontend.

Verification: 34 Rust tests, five Node tests, frontend typecheck/build, formatting, Clippy, and native debug build passed. A bounded Fedora GNOME/Wayland accessibility smoke test checked relocated Linux resources, loaded SVG content, invalid-package isolation, and error/reload recovery. Pixel-level visual QA and release packaging remain unverified. See [`PROGRESS.md`](PROGRESS.md) and [report 005](docs/reports/005-phase-1-component-loader.md).

The interactive canvas, wiring, netlist, simulation, and firmware editing are not implemented. Phase 1 as a whole remains `in-progress`.

Before contributing or handing work to another agent, read:

1. [`AGENTS.md`](AGENTS.md)
2. [`master_project_blueprint_ai_context_specification.md`](master_project_blueprint_ai_context_specification.md)
3. [`PROGRESS.md`](PROGRESS.md) and its latest numbered report

## Development prerequisites

- Node.js 22.12+ on the 22.x line, or Node.js 24+; this stage used Node.js 22.23.1.
- npm 10.9.8, with dependency versions recorded in `package-lock.json`.
- Rust/Cargo supporting Rust 1.90 or newer. The manifest uses Edition 2021; the detected Fedora Rust and Cargo versions are both 1.99.1. The actual compiler is recorded, not automatically pinned via rustup. `src-tauri/Cargo.lock` records the resolved native dependency graph; retain it and use `--locked` for Cargo checks.
- Native Tauri development dependencies for the target operating system.

### Fedora

These package names were found in the Fedora 45 local repository metadata. Run this from an interactive host terminal with administrator access; do not provide your password to an agent:

```sh
sudo dnf install rust cargo rustfmt clippy gcc-c++ make pkgconf-pkg-config webkit2gtk4.1-devel openssl-devel librsvg2-devel libappindicator-gtk3-devel libxdo-devel
```

This uses the Fedora Rust toolchain. `rustup` is an alternative, not an additional requirement; avoid installing competing toolchains without deciding which one owns your PATH. AppIndicator development support is listed in the official Tauri Fedora prerequisites; this shell does not enable a tray feature.

See the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) if package availability or your operating system differs. `arduino-cli`, a Lua runtime, and emulator dependencies are not needed at this stage.

## Install and check

Run from the existing workspace root:

```sh
npm ci
npm run typecheck
npm test
npm run build
```

The Node tests check scaffold identity, paths, narrow command permissions, CSP, and the shared Rust/TypeScript manifest fixture, including an invalid pin type. They do not compile Rust or exercise a browser/WebView.

After native dependencies are installed, run:

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

The 34 Rust tests cover manifest validation, shared serialization, SVG rejection/normalization, file and catalog limits, package isolation, symlinks/FIFOs, and runtime resource path construction. See [`docs/nodpkg-v1.md`](docs/nodpkg-v1.md) for the implemented contract and filesystem threat-model limits. Add `--offline` to Cargo checks when dependencies are already cached.

## Run

Frontend-only preview during development:

```sh
npm run dev
```

Open `http://127.0.0.1:1420`. Browser preview reports that component discovery requires the desktop app; it does not gain filesystem access. A browser preview is not a desktop smoke test.

After native prerequisites are available:

```sh
npm run tauri -- info
npm run tauri -- dev
```

For the verified native debug build, without creating distribution packages:

```sh
npm run tauri -- build --no-bundle --debug
./src-tauri/target/debug/nodus
```

The first build downloads dependencies and may take several minutes. Run the binary directly from its Cargo output directory to inspect the catalog without a Vite server. The app remains open until you close its window.

On Linux, Tauri resolves resources beside the binary in a recognized Cargo output directory, or from `../lib/Nodus Studio/` in an installed-style layout. Copying the binary alone is insufficient; do not rely on the current working directory.

For the bounded catalog smoke test on GNOME/Wayland with system `pyatspi` already available:

```sh
python3 scripts/desktop-catalog-smoke.py
```

The script launches a private relocated binary/resource copy, changes only that copy, inspects accessibility by child PID, and cleans up. It does not take screenshots or install dependencies.

For a release binary (not yet verified), omit `--debug`:

```sh
npm run tauri -- build --no-bundle
```

The application identifier is `org.nodus.Studio`; the Cargo binary is `nodus`. RPM/Flatpak packaging is reserved for Phase 5. The local main window has only the `allow-list-components` permission. No filesystem/shell plugin or remote capability is enabled.

## Next acceptance gate

Next is Phase 1, step 3: build `Workbench.tsx` and `NodeRenderer.tsx`, connect the existing catalog to placement, and add pan/zoom, 25.4-unit snap-to-grid, and drag-and-drop. Multiple inline SVG instances must namespace IDs and references per `instance_id`. Then step 4 adds wires and a Rust-owned netlist. Do not execute Lua in Phase 1.

Continue desktop interaction and visual checks as the UI develops. The current smoke test does not prove pixel appearance, every CSP directive, canvas behavior, or netlist correctness.

Each meaningful stage must create a new numbered report using [`docs/reports/TEMPLATE.md`](docs/reports/TEMPLATE.md) and update `PROGRESS.md`, even when blocked.
