import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = new URL("../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");
const pkg = JSON.parse(read("package.json"));
const config = JSON.parse(read("src-tauri/tauri.conf.json"));
const cargo = read("src-tauri/Cargo.toml");

test("desktop and frontend identities match the blueprint", () => {
  assert.equal(pkg.name, "nodus-studio");
  assert.equal(config.identifier, "org.nodus.Studio");
  assert.equal(config.productName, "Nodus Studio");
  assert.equal(config.version, pkg.version);
  assert.match(cargo, /^name = "nodus-studio"$/m);
  assert.match(cargo, /^version = "0\.1\.0"$/m);
  assert.match(cargo, /\[\[bin\]\]\nname = "nodus"/);
});

test("build hooks and entry points reference existing project files", () => {
  assert.equal(config.build.beforeDevCommand, "npm run dev");
  assert.equal(config.build.beforeBuildCommand, "npm run build");
  assert.equal(config.build.frontendDist, "../dist");
  assert.equal(new URL(config.build.devUrl).port, "1420");
  assert.match(read("vite.config.ts"), /port: 1420/);
  assert.match(read("index.html"), /src="\/src\/index\.tsx"/);
  for (const path of ["src/index.tsx", "src/App.tsx", "src/styles.css", "src-tauri/src/main.rs"]) {
    assert.ok(existsSync(fileURLToPath(new URL(path, root))), path);
  }
  for (const icon of config.bundle.icon) {
    const bytes = readFileSync(new URL(`src-tauri/${icon}`, root));
    assert.deepEqual([...bytes.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
  }
});

test("catalog grants only local discovery and retains a production CSP", () => {
  assert.equal(config.bundle.active, false);
  assert.equal(config.app.withGlobalTauri ?? false, false);
  assert.equal(config.app.security.assetProtocol?.enable ?? false, false);
  assert.deepEqual(config.app.security.capabilities, ["main-components"]);
  const capability = JSON.parse(read("src-tauri/capabilities/main.json"));
  assert.equal(capability.local, true);
  assert.equal(capability.remote, undefined);
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, [
    "allow-list-components",
    "allow-get-circuit",
    "allow-edit-circuit",
    "allow-get-simulation-frame",
    "allow-control-simulation",
    "allow-set-control-value",
    "allow-compile-sketch",
    "allow-cancel-compilation",
    "allow-serial-io",
  ]);
  assert.deepEqual(config.bundle.resources, { "../components-registry/": "components-registry/" });
  assert.ok(config.app.security.csp.includes("default-src 'self'"));
  assert.ok(config.app.security.csp.includes("object-src 'none'"));
  assert.ok(!config.app.security.csp.includes("'unsafe-eval'"));
  assert.ok(!config.app.security.csp.includes("'unsafe-inline'"));
  assert.equal(config.app.windows[0].label, "main");
  assert.equal(config.app.windows[0].dragDropEnabled, false);
});
