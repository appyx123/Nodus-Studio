import { createMemo, createSignal, onCleanup, onMount, Show } from "solid-js";
import type { Resource } from "solid-js";
import { connectedNetCount, EMPTY_CIRCUIT } from "../../store/circuitStore";
import { editorOpen, setEditorOpen, sketchSource } from "../../store/editorStore";
import { analyzerOpen, setAnalyzerOpen } from "../../store/logicAnalyzerStore";
import { mqttOpen, setMqttOpen } from "../../store/mqttStore";
import { controlSimulation } from "../../store/simulationStore";
import type { CircuitSnapshot, Endpoint, RegistryCatalog, WireColor } from "../../types/nodpkg";
import LogicAnalyzerPanel from "../analyzer/LogicAnalyzerPanel";
import CodeEditor from "../editor/CodeEditor";
import SerialMonitor from "../editor/SerialMonitor";
import MqttBridgePanel from "../mqtt/MqttBridgePanel";
import ComponentCatalog from "../library/ComponentCatalog";
import CanvasNavigation from "./CanvasNavigation";
import CanvasStage from "./CanvasStage";
import ComponentInspector from "./ComponentInspector";
import { INITIAL_VIEWPORT, MAX_ZOOM, MIN_ZOOM, PACKAGE_MIME, screenToWorld } from "./geometry";
import type { Point } from "./geometry";
import type { CanvasNode } from "./NodeRenderer";
import { handleOpenAction, handleSaveAction } from "./projectPersistence";
import { createWorkbenchActions } from "./useWorkbenchActions";
import { createWorkbenchGestures } from "./useWorkbenchGestures";
import { WorkbenchStatus } from "./WorkbenchStatus";
import { WorkbenchToolbar } from "./WorkbenchToolbar";

interface Props {
  catalog: Resource<RegistryCatalog>;
  onReload: () => void;
}

interface HistoryStep {
  nodes: CanvasNode[];
  circuit: CircuitSnapshot;
}

export default function Workbench(props: Props) {
  let stageEl: SVGSVGElement | undefined;
  const [viewport, setViewport] = createSignal(INITIAL_VIEWPORT);
  const [nodes, setNodes] = createSignal<CanvasNode[]>([]);
  const [circuit, setCircuit] = createSignal<CircuitSnapshot>(EMPTY_CIRCUIT);
  const [selected, setSelected] = createSignal<string>();
  const [selectedWire, setSelectedWire] = createSignal<string>();
  const [wireColor, setWireColor] = createSignal<WireColor>("green");
  const [pendingPin, setPendingPin] = createSignal<Endpoint>();
  const [cursorWorld, setCursorWorld] = createSignal<Point>();
  const [catalogOpen, setCatalogOpen] = createSignal(true);
  const [message, setMessage] = createSignal("");
  const [errorMessage, setErrorMessage] = createSignal("");
  const [undoStack, setUndoStack] = createSignal<HistoryStep[]>([]);
  const [redoStack, setRedoStack] = createSignal<HistoryStep[]>([]);

  const selectedNode = createMemo(() => nodes().find((n) => n.instanceId === selected()));
  const activeWire = createMemo(() => circuit().wires.find((w) => w.wire_id === selectedWire()));
  const multiEndpointNets = createMemo(() => connectedNetCount(circuit().nets));

  function pushHistory() {
    const step: HistoryStep = {
      nodes: nodes().map((n) => ({ ...n, position: { ...n.position } })),
      circuit: JSON.parse(JSON.stringify(circuit())),
    };
    setUndoStack((prev) => [...prev.slice(-25), step]);
    setRedoStack([]);
  }

  function handleUndo() {
    const stack = undoStack();
    if (stack.length === 0) return;
    const prev = stack[stack.length - 1];
    setUndoStack((s) => s.slice(0, -1));
    setRedoStack((r) => [...r, { nodes: nodes(), circuit: circuit() }]);
    setNodes(prev.nodes);
    setCircuit(prev.circuit);
    setSelected(undefined);
    setSelectedWire(undefined);
    setMessage("Undo applied.");
  }

  function handleRedo() {
    const stack = redoStack();
    if (stack.length === 0) return;
    const next = stack[stack.length - 1];
    setRedoStack((r) => r.slice(0, -1));
    setUndoStack((s) => [...s, { nodes: nodes(), circuit: circuit() }]);
    setNodes(next.nodes);
    setCircuit(next.circuit);
    setSelected(undefined);
    setSelectedWire(undefined);
    setMessage("Redo applied.");
  }

  function rotateSelected() {
    const id = selected();
    if (!id) return;
    pushHistory();
    setNodes((items) =>
      items.map((n) => (n.instanceId === id ? { ...n, rotation: ((n.rotation ?? 0) + 90) % 360 } : n))
    );
    setMessage("Component rotated 90°.");
  }

  const gestures = createWorkbenchGestures({
    getStage: () => stageEl,
    viewport,
    setViewport,
    nodes,
    setNodes,
    selected,
    setSelected,
    selectedWire,
    setSelectedWire,
    pendingPin,
    setPendingPin,
    setCursorWorld,
    setMessage,
    onUndo: handleUndo,
    onRedo: handleRedo,
    onRotate: rotateSelected,
    onDeleteSelected: () => {
      void actions.removeSelected();
    },
  });

  const actions = createWorkbenchActions({
    viewport,
    nodes,
    setNodes,
    circuit,
    setCircuit,
    selected,
    setSelected,
    selectedWire,
    setSelectedWire,
    wireColor,
    setWireColor,
    pendingPin,
    setPendingPin,
    setCursorWorld,
    setMessage,
    setErrorMessage,
    pushHistory,
    hasActiveGesture: gestures.hasActiveGesture,
    focusStage: () => stageEl?.focus(),
  });

  const wirePreview = createMemo(() => {
    const from = pendingPin();
    const cursor = cursorWorld();
    const start = from ? actions.endpointPosition(from) : undefined;
    return from && cursor && start ? { from, start, end: cursor, color: wireColor() } : undefined;
  });

  function onDrop(event: DragEvent) {
    event.preventDefault();
    if (gestures.hasActiveGesture()) return;
    const id = event.dataTransfer?.getData(PACKAGE_MIME);
    const packages = !props.catalog.error && props.catalog()?.packages;
    const pkg = packages && packages.find((item) => item.manifest.component.id === id);
    if (pkg) void actions.place(pkg, screenToWorld(gestures.localPoint(event), viewport()));
  }

  onMount(() => {
    if (!stageEl) return;
    stageEl.addEventListener("wheel", gestures.onWheel, { passive: false });
    onCleanup(() => stageEl?.removeEventListener("wheel", gestures.onWheel));
  });

  return (
    <div class="workbench-layout">
      <main class="workbench-main" aria-labelledby="workbench-heading">
        <h2 id="workbench-heading" class="sr-only">
          Circuit workbench
        </h2>
        <WorkbenchToolbar
          wireColor={wireColor()}
          activeWireColor={activeWire()?.color}
          onChooseColor={(c) => {
            void actions.chooseColor(c);
          }}
          canRotate={Boolean(selected())}
          onRotateSelected={() => rotateSelected()}
          canDelete={Boolean(selected() || selectedWire())}
          onDeleteSelected={() => {
            void actions.removeSelected();
          }}
          canUndo={undoStack().length > 0}
          onUndo={() => handleUndo()}
          canRedo={redoStack().length > 0}
          onRedo={() => handleRedo()}
          editorOpen={editorOpen()}
          onToggleEditor={() => setEditorOpen((open) => !open)}
          mqttOpen={mqttOpen()}
          onToggleMqtt={() => setMqttOpen((open) => !open)}
          analyzerOpen={analyzerOpen()}
          onToggleAnalyzer={() => setAnalyzerOpen((open) => !open)}
          catalogOpen={catalogOpen()}
          onToggleCatalog={() => setCatalogOpen((open) => !open)}
          onStartSimulation={() => {
            void controlSimulation("start").catch((err) => setErrorMessage(String(err)));
          }}
          onStopSimulation={() => {
            void controlSimulation("stop").catch((err) => setErrorMessage(String(err)));
          }}
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
          onExportSchematic={() => gestures.exportSchematic()}
        />

        <div class="relative flex flex-1 min-h-0 overflow-hidden">
          <CanvasStage
            viewport={viewport()}
            circuit={circuit()}
            nodes={nodes()}
            selectedId={selected()}
            selectedWireId={selectedWire()}
            wirePreview={wirePreview()}
            pendingPin={pendingPin()}
            stageRef={(el) => {
              stageEl = el;
            }}
            onStageKey={gestures.stageKey}
            onPointerDown={gestures.startGesture}
            onPointerMove={gestures.moveGesture}
            onPointerUp={gestures.endGesture}
            onPointerCancel={(e) => gestures.endGesture(e, true)}
            onLostCapture={(e) => gestures.endGesture(e, true)}
            onDrop={onDrop}
            onSelectWire={(wId) => {
              setPendingPin(undefined);
              setSelected(undefined);
              setSelectedWire(wId);
            }}
            onDeleteWire={(wId) => {
              void actions.removeWireById(wId);
            }}
            onSelectNode={(instanceId) => {
              setSelectedWire(undefined);
              setSelected(instanceId);
            }}
            onNodePointerDown={gestures.startGesture}
            onNodeKey={gestures.nodeKey}
            onPinActivate={(endpoint) => {
              void actions.activatePin(endpoint);
            }}
            endpointPosition={actions.endpointPosition}
          />

          <Show when={selectedNode()}>
            <ComponentInspector
              node={selectedNode()}
              onClose={() => setSelected(undefined)}
              onRotate={() => rotateSelected()}
              onDelete={() => {
                void actions.removeSelected();
              }}
              onError={setErrorMessage}
            />
          </Show>

          <CanvasNavigation
            zoom={viewport().zoom}
            minZoom={MIN_ZOOM}
            maxZoom={MAX_ZOOM}
            onZoomIn={() => gestures.changeZoom(1.25)}
            onZoomOut={() => gestures.changeZoom(1 / 1.25)}
            onResetView={() => {
              if (!gestures.hasActiveGesture()) setViewport(INITIAL_VIEWPORT);
            }}
            onFitToView={() => gestures.fitToView()}
          />
        </div>

        <WorkbenchStatus
          nodeCount={nodes().length}
          wireCount={circuit().wires.length}
          netCount={circuit().nets.length}
          multiEndpointNets={multiEndpointNets()}
          pendingPin={pendingPin()}
          selectedNode={selectedNode()}
          activeWire={activeWire()}
          onActivatePin={(ep) => {
            void actions.activatePin(ep);
          }}
          message={message()}
          errorMessage={errorMessage()}
        />
      </main>

      <Show when={catalogOpen()}>
        <aside aria-label="Component packages" class="w-80 shrink-0 border-l border-zinc-800 bg-zinc-900">
          <ComponentCatalog
            catalog={props.catalog}
            onReload={props.onReload}
            onPlace={(pkg) => {
              void actions.place(pkg);
            }}
            onClose={() => setCatalogOpen(false)}
          />
        </aside>
      </Show>

      <Show when={editorOpen()}>
        <aside aria-label="Firmware and serial panel" class="editor-sidebar">
          <CodeEditor onStatus={setMessage} onError={setErrorMessage} />
          <SerialMonitor />
        </aside>
      </Show>

      <MqttBridgePanel />
      <Show when={analyzerOpen()}>
        <LogicAnalyzerPanel selectedEndpoint={pendingPin()} />
      </Show>
    </div>
  );
}
