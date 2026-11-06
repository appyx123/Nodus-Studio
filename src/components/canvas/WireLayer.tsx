import { For, Show } from "solid-js";
import { wireHex } from "../../store/circuitStore";
import type { Endpoint, Wire, WireColor } from "../../types/nodpkg";
import { manhattanPath } from "./geometry";
import type { Point } from "./geometry";

export interface WirePreview {
  from: Endpoint;
  start: Point;
  end: Point;
  color: WireColor;
}

interface Props {
  wires: Wire[];
  selectedWireId: string | undefined;
  preview: WirePreview | undefined;
  endpointPosition: (endpoint: Endpoint) => Point | undefined;
  onSelectWire: (wireId: string) => void;
  onDeleteWire: (wireId: string) => void;
}

export default function WireLayer(props: Props) {
  function wireKey(event: KeyboardEvent, wireId: string) {
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      event.stopPropagation();
      props.onDeleteWire(wireId);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      event.stopPropagation();
      props.onSelectWire(wireId);
    }
  }

  return (
    <g data-wire-layer aria-label="Circuit wires">
      <For each={props.wires.map((wire) => wire.wire_id)}>{(wireId) => {
        const wire = () => props.wires.find((item) => item.wire_id === wireId)!;
        const start = () => props.endpointPosition(wire().a);
        const end = () => props.endpointPosition(wire().b);
        const isSelected = () => props.selectedWireId === wireId;
        return (
          <Show when={start() && end()}>
            <g
              data-wire-id={wireId}
              data-wire-color={wire().color}
              data-from-instance={wire().a.instance_id}
              data-from-pin={wire().a.pin_id}
              data-to-instance={wire().b.instance_id}
              data-to-pin={wire().b.pin_id}
              role="button"
              tabIndex={0}
              aria-label={`Wire ${wireId} (${wire().color}) from ${wire().a.instance_id}.${wire().a.pin_id} to ${wire().b.instance_id}.${wire().b.pin_id}`}
              aria-pressed={isSelected()}
              class="canvas-wire"
              classList={{ "is-selected": isSelected() }}
              onPointerDown={(event) => {
                if (event.button !== 0) return;
                event.preventDefault();
                event.stopPropagation();
                props.onSelectWire(wireId);
                (event.currentTarget as SVGGElement).focus();
              }}
              onClick={(event) => {
                event.stopPropagation();
                props.onSelectWire(wireId);
              }}
              onFocus={() => props.onSelectWire(wireId)}
              onKeyDown={(event) => wireKey(event, wireId)}
            >
              <path
                d={manhattanPath(start()!, end()!)}
                fill="none"
                stroke="transparent"
                stroke-width="10"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                data-wire-path
                d={manhattanPath(start()!, end()!)}
                fill="none"
                stroke={wireHex(wire().color)}
                stroke-width={isSelected() ? "3.5" : "2.4"}
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <circle cx={start()!.x} cy={start()!.y} r="2.2" fill={wireHex(wire().color)} />
              <circle cx={end()!.x} cy={end()!.y} r="2.2" fill={wireHex(wire().color)} />
            </g>
          </Show>
        );
      }}</For>
      <Show when={props.preview}>{(preview) => (
        <path
          data-wire-preview
          d={manhattanPath(preview().start, preview().end)}
          fill="none"
          stroke={wireHex(preview().color)}
          stroke-width="2"
          stroke-dasharray="4 3"
          stroke-linecap="round"
          stroke-linejoin="round"
          pointer-events="none"
        />
      )}</Show>
    </g>
  );
}
