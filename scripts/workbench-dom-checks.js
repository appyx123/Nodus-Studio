/* Synthetic DOM interaction in real WebKitGTK, with discovery mocked by the harness.
 * This does not exercise native Tauri IPC, trusted pointer capture, or physical input.
 * Executed after load via evaluate_javascript; Promise results use a message handler.
 */
void (async () => {
  const { mode, fixtureSvg } = globalThis.__nodusSmokeOptions;
  const passed = [];
  const runtimeErrors = [];
  const delay = (ms = 35) => new Promise((resolve) => setTimeout(resolve, ms));
  const assert = (condition, message) => {
    if (!condition) throw new Error(message);
  };
  const close = (actual, expected, label, tolerance = 0.01) => {
    assert(Number.isFinite(actual) && Math.abs(actual - expected) <= tolerance,
      `${label}: expected ${expected}, received ${actual}`);
  };
  const pointClose = (actual, expected, label, tolerance) => {
    for (const axis of ["x", "y"]) close(actual[axis], expected[axis], `${label}.${axis}`, tolerance);
  };
  const poll = async (predicate, label, timeout = 1200) => {
    const deadline = performance.now() + timeout;
    while (performance.now() < deadline) {
      const result = predicate();
      if (result) return result;
      await delay(25);
    }
    throw new Error(`Timed out: ${label}`);
  };
  const test = async (name, run) => {
    await run();
    assert(runtimeErrors.length === 0, `Page runtime errors: ${runtimeErrors.join("; ")}`);
    passed.push(name);
  };
  const button = (name) => [...document.querySelectorAll("button")]
    .find((element) => (element.getAttribute("aria-label") ?? element.textContent.trim()) === name);
  const click = async (name) => {
    const element = button(name);
    assert(element && !element.disabled, `Missing or disabled button: ${name}`);
    element.click();
    await delay();
  };
  addEventListener("error", (event) => runtimeErrors.push(event.message));
  addEventListener("unhandledrejection", (event) => runtimeErrors.push(String(event.reason)));

  try {
    if (mode === "browser-fallback") {
      await test("fresh browser page has no Tauri mock and shows discovery fallback", async () => {
        assert(globalThis.isTauri !== true && typeof globalThis.__TAURI_INTERNALS__ === "undefined",
          "Browser fallback unexpectedly contains a Tauri boundary");
        await poll(() => [...document.querySelectorAll('[role="alert"]')].some((element) =>
          element.textContent.includes("Component discovery requires the Tauri desktop app")
          && element.textContent.includes("Browser preview has no filesystem access")),
        "browser-only discovery error", 6000);
        assert(document.querySelector("h1")?.textContent.trim() === "Nodus Studio", "App shell missing");
        assert(!button("Add Red LED"), "Unavailable browser catalog offers placement");
        assert(!document.querySelector("[data-instance-id]"), "Browser fallback manufactured an instance");
        assert(!document.querySelector('[data-component-id="basic-led"]'), "Browser fallback manufactured a package");
      });
    } else {
      assert(mode === "mocked-discovery", `Unknown smoke mode: ${mode}`);
      const GRID = 25.4;
      const MIN_ZOOM = 0.2;
      const MAX_ZOOM = 4;
      const snap = (value) => Number((Math.round(value / GRID) * GRID).toFixed(1)) || 0;
      let stage;
      let initialViewport;
      const nodes = () => [...stage.querySelectorAll("g[data-instance-id]")];
      const number = (element, name) => {
        assert(element.hasAttribute(name), `Missing ${name}`);
        const raw = element.getAttribute(name);
        assert(raw.trim() !== "", `Empty ${name}`);
        const value = Number(raw);
        assert(Number.isFinite(value), `Invalid ${name}: ${raw}`);
        return value;
      };
      const viewport = () => ({ x: number(stage, "data-pan-x"), y: number(stage, "data-pan-y"),
        zoom: number(stage, "data-zoom") });
      const position = (node) => ({ x: number(node, "data-x"), y: number(node, "data-y") });
      const rect = () => stage.getBoundingClientRect();
      const worldAt = (client, view = viewport()) => ({
        x: (client.x - rect().left - view.x) / view.zoom,
        y: (client.y - rect().top - view.y) / view.zoom,
      });
      const clientAt = (world, view = viewport()) => ({
        x: rect().left + view.x + world.x * view.zoom,
        y: rect().top + view.y + world.y * view.zoom,
      });
      const pointer = (target, type, client, buttons) => target.dispatchEvent(new PointerEvent(type, {
        bubbles: true, cancelable: true, pointerId: 1, pointerType: "mouse", isPrimary: true,
        button: 0, buttons, clientX: client.x, clientY: client.y,
      }));
      const drag = async (target, start, end) => {
        // Synthetic events cannot create native pointer capture. Move/up target the stage explicitly.
        pointer(target, "pointerdown", start, 1);
        await delay();
        pointer(stage, "pointermove", end, 1);
        await delay();
        pointer(stage, "pointerup", end, 0);
        await delay();
      };
      const selected = (node) => assert(node.getAttribute("aria-pressed") === "true", "Node not selected");
      const fixture = new DOMParser().parseFromString(fixtureSvg, "image/svg+xml");
      const originalIds = [...fixture.querySelectorAll("[id]")].map((element) => element.id);
      const fixturePin = fixture.getElementById("pin-anode");
      assert(fixturePin, "Fixture has no pin-anode");
      const pinLocal = { x: Number(fixturePin.getAttribute("cx")), y: Number(fixturePin.getAttribute("cy")) };
      const geometry = (node) => {
        const view = viewport();
        const pos = position(node);
        close(pos.x, snap(pos.x), "snapped node x");
        close(pos.y, snap(pos.y), "snapped node y");
        const world = stage.querySelector("[data-world]");
        assert(world && world.contains(node), "Node outside world group");
        const transforms = world.transform.baseVal;
        assert(transforms.numberOfItems === 2 && transforms.getItem(0).type === 2
          && transforms.getItem(1).type === 3, "World must translate then scale");
        const translation = transforms.getItem(0).matrix;
        pointClose({ x: translation.e, y: translation.f }, view, "world translation");
        const scale = transforms.getItem(1).matrix;
        close(scale.a, view.zoom, "world scale x");
        close(scale.d, view.zoom, "world scale y");
        assert(node.transform.baseVal.numberOfItems === 1 && node.transform.baseVal.getItem(0).type === 2,
          "Node group must have a single translate(x y)");
        const translationNode = node.transform.baseVal.getItem(0).matrix;
        pointClose({ x: translationNode.e, y: translationNode.f }, pos, "node translation");
        const imported = node.querySelector("svg");
        assert(imported, "Missing nested imported SVG root");
        const instanceId = node.getAttribute("data-instance-id");
        for (const element of imported.querySelectorAll("[id]")) {
          assert(element.id.startsWith(`${instanceId}--`), `Unnamespaced imported ID: ${element.id}`);
        }
        if (imported.id) assert(imported.id.startsWith(`${instanceId}--`), "Unnamespaced imported root ID");
        for (const id of originalIds) {
          assert(document.getElementById(`${instanceId}--${id}`)?.closest("g[data-instance-id]") === node,
            `Missing namespaced fixture ID: ${id}`);
        }
        const pin = imported.querySelector('[data-original-id="pin-anode"]');
        assert(pin && pin.id === `${instanceId}--pin-anode`, "Pin lost its original ID or namespace");
        close(number(pin, "cx"), pinLocal.x, "preserved pin cx");
        close(number(pin, "cy"), pinLocal.y, "preserved pin cy");
        const matrix = pin.getScreenCTM();
        assert(matrix, "Pin has no screen CTM");
        const actual = new DOMPoint(pinLocal.x, pinLocal.y).matrixTransform(matrix);
        const expected = clientAt({ x: pos.x + pinLocal.x, y: pos.y + pinLocal.y });
        pointClose(actual, expected, "pin screen position", 0.5);
        close(matrix.a, view.zoom, "pin physical x scale");
        close(matrix.d, view.zoom, "pin physical y scale");
      };
      const uniqueIds = () => {
        const ids = [...document.querySelectorAll("[id]")].map((element) => element.id);
        assert(new Set(ids).size === ids.length, "Duplicate IDs in document");
      };

      await test("mocked discovery renders the fixture catalog", async () => {
        await poll(() => [...document.querySelectorAll('[role="status"]')].some((element) =>
          element.textContent.includes("1 package(s) loaded")), "fixture discovery readiness", 6000);
        assert([...document.querySelectorAll("h3")].some((element) => element.textContent.trim() === "Red LED"),
          "Fixture catalog metadata missing");
      });
      await test("catalog readiness and 1 SVG unit = 1 CSS pixel workbench", async () => {
        await poll(() => button("Add Red LED"), "catalog Add Red LED button (build upcoming UI first)", 6000);
        stage = await poll(() => document.querySelector("svg[data-workbench]"), "workbench SVG");
        await delay(80);
        assert(!stage.hasAttribute("viewBox"), "Workbench must not rescale via viewBox");
        assert(rect().width > 200 && rect().height > 200, "Workbench has no usable screen geometry");
        assert(rect().left >= 0 && rect().top >= 0 && rect().right <= innerWidth + 1
          && rect().bottom <= innerHeight + 1, "Workbench does not fit the actual window");
        const matrix = stage.getScreenCTM();
        assert(matrix, "Workbench has no screen CTM");
        close(matrix.a, 1, "stage CSS x scale");
        close(matrix.d, 1, "stage CSS y scale");
        close(matrix.b, 0, "stage skew b");
        close(matrix.c, 0, "stage skew c");
        pointClose({ x: matrix.e, y: matrix.f }, { x: rect().left, y: rect().top }, "stage origin", 0.5);
        initialViewport = viewport();
        pointClose(initialViewport, { x: 60, y: 40 }, "initial pan");
        close(initialViewport.zoom, 2, "initial zoom");
        assert(nodes().length === 0, "Expected clean initial canvas");
      });
      let first;
      let second;
      await test("Add twice creates UUID instances, snapped positions, and unique imported IDs", async () => {
        await click("Add Red LED");
        await poll(() => nodes().length === 1, "first instance");
        first = nodes()[0];
        await click("Add Red LED");
        await poll(() => nodes().length === 2, "second instance");
        second = nodes().find((node) => node !== first);
        const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
        for (const node of [first, second]) {
          assert(uuid.test(node.getAttribute("data-instance-id").replace(/^node-/, "")), "Instance ID is not a UUID v4");
          assert(["true", "false"].includes(node.getAttribute("aria-pressed")), "Missing selection aria-pressed");
          geometry(node);
        }
        assert(first.dataset.instanceId !== second.dataset.instanceId, "Instances share identity");
        uniqueIds();
      });
      await test("painted SVG body receives hit testing for physical selection and drag", async () => {
        const pos = position(second);
        const client = clientAt({ x: pos.x + 38.1, y: pos.y + 34 });
        const hit = document.elementFromPoint(client.x, client.y);
        assert(hit?.closest("g[data-instance-id]") === second,
          `LED body hit background instead of its instance: ${hit?.tagName}`);
      });
      await test("pointer drag from the painted body works and cancellation restores position", async () => {
        const before = position(second);
        const start = clientAt({ x: before.x + 38.1, y: before.y + 34 });
        const hit = document.elementFromPoint(start.x, start.y);
        const end = { x: start.x + 60, y: start.y - 40 };
        pointer(hit, "pointerdown", start, 1);
        pointer(stage, "pointermove", end, 1);
        await delay();
        assert(position(second).x !== before.x, "Painted-body pointer did not move its instance");
        pointer(stage, "pointercancel", end, 0);
        await delay();
        pointClose(position(second), before, "cancelled node position");
        geometry(second);
        const view = viewport();
        pointer(stage, "pointerdown", { x: 320, y: 250 }, 1);
        pointer(stage, "pointermove", { x: 370, y: 270 }, 1);
        pointer(stage, "pointercancel", { x: 370, y: 270 }, 0);
        await delay();
        pointClose(viewport(), view, "cancelled pan");
      });
      await test("wheel zoom anchors the world point under an offset cursor", async () => {
        const cursor = { x: Math.round(rect().left + rect().width * 0.68),
                  y: Math.round(rect().top + rect().height * 0.61) };
        const before = viewport();
        const anchor = worldAt(cursor);
        stage.dispatchEvent(new WheelEvent("wheel", { bubbles: true, cancelable: true,
          clientX: cursor.x, clientY: cursor.y, deltaY: -120, deltaMode: 0 }));
        await poll(() => viewport().zoom > before.zoom, "wheel zoom in");
        pointClose(worldAt(cursor), anchor, "wheel world anchor");
        geometry(first);
        geometry(second);
      });
      await test("synthetic node drag snaps screen delta in zoomed world coordinates", async () => {
        const before = position(first);
        const start = clientAt({ x: before.x + 38.1, y: before.y + 34 });
        const end = { x: start.x + GRID * viewport().zoom * 1.3,
          y: start.y - GRID * viewport().zoom * 0.8 };
        const expected = { x: snap(before.x + (end.x - start.x) / viewport().zoom),
          y: snap(before.y + (end.y - start.y) / viewport().zoom) };
        await drag(first, start, end);
        pointClose(position(first), expected, "dragged world position");
        selected(first);
        assert(second.getAttribute("aria-pressed") === "false", "Previous selection not cleared");
        geometry(first);
        geometry(second);
      });
      await test("blank SVG pointer drag pans in CSS pixels without moving world positions", async () => {
        const before = viewport();
        const pos = position(first);
        const start = { x: rect().right - 40, y: rect().bottom - 50 };
        await drag(stage, start, { x: start.x - 73, y: start.y + 29 });
        pointClose(viewport(), { x: before.x - 73, y: before.y + 29 }, "panned viewport");
        close(viewport().zoom, before.zoom, "pan keeps zoom");
        pointClose(position(first), pos, "pan keeps node position");
        geometry(first);
        geometry(second);
      });
      await test("zoom buttons clamp limits and Reset view restores initial viewport", async () => {
        for (let index = 0; index < 40 && !button("Zoom in").disabled; index++) await click("Zoom in");
        close(viewport().zoom, MAX_ZOOM, "maximum zoom");
        geometry(first);
        for (let index = 0; index < 40 && !button("Zoom out").disabled; index++) await click("Zoom out");
        close(viewport().zoom, MIN_ZOOM, "minimum zoom");
        geometry(first);
        await click("Reset view");
        pointClose(viewport(), initialViewport, "reset pan");
        close(viewport().zoom, initialViewport.zoom, "reset zoom");
        geometry(first);
      });
      await test("catalog dragstart carries component MIME; valid drop snaps, unknown ID is ignored", async () => {
        const source = document.querySelector('[data-component-id="basic-led"]');
        assert(source && source.draggable && source.getAttribute("draggable") === "true", "Missing HTML drag source");
        const data = new DataTransfer();
        source.dispatchEvent(new DragEvent("dragstart", { bubbles: true, cancelable: true, dataTransfer: data }));
        assert(data.getData("application/x-nodus-component") === "basic-led", "Wrong component MIME payload");
        const client = { x: rect().left + rect().width * 0.77, y: rect().top + rect().height * 0.57 };
        const world = worldAt(client);
        const drop = async (transfer) => {
          for (const type of ["dragover", "drop"]) {
            stage.dispatchEvent(new DragEvent(type, { bubbles: true, cancelable: true,
              clientX: client.x, clientY: client.y, dataTransfer: transfer }));
            await delay();
          }
        };
        const previous = new Set(nodes());
        await drop(data);
        await poll(() => nodes().length === 3, "valid component drop");
        const dropped = nodes().find((node) => !previous.has(node));
        pointClose(position(dropped), { x: snap(world.x), y: snap(world.y) }, "drop world position");
        geometry(dropped);
        uniqueIds();
        source.dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: data }));
        const unknown = new DataTransfer();
        unknown.setData("application/x-nodus-component", "unknown-component");
        const ids = nodes().map((node) => node.dataset.instanceId).join(",");
        await drop(unknown);
        await delay(80);
        assert(nodes().map((node) => node.dataset.instanceId).join(",") === ids, "Unknown drop changed instances");
      });
      await test("selected-node arrow keys move one GRID and Delete removes the selected instance", async () => {
        const start = clientAt(position(first));
        await drag(first, start, start);
        selected(first);
        first.focus();
        for (const [key, dx, dy] of [["ArrowRight", GRID, 0], ["ArrowDown", 0, GRID],
          ["ArrowLeft", -GRID, 0], ["ArrowUp", 0, -GRID]]) {
          const before = position(first);
          first.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
          await delay();
          pointClose(position(first), { x: before.x + dx, y: before.y + dy }, key);
          assert(document.activeElement === first, "Moving an instance lost keyboard focus");
          geometry(first);
        }
        const deletedId = first.dataset.instanceId;
        first.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true, cancelable: true }));
        await poll(() => nodes().length === 2 && !first.isConnected, "keyboard Delete");
        assert(![...document.querySelectorAll("[id]")].some((element) => element.id.startsWith(`${deletedId}--`)),
          "Deleted node left imported IDs behind");
        geometry(second);
      });
      await test("pin-to-pin click connects Manhattan wire, updates nets, recolors, and tracks drag/zoom", async () => {
        const remaining = nodes();
        assert(remaining.length === 2, "Expected two nodes for wiring test");
        const [nodeA, nodeB] = remaining;
        const pinA = nodeA.querySelector('[data-pin-id="A"]');
        const pinK = nodeB.querySelector('[data-pin-id="K"]');
        assert(pinA && pinK, "Missing pin elements on canvas instances");
        pinA.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
        await poll(() => pinA.getAttribute("aria-pressed") === "true", "pending pin active");
        pinK.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
        const wireEl = await poll(() => stage.querySelector("g[data-wire-id]"), "wire element created");
        assert(number(stage, "data-wire-count") === 1, "Expected 1 wire in circuit");
        assert(number(stage, "data-net-count") === 3, "Expected 3 nets (1 connected pair + 2 singletons)");
        assert(wireEl.getAttribute("data-wire-color") === "green", "Default wire color must be green");
        const pathEl = wireEl.querySelector("path[data-wire-path]");
        assert(pathEl && pathEl.getAttribute("d").startsWith("M "), "Missing Manhattan path d");
        await click("Wire color Violet");
        await poll(() => wireEl.getAttribute("data-wire-color") === "violet", "wire recolored to violet");
        const beforePath = pathEl.getAttribute("d");
        nodeB.focus();
        nodeB.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true }));
        await delay();
        assert(pathEl.getAttribute("d") !== beforePath, "Moving endpoint instance must update wire path");
        await click("Delete selected");
        await poll(() => nodes().length === 1 && !nodeB.isConnected, "deleting wired instance");
        assert(!stage.querySelector("g[data-wire-id]"), "Incident wire must be removed when instance is deleted");
        assert(number(stage, "data-wire-count") === 0, "Wire count must return to 0");
        assert(number(stage, "data-net-count") === 2, "Remaining instance has 2 singleton pin nets");
        geometry(nodes()[0]);
        uniqueIds();
      });
    }
    globalThis.webkit.messageHandlers.nodusSmoke.postMessage(JSON.stringify({ mode, ok: true, passed }));
  } catch (error) {
    globalThis.webkit.messageHandlers.nodusSmoke.postMessage(JSON.stringify({ mode, ok: false, passed,
      error: `${String(error)}\n${error?.stack || ""}`, runtimeErrors }));
  }
})();
