import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { Resource } from "solid-js";
import type { ComponentPackage, RegistryCatalog } from "../../types/nodpkg";
import ComponentCatalog from "../library/ComponentCatalog";
import { dragPosition, GRID, MAX_ZOOM, MIN_ZOOM, screenToWorld, snapPoint, zoomAt } from "./geometry";
import type { Point, Viewport } from "./geometry";
import NodeRenderer from "./NodeRenderer";
import type { CanvasNode } from "./NodeRenderer";

interface Props {
  catalog: Resource<RegistryCatalog>;
  onReload: () => void;
}

type Gesture = {
  pointerId: number;
  start: Point;
  viewport: Viewport;
  node?: { id: string; position: Point };
};

const initialView = (): Viewport => ({ x: 60, y: 40, zoom: 2 });
const buttonClass = "workbench-button";
const PACKAGE_MIME = "application/x-nodus-component";
const MAX_INSTANCES = 256;

export default function Workbench(props: Props) {
  let stage!: SVGSVGElement;
  let gesture: Gesture | undefined;
  const [viewport, setViewport] = createSignal(initialView());
  const [nodes, setNodes] = createSignal<CanvasNode[]>([]);
  const [selected, setSelected] = createSignal<string>();
  const [message, setMessage] = createSignal("");
  const selectedNode = createMemo(() => nodes().find((node) => node.instanceId === selected()));

  function localPoint(event: { clientX: number; clientY: number }): Point {
    const bounds = stage.getBoundingClientRect();
    return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  }

  function moveNode(id: string, position: Point) {
    setNodes((items) => items.map((node) => node.instanceId === id ? { ...node, position } : node));
  }

  function place(pkg: ComponentPackage, point?: Point) {
    if (nodes().length >= MAX_INSTANCES) {
      setMessage("Canvas limit reached (256 instances). Delete an instance before adding another.");
      return;
    }
    const bounds = stage.getBoundingClientRect();
    const center = screenToWorld({ x: bounds.width / 2, y: bounds.height / 2 }, viewport());
    const offset = (nodes().length % 6) * GRID;
    const position = snapPoint(point ?? { x: center.x + offset, y: center.y + offset });
    const instanceId = `node-${crypto.randomUUID()}`;
    setNodes((items) => [...items, { instanceId, pkg, position }]);
    setSelected(instanceId);
    setMessage(`${pkg.manifest.component.name} added. Placement is session-only.`);
  }

  function startGesture(event: PointerEvent, node?: CanvasNode) {
    if (gesture || (event.button !== 0 && event.button !== 1)) return;
    event.preventDefault();
    event.stopPropagation();
    const movingNode = node && event.button === 0 ? node : undefined;
    gesture = {
      pointerId: event.pointerId, start: localPoint(event), viewport: { ...viewport() },
      node: movingNode ? { id: movingNode.instanceId, position: { ...movingNode.position } } : undefined,
    };
    if (movingNode) {
      setSelected(movingNode.instanceId);
      (event.currentTarget as SVGGElement).focus();
    } else {
      if (event.button === 0) setSelected(undefined);
      stage.focus();
    }
    if (event.isTrusted) stage.setPointerCapture(event.pointerId);
  }

  function moveGesture(event: PointerEvent) {
    if (!gesture || event.pointerId !== gesture.pointerId) return;
    const point = localPoint(event);
    if (gesture.node) {
      moveNode(gesture.node.id, dragPosition(gesture.node.position, gesture.start, point, gesture.viewport));
    } else {
      setViewport({ ...gesture.viewport,
        x: gesture.viewport.x + point.x - gesture.start.x,
        y: gesture.viewport.y + point.y - gesture.start.y });
    }
  }

  function endGesture(event: PointerEvent, cancel = false) {
    if (!gesture || event.pointerId !== gesture.pointerId) return;
    if (cancel) {
      if (gesture.node) moveNode(gesture.node.id, gesture.node.position);
      else setViewport(gesture.viewport);
    } else moveGesture(event);
    gesture = undefined;
    if (stage.hasPointerCapture(event.pointerId)) stage.releasePointerCapture(event.pointerId);
  }

  function changeZoom(factor: number, point?: Point) {
    if (gesture) return;
    const bounds = stage.getBoundingClientRect();
    setViewport((view) => zoomAt(view, point ?? { x: bounds.width / 2, y: bounds.height / 2 }, view.zoom * factor));
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault();
    const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? stage.clientHeight : 1;
    changeZoom(Math.exp(-Math.max(-300, Math.min(300, event.deltaY * unit)) * 0.002), localPoint(event));
  }

  function removeSelected() {
    if (gesture) return;
    setNodes((items) => items.filter((node) => node.instanceId !== selected()));
    setSelected(undefined);
    setMessage("Selected instance deleted.");
    stage.focus();
  }

  function nodeKey(event: KeyboardEvent, node: CanvasNode) {
    if (gesture) return;
    const directions: Record<string, Point> = {
      ArrowLeft: { x: -GRID, y: 0 }, ArrowRight: { x: GRID, y: 0 },
      ArrowUp: { x: 0, y: -GRID }, ArrowDown: { x: 0, y: GRID },
    };
    if (directions[event.key]) {
      event.preventDefault();
      event.stopPropagation();
      const step = directions[event.key];
      setSelected(node.instanceId);
      moveNode(node.instanceId, snapPoint({ x: node.position.x + step.x, y: node.position.y + step.y }));
    } else if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      event.stopPropagation();
      setSelected(node.instanceId);
      removeSelected();
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      event.stopPropagation();
      setSelected(node.instanceId);
    }
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    if (gesture) return;
    const id = event.dataTransfer?.getData(PACKAGE_MIME);
    const packages = !props.catalog.error && props.catalog()?.packages;
    const pkg = packages && packages.find((item) => item.manifest.component.id === id);
    if (pkg) place(pkg, screenToWorld(localPoint(event), viewport()));
  }

  onMount(() => {
    // A non-passive listener is required to prevent page scroll during viewport zoom.
    stage.addEventListener("wheel", onWheel, { passive: false });
    onCleanup(() => stage.removeEventListener("wheel", onWheel));
  });

  return (
    <div class="workbench-layout">
      <aside aria-label="Component packages" class="component-sidebar">
        <ComponentCatalog catalog={props.catalog} onReload={props.onReload} onPlace={place} />
      </aside>
      <main class="workbench-main" aria-labelledby="workbench-heading">
        <div class="workbench-toolbar">
          <div>
            <h2 id="workbench-heading" class="text-sm font-semibold">Circuit workbench</h2>
            <p class="mt-1 text-xs text-zinc-400">Visual placement · no wiring or simulation yet</p>
          </div>
          <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Viewport controls">
            <button class={buttonClass} aria-label="Zoom out" disabled={viewport().zoom <= MIN_ZOOM}
              onClick={() => changeZoom(1 / 1.25)}>−</button>
            <output class="min-w-14 text-center font-mono text-xs" aria-label="Zoom level">
              {Math.round(viewport().zoom * 100)}%
            </output>
            <button class={buttonClass} aria-label="Zoom in" disabled={viewport().zoom >= MAX_ZOOM}
              onClick={() => changeZoom(1.25)}>+</button>
            <button class={buttonClass} onClick={() => { if (!gesture) setViewport(initialView()); }}>Reset view</button>
            <button class={buttonClass} disabled={!selected()} onClick={removeSelected}>Delete selected</button>
          </div>
        </div>
        <div class="workbench-stage">
          <svg ref={stage} data-workbench data-zoom={viewport().zoom}
            data-pan-x={viewport().x} data-pan-y={viewport().y}
            class="workbench-svg" tabIndex={0} role="group" aria-label="Circuit canvas" aria-describedby="canvas-help"
            onPointerDown={(event) => startGesture(event)} onPointerMove={moveGesture}
            onPointerUp={(event) => endGesture(event)} onPointerCancel={(event) => endGesture(event, true)}
            onLostPointerCapture={(event) => endGesture(event, true)}
            onDragOver={(event) => {
              if (event.dataTransfer?.types.includes(PACKAGE_MIME)) {
                event.preventDefault();
                event.dataTransfer.dropEffect = "copy";
              }
            }} onDrop={onDrop}>
            <defs>
              <pattern id="workbench-grid" patternUnits="userSpaceOnUse"
                width={GRID * viewport().zoom} height={GRID * viewport().zoom}
                x={viewport().x} y={viewport().y}>
                <circle cx="0" cy="0" r="1" fill="#3f3f46" />
              </pattern>
            </defs>
            <rect width="100%" height="100%" fill="url(#workbench-grid)" />
            <g data-world transform={`translate(${viewport().x} ${viewport().y}) scale(${viewport().zoom})`}>
              <For each={nodes().map((node) => node.instanceId)}>{(id) => {
                const node = () => nodes().find((item) => item.instanceId === id)!;
                return <NodeRenderer node={node()} selected={selected() === id}
                  onPointerDown={startGesture} onSelect={setSelected} onKeyDown={nodeKey} />;
              }}</For>
            </g>
          </svg>
          <Show when={nodes().length === 0}>
            <div class="canvas-empty" aria-hidden="true">
              <p class="text-lg font-semibold text-zinc-200">A space for connected ideas.</p>
              <p class="mt-2 text-sm text-zinc-400">Add a component from the library or drag it here.</p>
            </div>
          </Show>
        </div>
        <div class="workbench-status">
          <p id="canvas-help">Drag empty space to pan · Scroll to zoom · Drag a component to move · Arrow keys to nudge</p>
          <p class="mt-1" role="status">{nodes().length} instance(s) · Grid 2.54 mm · Session-only placement</p>
          <Show when={selectedNode()}>{(node) => (
            <p class="mt-1 font-mono">Selected: {node().pkg.manifest.component.name} · x {node().position.x} · y {node().position.y}</p>
          )}</Show>
          <p class="sr-only" role="status">{message()}</p>
        </div>
      </main>
    </div>
  );
}
