import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const directory = mkdtempSync(join(root, ".contract-test-"));
let mod;
try {
  writeFileSync(join(directory, "tsconfig.json"), JSON.stringify({
    extends: "../tsconfig.json",
    compilerOptions: { noEmit: false, noEmitOnError: true, rootDir: join(root, "src/store"),
      outDir: join(directory, "compiled"), types: [], lib: ["ES2022", "DOM"] },
    include: [join(root, "src/store/frameDecoder.ts")],
  }));
  const result = spawnSync(process.execPath, [join(root, "node_modules/typescript/bin/tsc"),
    "--project", join(directory, "tsconfig.json")], { cwd: root, encoding: "utf8", timeout: 30_000 });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  writeFileSync(join(directory, "compiled/package.json"), '{"type":"module"}');
  mod = await import(pathToFileURL(join(directory, "compiled/frameDecoder.js")).href);
} finally {
  rmSync(directory, { recursive: true, force: true });
}

// Mirrors the Rust frame_streamer layout (little-endian).
function build() {
  const enc = new TextEncoder();
  const parts = [];
  const push = (n, fn) => { const b = new DataView(new ArrayBuffer(n)); fn(b); parts.push(new Uint8Array(b.buffer)); };
  const str = (s) => { const e = enc.encode(s); push(2, (v) => v.setUint16(0, e.length, true)); parts.push(e); };
  push(32, (v) => {
    "NDFR".split("").forEach((c, i) => v.setUint8(i, c.charCodeAt(0)));
    v.setUint8(4, 1); v.setUint8(5, 1); v.setUint16(6, 3, true);
    v.setBigUint64(8, 42n, true); v.setBigUint64(16, 2_000_000_000n, true);
    v.setFloat32(24, 0.5, true); v.setUint16(28, 1, true); v.setUint16(30, 1, true);
  });
  str("node-a"); push(1, (v) => v.setUint8(0, 0)); push(2, (v) => v.setUint16(0, 1, true));
  str("brightness"); push(8, (v) => v.setFloat64(0, 0.75, true));
  push(1, (v) => v.setUint8(0, 3)); push(1, (v) => v.setUint8(0, 1)); push(2, (v) => v.setUint16(0, 1, true));
  str("node-a"); str("A");
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let o = 0; for (const p of parts) { out.set(p, o); o += p.length; }
  return out.buffer;
}

test("decodes a Rust-layout frame", () => {
  const f = mod.decodeFrame(build());
  assert.equal(f.seq, 42n);
  assert.equal(f.run_state, "running");
  assert.equal(f.sim_time_ns, 1_000_000_000);
  assert.equal(f.speed_ratio, 0.5);
  assert.equal(f.has_conflict, true);
  assert.equal(f.has_fault, true);
  assert.deepEqual(f.instances, [{ instance_id: "node-a", faulted: false, visuals: { brightness: 0.75 } }]);
  assert.deepEqual(f.nets, [{ state: "UNKNOWN", conflict: true, endpoints: [{ instance_id: "node-a", pin_id: "A" }] }]);
});

test("rejects truncated, bad magic, bad version, trailing bytes", () => {
  const good = build();
  for (let cut = 0; cut < good.byteLength; cut++) assert.throws(() => mod.decodeFrame(good.slice(0, cut)));
  const bad = new Uint8Array(good.slice(0)); bad[0] = 88;
  assert.throws(() => mod.decodeFrame(bad.buffer));
  const ver = new Uint8Array(good.slice(0)); ver[4] = 9;
  assert.throws(() => mod.decodeFrame(ver.buffer));
  const trail = new Uint8Array(good.byteLength + 1); trail.set(new Uint8Array(good));
  assert.throws(() => mod.decodeFrame(trail.buffer));
});
