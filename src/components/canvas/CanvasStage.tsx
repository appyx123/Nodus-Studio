import { For, Show } from "solid-js";
import type { CircuitSnapshot, Endpoint, WireColor } from "../../types/nodpkg";
import { GRID, PACKAGE_MIME } from "./geometry";
import type { Point, Viewport } from "./geometry";
import NodeRenderer, { type CanvasNode } from "./NodeRenderer";
import WireLayer from "./WireLayer";

export interface WirePreview {
  from: Endpoint;
  start: Point;
  end: Point;
  color: WireColor;
}

export interface CanvasStageProps {
  viewport: Viewport;
  circuit: CircuitSnapshot;
  nodes: CanvasNode[];
  selectedId: string | undefined;
  selectedWireId: string | undefined;
  wirePreview: WirePreview | undefined;
  pendingPin: Endpoint | undefined;
  stageRef: (el: SVGSVGElement) => void;
  onStageKey: (e: KeyboardEvent) => void;
  onPointerDown: (e: PointerEvent) => void;
  onPointerMove: (e: PointerEvent) => void;
  onPointerUp: (e: PointerEvent) => void;
  onPointerCancel: (e: PointerEvent) => void;
  onLostCapture: (e: PointerEvent) => void;
  onDrop: (e: DragEvent) => void;
  onSelectWire: (wireId: string) => void;
  onDeleteWire: (wireId: string) => void;
  onSelectNode: (instanceId: string) => void;
  onNodePointerDown: (e: PointerEvent, node: CanvasNode) => void;
  onNodeKey: (e: KeyboardEvent, node: CanvasNode) => void;
  onPinActivate: (endpoint: Endpoint) => void;
  endpointPosition: (endpoint: Endpoint) => Point | undefined;
}

export default function CanvasStage(props: CanvasStageProps) {
  return (
    <div class="workbench-stage relative flex-1">
      <svg
        ref={props.stageRef}
        data-workbench
        data-zoom={props.viewport.zoom}
        data-pan-x={props.viewport.x}
        data-pan-y={props.viewport.y}
        data-wire-count={props.circuit.wires.length}
        data-net-count={props.circuit.nets.length}
        class="workbench-svg"
        tabIndex={0}
        role="group"
        aria-label="Circuit canvas"
        aria-describedby="canvas-help"
        onKeyDown={(e) => props.onStageKey(e)}
        onPointerDown={(e) => props.onPointerDown(e)}
        onPointerMove={(e) => props.onPointerMove(e)}
        onPointerUp={(e) => props.onPointerUp(e)}
        onPointerCancel={(e) => props.onPointerCancel(e)}
        onLostPointerCapture={(e) => props.onLostCapture(e)}
        onDragOver={(e) => {
          if (e.dataTransfer?.types.includes(PACKAGE_MIME)) {
            e.preventDefault();
            e.dataTransfer.dropEffect = "copy";
          }
        }}
        onDrop={(e) => props.onDrop(e)}
      >
        <defs>
          <pattern
            id="workbench-grid"
            patternUnits="userSpaceOnUse"
            width={GRID * props.viewport.zoom}
            height={GRID * props.viewport.zoom}
            x={props.viewport.x}
            y={props.viewport.y}
          >
            <circle cx="0" cy="0" r="1" fill="#3f3f46" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#workbench-grid)" />
        <g
          data-world
          transform={`translate(${props.viewport.x} ${props.viewport.y}) scale(${props.viewport.zoom})`}
        >
          <WireLayer
            wires={props.circuit.wires}
            selectedWireId={props.selectedWireId}
            preview={props.wirePreview}
            endpointPosition={props.endpointPosition}
            onSelectWire={props.onSelectWire}
            onDeleteWire={props.onDeleteWire}
          />
          <For each={props.nodes.map((node) => node.instanceId)}>
            {(id) => {
              const node = () => props.nodes.find((item) => item.instanceId === id)!;
              return (
                <NodeRenderer
                  node={node()}
                  selected={props.selectedId === id && !props.selectedWireId}
                  activeEndpoint={props.pendingPin}
                  onPointerDown={props.onNodePointerDown}
                  onSelect={props.onSelectNode}
                  onKeyDown={props.onNodeKey}
                  onPinActivate={props.onPinActivate}
                />
              );
            }}
          </For>
        </g>
      </svg>

      <Show when={props.nodes.length === 0}>
        <div class="canvas-empty" aria-hidden="true">
          <p class="text-lg font-semibold text-zinc-200">A space for connected ideas.</p>
          <p class="mt-2 text-sm text-zinc-400">Click or drag a component from the library on the right.</p>
        </div>
      </Show>
    </div>
  );
}
