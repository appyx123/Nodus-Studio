import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const directory = mkdtempSync(join(root, ".contract-test-"));
let geometry;
try {
  writeFileSync(join(directory, "tsconfig.json"), JSON.stringify({
    extends: "../tsconfig.json",
    compilerOptions: {
      noEmit: false,
      noEmitOnError: true,
      rootDir: join(root, "src/components/canvas"),
      outDir: join(directory, "compiled"),
      types: [],
      lib: ["ES2022"],
    },
    include: [join(root, "src/components/canvas/geometry.ts")],
  }));
  const result = spawnSync(process.execPath, [join(root, "node_modules/typescript/bin/tsc"),
    "--project", join(directory, "tsconfig.json")], {
    cwd: root, encoding: "utf8", timeout: 30_000,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  geometry = await import(pathToFileURL(join(directory, "compiled/geometry.js")).href);
} finally {
  rmSync(directory, { recursive: true, force: true });
}

const { GRID, MIN_ZOOM, MAX_ZOOM, screenToWorld, worldToScreen,
  snapPoint, zoomAt, dragPosition, pinWorldPoint, manhattanRoute, manhattanPath } = geometry;

function closePoint(actual, expected) {
  for (const axis of ["x", "y"]) {
    assert.ok(Math.abs(actual[axis] - expected[axis]) < 1e-9,
      `${axis}: expected ${expected[axis]}, received ${actual[axis]}`);
  }
}

test("geometry exports the physical grid and zoom limits", () => {
  assert.equal(GRID, 25.4);
  assert.equal(MIN_ZOOM, 0.2);
  assert.equal(MAX_ZOOM, 4);
});

test("screenToWorld removes viewport offsets before dividing by zoom", () => {
  assert.deepEqual(screenToWorld({ x: 130, y: -10 }, { x: 80, y: 40, zoom: 2 }),
    { x: 25, y: -25 });
});

test("worldToScreen scales negative world positions before adding offsets", () => {
  assert.deepEqual(worldToScreen({ x: -25, y: 15 }, { x: 80, y: -40, zoom: 2 }),
    { x: 30, y: -10 });
});

test("screen/world round trips hold across pans, fractional zooms, and negative coordinates", () => {
  for (const zoom of [MIN_ZOOM, 0.75, 1, 2, MAX_ZOOM]) {
    for (const viewport of [{ x: 0, y: 0, zoom }, { x: -137.5, y: 89.25, zoom }]) {
      for (const point of [{ x: 0, y: 0 }, { x: -76.2, y: 50.8 }, { x: 123.456, y: -987.654 }]) {
        closePoint(screenToWorld(worldToScreen(point, viewport), viewport), point);
        closePoint(worldToScreen(screenToWorld(point, viewport), viewport), point);
      }
    }
  }
});

test("snapPoint chooses the nearest grid intersection on both axes", () => {
  assert.deepEqual(snapPoint({ x: 38.2, y: 37.9 }), { x: 50.8, y: 25.4 });
  assert.deepEqual(snapPoint({ x: -38.2, y: -37.9 }), { x: -50.8, y: -25.4 });
});

test("snapPoint normalizes floating roundoff and negative zero", () => {
  assert.deepEqual(snapPoint({ x: 3 * GRID, y: -3 * GRID }), { x: 76.2, y: -76.2 });
  assert.deepEqual(snapPoint({ x: -0.01, y: -0 }), { x: 0, y: 0 });
});

test("snapPoint preserves grid positions and is idempotent", () => {
  for (const point of [{ x: 0, y: 0 }, { x: 25.4, y: -50.8 }, { x: -76.2, y: 101.6 }]) {
    assert.deepEqual(snapPoint(point), point);
    assert.deepEqual(snapPoint(snapPoint(point)), point);
  }
});

test("snapPoint uses Math.round ties toward positive infinity", () => {
  assert.deepEqual(snapPoint({ x: GRID / 2, y: -GRID / 2 }), { x: 25.4, y: 0 });
});

test("zoomAt preserves the world point under an offset cursor", () => {
  const viewport = { x: -120, y: 60, zoom: 2 };
  const cursor = { x: 180, y: -40 };
  const next = zoomAt(viewport, cursor, 3);
  assert.deepEqual(next, { x: -270, y: 110, zoom: 3 });
  closePoint(screenToWorld(cursor, next), screenToWorld(cursor, viewport));
});

test("zoomAt clamps both bounds while keeping the cursor anchored", () => {
  const viewport = { x: 75, y: -125, zoom: 2 };
  const cursor = { x: -80, y: 210 };
  for (const [requested, expected] of [[-1, MIN_ZOOM], [0, MIN_ZOOM], [99, MAX_ZOOM],
    [MIN_ZOOM, MIN_ZOOM], [MAX_ZOOM, MAX_ZOOM]]) {
    const next = zoomAt(viewport, cursor, requested);
    assert.equal(next.zoom, expected);
    closePoint(screenToWorld(cursor, next), screenToWorld(cursor, viewport));
  }
});

test("zoomAt keeps an unchanged zoom and supports a cursor at the screen origin", () => {
  const viewport = { x: 100, y: -50, zoom: 2 };
  assert.deepEqual(zoomAt(viewport, { x: -30, y: 20 }, 2), viewport);
  assert.deepEqual(zoomAt(viewport, { x: 0, y: 0 }, 1), { x: 50, y: -25, zoom: 1 });
});

test("dragPosition converts screen delta using zoom and snaps the resulting world position", () => {
  assert.deepEqual(dragPosition({ x: 25.4, y: -50.8 }, { x: 100, y: 200 },
    { x: 150.8, y: 149.2 }, { x: 350, y: -90, zoom: 2 }), { x: 50.8, y: -76.2 });
});

test("dragPosition respects fractional zoom and ignores viewport translation", () => {
  for (const zoom of [MIN_ZOOM, 0.5, 2, MAX_ZOOM]) {
    for (const offset of [0, -450]) {
      assert.deepEqual(dragPosition({ x: 0, y: 0 }, { x: -30, y: 60 },
        { x: -30 - GRID * zoom, y: 60 + GRID * 2 * zoom },
        { x: offset, y: -offset, zoom }), { x: -25.4, y: 50.8 });
    }
  }
});

test("dragPosition snaps the final position, not the delta, and handles zero movement", () => {
  const viewport = { x: 50, y: -90, zoom: 2 };
  assert.deepEqual(dragPosition({ x: 12, y: -12 }, { x: 0, y: 0 },
    { x: 4, y: -4 }, viewport), { x: 25.4, y: -25.4 });
  assert.deepEqual(dragPosition({ x: 26, y: -51 }, { x: 10, y: 20 },
    { x: 10, y: 20 }, viewport), { x: 25.4, y: -50.8 });
});

test("geometry operations do not mutate their inputs", () => {
  const point = Object.freeze({ x: -10, y: 30 });
  const cursor = Object.freeze({ x: 100, y: -20 });
  const viewport = Object.freeze({ x: 60, y: -80, zoom: 2 });
  screenToWorld(point, viewport);
  worldToScreen(point, viewport);
  snapPoint(point);
  zoomAt(viewport, cursor, 3);
  dragPosition(point, cursor, point, viewport);
  assert.deepEqual(point, { x: -10, y: 30 });
  assert.deepEqual(cursor, { x: 100, y: -20 });
  assert.deepEqual(viewport, { x: 60, y: -80, zoom: 2 });
});

test("pinWorldPoint offsets local pin coordinates by instance position and viewBox origin", () => {
  assert.deepEqual(pinWorldPoint({ x: 50.8, y: 101.6 }, { x: 25.4, y: 88.9 }), { x: 76.2, y: 190.5 });
  assert.deepEqual(pinWorldPoint({ x: 50.8, y: 101.6 }, { x: 25.4, y: 88.9 }, { x: 10, y: -20 }),
    { x: 66.2, y: 210.5 });
});

test("manhattanRoute generates orthogonal segments for aligned and offset endpoints", () => {
  assert.deepEqual(manhattanRoute({ x: 25.4, y: 50.8 }, { x: 25.4, y: 50.8 }),
    [{ x: 25.4, y: 50.8 }, { x: 25.4, y: 50.8 }]);
  assert.deepEqual(manhattanRoute({ x: 25.4, y: 50.8 }, { x: 127, y: 50.8 }),
    [{ x: 25.4, y: 50.8 }, { x: 127, y: 50.8 }]);
  assert.deepEqual(manhattanRoute({ x: 25.4, y: 50.8 }, { x: 25.4, y: 152.4 }),
    [{ x: 25.4, y: 50.8 }, { x: 25.4, y: 152.4 }]);
  const route = manhattanRoute({ x: 25.4, y: 88.9 }, { x: 127, y: 139.7 });
  assert.equal(route.length, 4);
  assert.deepEqual(route[0], { x: 25.4, y: 88.9 });
  assert.deepEqual(route[3], { x: 127, y: 139.7 });
  for (let index = 1; index < route.length; index++) {
    const prev = route[index - 1];
    const curr = route[index];
    assert.ok(prev.x === curr.x || prev.y === curr.y, `Segment ${index} is not orthogonal`);
  }
  assert.equal(manhattanPath({ x: 25.4, y: 88.9 }, { x: 127, y: 139.7 }),
    "M 25.4 88.9 L 76.2 88.9 L 76.2 139.7 L 127 139.7");
});
