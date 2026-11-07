import { For, Show } from "solid-js";
import type { Endpoint, Wire } from "../../types/nodpkg";
import type { CanvasNode } from "./NodeRenderer";

export interface WorkbenchStatusProps {
  nodeCount: number;
  wireCount: number;
  netCount: number;
  multiEndpointNets: number;
  pendingPin?: Endpoint;
  selectedNode?: CanvasNode;
  activeWire?: Wire;
  onActivatePin: (endpoint: Endpoint) => void;
  message: string;
  errorMessage: string;
}

const buttonClass = "workbench-button";

export function WorkbenchStatus(props: WorkbenchStatusProps) {
  return (
    <div class="workbench-status">
      <p id="canvas-help">
        Click pin A then pin B to wire · Drag empty space to pan · Scroll to zoom · Drag a component to move
      </p>
      <p class="mt-1" role="status">
        {props.nodeCount} instance(s) · {props.wireCount} wire(s) · {props.netCount} net(s) (
        {props.multiEndpointNets} connected) · Grid 2.54 mm
      </p>
      <Show when={props.pendingPin}>
        {(pin) => (
          <p class="mt-1 font-mono text-emerald-400" role="status">
            Wiring from {pin().instance_id}.{pin().pin_id} · Click target pin or press Escape
          </p>
        )}
      </Show>
      <Show when={props.selectedNode}>
        {(node) => (
          <div class="mt-1 flex flex-wrap items-center gap-2 font-mono">
            <span>
              Selected: {node().pkg.manifest.component.name} · x {node().position.x} · y {node().position.y}
            </span>
            <For each={node().pkg.manifest.pins.filter((pin) => pin.type !== "nc")}>
              {(pin) => (
                <button
                  type="button"
                  class={buttonClass}
                  aria-label={`Pin ${pin.id} on ${node().instanceId}`}
                  onClick={() =>
                    props.onActivatePin({
                      instance_id: node().instanceId,
                      pin_id: pin.id,
                    })
                  }
                >
                  Pin {pin.id}
                </button>
              )}
            </For>
          </div>
        )}
      </Show>
      <Show when={props.activeWire}>
        {(wire) => (
          <p class="mt-1 font-mono">
            Selected wire: {wire().wire_id} ({wire().color}) · {wire().a.instance_id}.{wire().a.pin_id} ↔{" "}
            {wire().b.instance_id}.{wire().b.pin_id}
          </p>
        )}
      </Show>
      <Show when={props.errorMessage}>
        <p class="mt-1 text-red-300" role="alert">
          {props.errorMessage}
        </p>
      </Show>
      <p class="sr-only" role="status">
        {props.message}
      </p>
    </div>
  );
}
