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
