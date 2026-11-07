# Nodus Project File Format Specification (v1.0.0)

## Overview

The `.nodus` project file is the authoritative persistence format for Nodus Studio workspaces. It serializes the entire state of an interactive workbench session, including circuit graph topology, physical component coordinates, visual controls, firmware sketch source code, target microcontroller configuration, and virtual IoT bridge settings.

## Format Specification

- **File extension:** `.nodus`
- **MIME type:** `application/x-nodus-project+json`
- **Encoding:** UTF-8 JSON
- **Size limit:** Maximum 10 MiB (10,485,760 bytes)
- **Schema version:** `1.0.0`

## JSON Schema Structure

```json
{
  "format": "nodus-project",
  "version": "1.0.0",
  "app_id": "org.nodus.Studio",
  "metadata": {
    "name": "Blink Circuit",
    "created_at": 1730000000,
    "modified_at": 1730000000,
    "description": "Optional project description"
  },
  "viewport": {
    "pan": { "x": 0.0, "y": 0.0 },
    "zoom": 1.0
  },
  "circuit": {
    "instances": [
      {
        "instance_id": "node-550e8400-e29b-41d4-a716-446655440000",
        "component_id": "basic-led",
        "component_version": "1.0.0",
        "position": { "x": 100.0, "y": 150.0 },
        "control_values": { "slider": 0.5 }
      }
    ],
    "wires": [
      {
        "wire_id": "wire-550e8400-e29b-41d4-a716-446655440001",
        "a": { "instance_id": "node-550e8400-e29b-41d4-a716-446655440000", "pin_id": "anode" },
        "b": { "instance_id": "node-550e8400-e29b-41d4-a716-446655440002", "pin_id": "d13" },
        "color": "green"
      }
    ]
  },
  "firmware": {
    "sketch_source": "void setup() { pinMode(13, OUTPUT); }\nvoid loop() { ... }",
    "fqbn": "arduino:avr:uno"
  },
  "mqtt_bridge": {
    "enabled": false,
    "host": "localhost",
    "port": 1883,
    "client_id": "nodus-client",
    "sub_topics": [],
    "pub_topics": []
  }
}
```

## Package Dependency Policy

1. **Referencing vs Embedding:**
   - Component packages (`.nodpkg`) are **referenced** by canonical ID and SemVer version, never embedded inside the project file.
   - This ensures projects stay lightweight (< 100 KiB typically), diff cleanly in version control, and benefit automatically from bug fixes in the component registry.

2. **Version Migration Rules:**
   - SemVer compatibility: Minor and patch versions within the same major version are automatically accepted.
   - Major version mismatches emit diagnostic warnings (`ProjectWarning`) to notify the user of potential pinout or behavior changes.

3. **Missing Package Recovery:**
   - If a referenced package is absent from the local registry, the project load **does not abort or panic**.
   - The instance is preserved with its original coordinates, instance ID, and connected wires.
   - A `ProjectWarning` of level `Warning` is reported, allowing the user to inspect the circuit, install the missing package, or substitute the component without losing work.

## Security & Reliability Guarantees

1. **No Stored Credentials:**
   - MQTT passwords, secret authentication keys, and user credentials are **strictly prohibited** from serialization into `.nodus` files. Any credential fields encountered during export or save are sanitized/stripped.

2. **Atomic Write:**
   - Projects are saved by writing to a temporary file in the target directory (`<path>.tmp.<uuid>`), flushing and syncing to storage, and executing an atomic filesystem rename (`std::fs::rename`). This eliminates corruption or truncation from sudden process interrupts or power loss.

3. **Atomic Load & State Protection:**
   - Validation and parsing occur in Rust before touching active circuit or editor state.
   - If a project file is corrupted or incompatible, the active session is 100% preserved.

4. **Resource Bounds:**
   - Enforces a 10 MiB maximum file size limit.
   - Rejects path traversal attacks and non-regular files (FIFOs, device nodes, symlinks outside workspace).
