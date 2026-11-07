import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { Resource } from "solid-js";
import { connectedNetCount, editCircuit, EMPTY_CIRCUIT, sameEndpoint } from "../../store/circuitStore";
import { editorOpen, setEditorOpen, sketchSource } from "../../store/editorStore";
import { mqttOpen, setMqttOpen } from "../../store/mqttStore";
import type { CircuitEdit, CircuitSnapshot, ComponentPackage, Endpoint, RegistryCatalog, WireColor } from "../../types/nodpkg";
import CodeEditor from "../editor/CodeEditor";
import SerialMonitor from "../editor/SerialMonitor";
import MqttBridgePanel from "../mqtt/MqttBridgePanel";
import ComponentCatalog from "../library/ComponentCatalog";
import {
  dragPosition,
  GRID,
  INITIAL_VIEWPORT,
  MAX_INSTANCES,
  MAX_ZOOM,
  MIN_ZOOM,
  PACKAGE_MIME,
  pinWorldPoint,
  screenToWorld,
  snapPoint,
  zoomAt,
} from "./geometry";
import type { Point, Viewport } from "./geometry";
import { instanceGraphic } from "./graphic";
import NodeRenderer from "./NodeRenderer";
import type { CanvasNode } from "./NodeRenderer";
import { handleOpenAction, handleSaveAction } from "./projectPersistence";
import SimulationBar from "./SimulationBar";
import { WorkbenchStatus } from "./WorkbenchStatus";
import { WorkbenchToolbar } from "./WorkbenchToolbar";
import WireLayer from "./WireLayer";

interface Props {
  catalog: Resource<RegistryCatalog>;
  onReload: () => void;
}

type Gesture = { pointerId: number; start: Point; viewport: Viewport; node?: { id: string; position: Point } };

export default function Workbench(props: Props) {
  let stage!: SVGSVGElement;
  let gesture: Gesture | undefined;
  const [viewport, setViewport] = createSignal(INITIAL_VIEWPORT);
  const [nodes, setNodes] = createSignal<CanvasNode[]>([]);
  const [circuit, setCircuit] = createSignal<CircuitSnapshot>(EMPTY_CIRCUIT);
  const [selected, setSelected] = createSignal<string>();
  const [selectedWire, setSelectedWire] = createSignal<string>();
  const [wireColor, setWireColor] = createSignal<WireColor>("green");
  const [pendingPin, setPendingPin] = createSignal<Endpoint>();
  const [cursorWorld, setCursorWorld] = createSignal<Point>();
  const [message, setMessage] = createSignal("");
  const [errorMessage, setErrorMessage] = createSignal("");
  const selectedNode = createMemo(() => nodes().find((node) => node.instanceId === selected()));
  const activeWire = createMemo(() => circuit().wires.find((wire) => wire.wire_id === selectedWire()));
  const multiEndpointNets = createMemo(() => connectedNetCount(circuit().nets));

  function localPoint(event: { clientX: number; clientY: number }): Point {
    const bounds = stage.getBoundingClientRect();
    return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  }

  function endpointPosition(endpoint: Endpoint): Point | undefined {
    const node = nodes().find((item) => item.instanceId === endpoint.instance_id);
    const localPin = node?.pinPositions.get(endpoint.pin_id);
    return node && localPin ? pinWorldPoint(node.position, localPin, node.viewBoxOrigin) : undefined;
  }

  const wirePreview = createMemo(() => {
    const from = pendingPin();
    const cursor = cursorWorld();
    const start = from ? endpointPosition(from) : undefined;
    return from && cursor && start ? { from, start, end: cursor, color: wireColor() } : undefined;
  });

  function moveNode(id: string, position: Point) {
    setNodes((items) => items.map((node) => node.instanceId === id ? { ...node, position } : node));
  }

  async function applyEdit(edit: CircuitEdit): Promise<CircuitSnapshot | undefined> {
    try {
      const snapshot = await editCircuit(edit);
      setCircuit(snapshot);
      setErrorMessage("");
      return snapshot;
    } catch (error) {
      const text = String(error);
      setErrorMessage(text);
      setMessage(text);
      return undefined;
    }
  }

  async function place(pkg: ComponentPackage, point?: Point) {
    if (nodes().length >= MAX_INSTANCES) {
      const text = "Canvas limit reached (256 instances). Delete an instance before adding another.";
      setErrorMessage(text);
      setMessage(text);
      return;
    }
    const bounds = stage.getBoundingClientRect();
    const center = screenToWorld({ x: bounds.width / 2, y: bounds.height / 2 }, viewport());
    const offset = (nodes().length % 6) * GRID;
    const position = snapPoint(point ?? { x: center.x + offset, y: center.y + offset });
    const instanceId = `node-${crypto.randomUUID()}`;
    const graphic = instanceGraphic(pkg.svg, instanceId, pkg.manifest.pins);
    const snapshot = await applyEdit({
      type: "add_instance", instance_id: instanceId,
      component_id: pkg.manifest.component.id, component_version: pkg.manifest.component.version,
    });
    if (!snapshot) return;
    setNodes((items) => [...items, {
      instanceId, pkg, position, viewBoxOrigin: graphic.viewBoxOrigin, pinPositions: graphic.pinPositions,
    }]);
    setSelectedWire(undefined);
    setSelected(instanceId);
    setMessage(`${pkg.manifest.component.name} added.`);
  }

  async function activatePin(endpoint: Endpoint) {
    if (gesture) return;
    const start = pendingPin();
    if (!start) {
      setPendingPin(endpoint);
      setCursorWorld(endpointPosition(endpoint));
      setSelectedWire(undefined);
      setErrorMessage("");
      setMessage(`Wiring from ${endpoint.pin_id}. Click a second pin to connect, or press Escape to cancel.`);
      return;
    }
    if (sameEndpoint(start, endpoint)) {
      setPendingPin(undefined);
      setMessage("Wiring cancelled.");
      return;
    }
    const wireId = `wire-${crypto.randomUUID()}`;
    const snapshot = await applyEdit({ type: "connect", wire_id: wireId, a: start, b: endpoint, color: wireColor() });
    setPendingPin(undefined);
    if (!snapshot) return;
    setSelectedWire(wireId);
    setMessage(`Connected ${start.pin_id} to ${endpoint.pin_id}.`);
  }

  async function chooseColor(color: WireColor) {
    setWireColor(color);
    const wireId = selectedWire();
    if (wireId && (await applyEdit({ type: "set_wire_color", wire_id: wireId, color }))) {
      setMessage(`Wire color changed to ${color}.`);
    }
  }

  async function removeWireById(wireId: string) {
    if (gesture || !(await applyEdit({ type: "remove_wire", wire_id: wireId }))) return;
    if (selectedWire() === wireId) setSelectedWire(undefined);
    setMessage("Selected wire deleted.");
    stage.focus();
  }

  async function removeSelected() {
    if (gesture) return;
    const wireId = selectedWire();
    if (wireId) return removeWireById(wireId);
    const nodeId = selected();
    if (!nodeId || !(await applyEdit({ type: "remove_instance", instance_id: nodeId }))) return;
    setNodes((items) => items.filter((node) => node.instanceId !== nodeId));
    setSelected(undefined);
    if (pendingPin()?.instance_id === nodeId) setPendingPin(undefined);
    setMessage("Selected instance deleted.");
    stage.focus();
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
      setSelectedWire(undefined);
      setSelected(movingNode.instanceId);
      (event.currentTarget as SVGGElement).focus();
    } else {
      if (event.button === 0) {
        setSelected(undefined);
        setSelectedWire(undefined);
        setPendingPin(undefined);
      }
      stage.focus();
    }
    if (event.isTrusted) stage.setPointerCapture(event.pointerId);
  }

  function moveGesture(event: PointerEvent) {
    const point = localPoint(event);
    if (pendingPin()) setCursorWorld(screenToWorld(point, viewport()));
    if (!gesture || event.pointerId !== gesture.pointerId) return;
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

  function stageKey(event: KeyboardEvent) {
    if (event.key === "Escape" && pendingPin()) {
      event.preventDefault();
      setPendingPin(undefined);
      setMessage("Wiring cancelled.");
    } else if ((event.key === "Delete" || event.key === "Backspace") && (selected() || selectedWire())) {
      event.preventDefault();
      void removeSelected();
    }
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
      setSelectedWire(undefined);
      setSelected(node.instanceId);
      moveNode(node.instanceId, snapPoint({ x: node.position.x + step.x, y: node.position.y + step.y }));
    } else if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      event.stopPropagation();
      setSelectedWire(undefined);
      setSelected(node.instanceId);
      void removeSelected();
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      event.stopPropagation();
      setSelectedWire(undefined);
      setSelected(node.instanceId);
    }
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    if (gesture) return;
    const id = event.dataTransfer?.getData(PACKAGE_MIME);
    const packages = !props.catalog.error && props.catalog()?.packages;
    const pkg = packages && packages.find((item) => item.manifest.component.id === id);
    if (pkg) void place(pkg, screenToWorld(localPoint(event), viewport()));
  }

  onMount(() => {
    stage.addEventListener("wheel", onWheel, { passive: false });
    onCleanup(() => stage.removeEventListener("wheel", onWheel));
  });

  return (
    <div class="workbench-layout">
      <aside aria-label="Component packages" class="component-sidebar">
        <ComponentCatalog catalog={props.catalog} onReload={props.onReload} onPlace={(pkg) => { void place(pkg); }} />
      </aside>
      <main class="workbench-main" aria-labelledby="workbench-heading">
        <div class="workbench-toolbar">
          <div>
            <h2 id="workbench-heading" class="text-sm font-semibold">Circuit workbench</h2>
            <p class="mt-1 text-xs text-zinc-400">Authoritative Rust topology · no electrical simulation yet</p>
          </div>
          <SimulationBar selectedNode={selectedNode()} onError={setErrorMessage} onStatus={setMessage} />
          <WorkbenchToolbar
            wireColor={wireColor()}
            activeWireColor={activeWire()?.color}
            onChooseColor={(c) => { void chooseColor(c); }}
            zoom={viewport().zoom}
            minZoom={MIN_ZOOM}
            maxZoom={MAX_ZOOM}
            onChangeZoom={(factor) => changeZoom(factor)}
            onResetView={() => { if (!gesture) setViewport(INITIAL_VIEWPORT); }}
            canDelete={Boolean(selected() || selectedWire())}
            onDeleteSelected={() => { void removeSelected(); }}
            editorOpen={editorOpen()}
            onToggleEditor={() => setEditorOpen((open) => !open)}
            mqttOpen={mqttOpen()}
            onToggleMqtt={() => setMqttOpen((open) => !open)}
            onSaveProject={() => {
              void handleSaveAction(nodes(), circuit(), viewport(), sketchSource(), setMessage, setErrorMessage);
            }}
            onOpenProject={(e) => {
              void handleOpenAction(
                e,
                props.catalog()?.packages ?? [],
                (res) => {
                  setNodes(res.nodes);
                  setCircuit(res.circuit);
                  setViewport(res.viewport);
                  setSelected(undefined);
                  setSelectedWire(undefined);
                },
                setMessage,
                setErrorMessage
              );
            }}
          />
        </div>
        <div class="workbench-stage">
          <svg ref={stage} data-workbench data-zoom={viewport().zoom} data-pan-x={viewport().x} data-pan-y={viewport().y}
            data-wire-count={circuit().wires.length} data-net-count={circuit().nets.length}
            class="workbench-svg" tabIndex={0} role="group" aria-label="Circuit canvas" aria-describedby="canvas-help"
            onKeyDown={stageKey} onPointerDown={(event) => startGesture(event)} onPointerMove={moveGesture}
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
                width={GRID * viewport().zoom} height={GRID * viewport().zoom} x={viewport().x} y={viewport().y}>
                <circle cx="0" cy="0" r="1" fill="#3f3f46" />
              </pattern>
            </defs>
            <rect width="100%" height="100%" fill="url(#workbench-grid)" />
            <g data-world transform={`translate(${viewport().x} ${viewport().y}) scale(${viewport().zoom})`}>
              <WireLayer wires={circuit().wires} selectedWireId={selectedWire()} preview={wirePreview()}
                endpointPosition={endpointPosition}
                onSelectWire={(wireId) => { setPendingPin(undefined); setSelected(undefined); setSelectedWire(wireId); }}
                onDeleteWire={(wireId) => { void removeWireById(wireId); }} />
              <For each={nodes().map((node) => node.instanceId)}>{(id) => {
                const node = () => nodes().find((item) => item.instanceId === id)!;
                return <NodeRenderer node={node()} selected={selected() === id && !selectedWire()} activeEndpoint={pendingPin()}
                  onPointerDown={startGesture}
                  onSelect={(instanceId) => { setSelectedWire(undefined); setSelected(instanceId); }}
                  onKeyDown={nodeKey} onPinActivate={(endpoint) => { void activatePin(endpoint); }} />;
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
        <WorkbenchStatus
          nodeCount={nodes().length}
          wireCount={circuit().wires.length}
          netCount={circuit().nets.length}
          multiEndpointNets={multiEndpointNets()}
          pendingPin={pendingPin()}
          selectedNode={selectedNode()}
          activeWire={activeWire()}
          onActivatePin={(ep) => { void activatePin(ep); }}
          message={message()}
          errorMessage={errorMessage()}
        />
      </main>
      <Show when={editorOpen()}>
        <aside aria-label="Firmware and serial panel" class="editor-sidebar">
          <CodeEditor onStatus={setMessage} onError={setErrorMessage} />
          <SerialMonitor />
        </aside>
      </Show>
      <MqttBridgePanel />
    </div>
  );
}
