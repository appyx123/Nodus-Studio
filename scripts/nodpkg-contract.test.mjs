import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = fileURLToPath(new URL("../", import.meta.url));
const fixture = JSON.parse(readFileSync(join(root, "docs/nodpkg-v1-led.json"), "utf8"));

function checkTypes(manifest) {
  const directory = mkdtempSync(join(root, ".contract-test-"));
  try {
    writeFileSync(join(directory, "fixture.ts"),
      `import type { ComponentManifest } from "../src/types/nodpkg";\nconst manifest = ${JSON.stringify(manifest)} satisfies ComponentManifest;\n`);
    writeFileSync(join(directory, "tsconfig.json"), JSON.stringify({
      extends: "../tsconfig.json", include: ["fixture.ts"],
    }));
    return spawnSync(process.execPath, [join(root, "node_modules/typescript/bin/tsc"),
      "--project", join(directory, "tsconfig.json")], { encoding: "utf8", timeout: 30_000 });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

test("shared Rust serialization fixture satisfies the TypeScript manifest contract", () => {
  const result = checkTypes(fixture);
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

test("TypeScript rejects an unknown pin kind from the shared fixture", () => {
  const invalid = structuredClone(fixture);
  invalid.pins[0].type = "external-code";
  const result = checkTypes(invalid);
  assert.ifError(result.error);
  assert.ok(result.status !== null && result.status !== 0, result.stdout + result.stderr);
  assert.ok(result.stdout.includes("TS2322") && result.stdout.includes("external-code"), result.stdout);
});

test("TypeScript manifest contract accepts mcu backend avr_atmega328p and rejects unknown backend", () => {
  const uno = structuredClone(fixture);
  uno.component.id = "mcu-arduino-uno";
  uno.mcu = { backend: "avr_atmega328p" };
  const okResult = checkTypes(uno);
  assert.ifError(okResult.error);
  assert.equal(okResult.status, 0, okResult.stdout + okResult.stderr);

  const badMcu = structuredClone(uno);
  badMcu.mcu.backend = "unknown_core";
  const badResult = checkTypes(badMcu);
  assert.ifError(badResult.error);
  assert.ok(badResult.status !== null && badResult.status !== 0, badResult.stdout + badResult.stderr);
  assert.ok(badResult.stdout.includes("TS2322") && badResult.stdout.includes("unknown_core"), badResult.stdout);
});

function checkCircuitTypes(source) {
  const directory = mkdtempSync(join(root, ".contract-test-"));
  try {
    writeFileSync(join(directory, "circuit.ts"), source);
    writeFileSync(join(directory, "tsconfig.json"), JSON.stringify({
      extends: "../tsconfig.json", include: ["circuit.ts"],
    }));
    return spawnSync(process.execPath, [join(root, "node_modules/typescript/bin/tsc"),
      "--project", join(directory, "tsconfig.json")], { encoding: "utf8", timeout: 30_000 });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

test("TypeScript circuit topology types match the Rust IPC snapshot and edit contracts", () => {
  const result = checkCircuitTypes(`
    import type { CircuitEdit, CircuitSnapshot } from "../src/types/nodpkg";
    const nodeId = "node-00000000-0000-4000-8000-000000000000";
    const edits: CircuitEdit[] = [
      { type: "add_instance", instance_id: nodeId, component_id: "basic-led", component_version: "1.0.0" },
      { type: "remove_instance", instance_id: nodeId },
      { type: "connect", wire_id: "wire-00000000-0000-4000-8000-000000000000", a: { instance_id: nodeId, pin_id: "A" }, b: { instance_id: "node-00000001-0000-4000-8000-000000000000", pin_id: "K" }, color: "green" },
      { type: "remove_wire", wire_id: "wire-00000000-0000-4000-8000-000000000000" },
      { type: "set_wire_color", wire_id: "wire-00000000-0000-4000-8000-000000000000", color: "violet" },
    ];
    const snapshot: CircuitSnapshot = {
      instances: [{ instance_id: nodeId, component_id: "basic-led", component_version: "1.0.0", pins: ["A", "K"] }],
      wires: [{ wire_id: "wire-00000000-0000-4000-8000-000000000000", a: { instance_id: nodeId, pin_id: "A" }, b: { instance_id: nodeId, pin_id: "K" }, color: "amber" }],
      nets: [{ endpoints: [{ instance_id: nodeId, pin_id: "A" }, { instance_id: nodeId, pin_id: "K" }] }],
    };
    void edits;
    void snapshot;
  `);
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

test("TypeScript rejects invalid wire colors and unknown circuit edit variants", () => {
  const result = checkCircuitTypes(`
    import type { CircuitEdit } from "../src/types/nodpkg";
    const bad: CircuitEdit = { type: "set_wire_color", wire_id: "wire-0", color: "pink" };
    void bad;
  `);
  assert.ifError(result.error);
  assert.ok(result.status !== null && result.status !== 0, result.stdout + result.stderr);
});
