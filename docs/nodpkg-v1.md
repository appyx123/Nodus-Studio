# `.nodpkg` v1 — Phase 1 Package Contract

This document specifies the minimal implemented subset of blueprint section 4. The blueprint remains the architectural source of truth. Unsupported extensions fail closed; this contract does not introduce simulation in Phase 1.

## Package layout and discovery

A package is a direct directory `<component-id>.nodpkg/` inside the configured registry, not an archive. Required files are UTF-8 regular files:

| File | Maximum bytes | Behavior |
| --- | ---: | --- |
| `manifest.toml` | 65,536 | Parse and validate metadata |
| `graphic.svg` | 262,144 | Validate and rebuild safe XML |
| `logic.lua` | 131,072 | Require nonempty text; **never parse or execute Lua in Phase 1** |

Only directory packages are loaded. Files with the `.nodpkg` extension are ignored; archives are unsupported. Unrelated entries are ignored. Package errors produce individual diagnostics and do not prevent valid siblings loading. An unreadable/invalid registry or more than 512 direct entries is a registry-level error.

- Sorting is deterministic by directory filename; at most 128 package directories are processed. Excess package directories produce diagnostics.
- Returned package JSON is budgeted at 14 MiB; diagnostics are limited to 512 entries and 256 UTF-8 bytes per field, keeping the catalog below 16 MiB.
- Root, package, and required-file symlinks are rejected, even when they point inside the registry. Canonical parent containment is checked. Nonregular files are rejected; bounded reads detect growth beyond the file limit.
- On Unix the final file open also uses `O_NOFOLLOW | O_NONBLOCK`, followed by a descriptor type check, preventing final-component symlink following or FIFO blocking after pathname validation.
- The built-in registry is controlled by the application owner and must not be concurrently modified by hostile writers. Full descriptor-relative traversal against concurrent replacement of parent directories is **not implemented**. The final-open hardening is not a claim of race-proof filesystem confinement on all platforms.

### Runtime paths

- Tauri development (`cfg(dev)`): resolve the registry relative to `CARGO_MANIFEST_DIR`, not the launch working directory.
- Native builds, including `tauri build --debug`: resolve `components-registry/` through Tauri's runtime resource directory. `bundle.resources` maps `../components-registry/` to that relative resource location.
- No client-supplied path argument exists. User-installed registries and archive extraction are not implemented.

## Manifest fields

Unknown fields are rejected at **every object depth**; duplicate TOML keys are invalid. Names below are identical in TOML and the JSON IPC response.

| Object | Required fields |
| --- | --- |
| Root | `schema_version = 1`, `compatibility`, `component`, `electrical`, `pins` |
| `compatibility` | `engine_api = 1` |
| `component` | `id`, `name`, `category`, `author`, `version`, `description` |
| `electrical` | `operating_voltage_min`, `operating_voltage_max` |
| Each `pins` entry | `id`, `name`, `type`, `svg_anchor` |
| Each `controls` entry | `id`, `label`, `type = "slider"`, `min`, `max`, `step`, `default` |
| Each `visuals` entry | `id`, `type = "number"`, `target`, `property = "opacity"`, `min`, `max`, `default` |

`controls` and `visuals` are optional and normalize to empty arrays. Other missing fields fail validation. Fields without an explicit numeric type above are strings.

### Values and identity

- Component ID: lowercase ASCII kebab-case segments containing letters/digits, maximum 64 bytes. It must equal the package directory basename without `.nodpkg`.
- Component version: SemVer, without surrounding whitespace. Package schema, API compatibility, and component version are separate concepts.
- Required metadata text must be nonempty after trimming: name/category/author/version and pin/control labels at most 256 UTF-8 bytes; description at most 2,048 bytes. Text is displayed as text, never HTML.
- Pins: 1–128 entries. Types are `input`, `output`, `bidirectional`, `power`, `ground`, `nc`. These are package descriptors, not the future runtime drive/resolution model.
- Pin/control/visual IDs match `[A-Za-z][A-Za-z0-9_-]*`, at most 64 bytes; unique within each separate scope. Pin anchors start `pin-`, must exist in SVG, and cannot be reused by another pin. Pin IDs do not need to equal SVG IDs.
- Electrical values are finite and satisfy `0 <= min <= max <= 1000` volts. This is compatibility metadata, **not a current/analog solver**; the LED fixture's 0–5 V envelope does not model bare-LED current safety.
- Controls and visuals: at most 32 entries each. Values are finite with `min <= default <= max`; slider `step > 0`.
- Initial visual subset is numeric opacity: `0 <= min <= default <= max <= 1`. Target must be an existing `dyn-*` ID. Duplicate target/property bindings are rejected. Buffer, boolean, arbitrary properties, and executable DOM instructions are not supported.
- Visual defaults are declarations for a future renderer/runtime. In the Phase 1 static fixture, the SVG explicitly uses the matching default opacity. No visual updates or simulation controls are executed yet.

The full fixture is [`../components-registry/basic-led.nodpkg/manifest.toml`](../components-registry/basic-led.nodpkg/manifest.toml). Its normalized expected JSON is [`nodpkg-v1-led.json`](nodpkg-v1-led.json), shared by Rust serialization and TypeScript compile checks.

## SVG subset and sanitization

The Rust validator uses `quick-xml`, validates decoded values, and rebuilds XML using its writer. Unvalidated original SVG is not returned to the frontend. Comments are discarded; safe text/character references are escaped as text.

- Exactly one non-nested `<svg>` root with `xmlns="http://www.w3.org/2000/svg"` and a four-number `viewBox`. Width/height must be positive. Physical scale: **1 SVG unit = 0.1 mm**; 2.54 mm pin spacing = 25.4 units.
- Maximum 2,048 elements, nesting depth 32, and 16 KiB per path `d`. All numeric geometry is finite with absolute magnitude at most 100,000; nonnegative dimensions/radii and opacity ranges are checked separately.
- Elements: `svg`, `g`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`, `path`, `text`, `title`, `desc`. Text/title/desc cannot contain nested elements.
- IDs follow the ASCII identifier rule above and are unique across the SVG.
- General presentation: `fill`/`stroke` allow hex colors or `none`/`currentColor`; opacity attributes are 0–1; nonnegative bounded `stroke-width` is allowed.
- Geometry attributes are allowed only on matching elements: root/rect width and height, rect/text x/y, circle/ellipse cx/cy, circle r, rect/ellipse rx/ry, line endpoints, polyline/polygon points, and path d.
- Text can set bounded nonnegative `font-size`, `font-family` from `serif`/`sans-serif`/`monospace`, and `text-anchor` from `start`/`middle`/`end`.
- Reject unknown elements/attributes, namespace rebinding/prefixes, XML declarations, DTD/entities, processing instructions, scripts, event attributes, style/CSS, foreignObject, image/use/href, external URLs, transforms, gradients, and filters. Rich SVG effects require a reviewed extension, not silent bypasses.

Catalog previews use a bounded-size `<img>` data URL, **not `innerHTML` or inline DOM SVG**. Each preview has its own image document, so IDs do not collide between previews. The canvas renderer parses only Rust-normalized SVG with `DOMParser`, imports its DOM without `innerHTML`, and prefixes IDs with `<instance_id>--`. Local ID-reference rewriting does not expand the Rust allowlist: package references are still rejected by v1. Catalog preview sizes are UI thumbnails, not physical canvas placement. Nested canvas SVG dimensions match its viewBox extent, preserving world-unit scale; a nonzero viewBox origin is handled by the nested SVG viewport.

## IPC and UI

The only added command is `list_components`, with no frontend-supplied arguments. It runs scanning in a blocking worker and returns:

```json
{
  "packages": [{ "manifest": "normalized manifest object", "svg": "normalized SVG string" }],
  "diagnostics": [{ "path": "package path", "message": "validation error" }]
}
```

The placeholder strings illustrate envelope shape, not the actual manifest type. Lua source and file handles are never returned. Registry-level errors use the serialized `NodusError` message; package-level failures remain diagnostics. Full paths may appear locally in diagnostics; do not upload logs containing private paths unnecessarily.

The local `main` capability grants only `allow-list-components`. App command permissions are generated explicitly by the build script; no remote capability, native filesystem plugin, shell plugin, or global API is enabled. Production CSP remains active. The browser-only preview reports that desktop discovery is unavailable instead of bypassing the backend.

## Current scope

Implemented: package metadata/visual contract, loader, fail-closed SVG subset, static LED fixture, local discovery IPC, catalog previews, diagnostics, reload, and session-only visual placement with namespaced instances, pan/zoom, grid snapping, dragging, selection, keyboard nudging, and deletion.

Placement uses world coordinates with 25.4-unit grid spacing; `screen = viewport_translation + world * zoom`. Canvas position is the top-left of the nested SVG viewport. Add places near the visible center; drop places that origin under the pointer. Instances keep their package snapshot across catalog Reload and have distinct `node-<UUID>` identities. The frontend limits placement to 256 instances; it does not yet register circuit topology with Rust.

Not implemented: pin connection behavior, netlist, project persistence, Lua execution, live visual updates, scheduler, or firmware emulation. Those remain behind their blueprint roadmap gates.
