import { createEffect, createMemo, onCleanup } from "solid-js";
import type { ComponentPackage, Endpoint } from "../../types/nodpkg";
import type { Point } from "./geometry";
import { instanceGraphic } from "./graphic";

export interface CanvasNode {
  instanceId: string;
  pkg: ComponentPackage;
  position: Point;
  viewBoxOrigin: Point;
  pinPositions: Map<string, Point>;
}

interface Props {
  node: CanvasNode;
  selected: boolean;
  activeEndpoint: Endpoint | undefined;
  onPointerDown: (event: PointerEvent, node: CanvasNode) => void;
  onSelect: (id: string) => void;
  onKeyDown: (event: KeyboardEvent, node: CanvasNode) => void;
  onPinActivate: (endpoint: Endpoint) => void;
}

export default function NodeRenderer(props: Props) {
  let host!: SVGGElement;
  const pkg = createMemo(() => props.node.pkg);
  const instanceId = createMemo(() => props.node.instanceId);
  createEffect(() => {
    const graphic = instanceGraphic(pkg().svg, instanceId(), pkg().manifest.pins);
    host.replaceChildren(graphic.element);
    onCleanup(() => graphic.element.remove());
  });
  createEffect(() => {
    const active = props.activeEndpoint;
    const id = instanceId();
    for (const pinEl of host.querySelectorAll("[data-pin-id]")) {
      const pinId = pinEl.getAttribute("data-pin-id");
      const isPending = Boolean(active && active.instance_id === id && active.pin_id === pinId);
      pinEl.classList.toggle("is-active-pin", isPending);
      pinEl.setAttribute("aria-pressed", String(isPending));
    }
  });

  function targetPin(target: EventTarget | null): string | undefined {
    const el = target instanceof Element ? target.closest("[data-pin-id]") : null;
    return el?.getAttribute("data-pin-id") ?? undefined;
  }

  return (
    <g data-instance-id={props.node.instanceId} data-x={props.node.position.x} data-y={props.node.position.y}
      transform={`translate(${props.node.position.x} ${props.node.position.y})`}
      role="button" tabIndex={0} aria-label={`${props.node.pkg.manifest.component.name} instance ${props.node.instanceId}`}
      aria-pressed={props.selected} class="canvas-node" classList={{ "is-selected": props.selected }}
      onPointerDown={(event) => {
        if (targetPin(event.target)) {
          event.stopPropagation();
          return;
        }
        props.onPointerDown(event, props.node);
      }}
      onClick={(event) => {
        const pinId = targetPin(event.target);
        if (pinId) {
          event.stopPropagation();
          props.onPinActivate({ instance_id: props.node.instanceId, pin_id: pinId });
          return;
        }
        props.onSelect(props.node.instanceId);
      }}
      onFocus={(event) => {
        if (!targetPin(event.target)) props.onSelect(props.node.instanceId);
      }}
      onKeyDown={(event) => {
        const pinId = targetPin(event.target);
        if (pinId && (event.key === "Enter" || event.key === " ")) {
          event.preventDefault();
          event.stopPropagation();
          props.onPinActivate({ instance_id: props.node.instanceId, pin_id: pinId });
          return;
        }
        props.onKeyDown(event, props.node);
      }}>
      <g ref={host} />
    </g>
  );
}
