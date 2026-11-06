import { createEffect, createMemo, onCleanup } from "solid-js";
import type { ComponentPackage } from "../../types/nodpkg";
import type { Point } from "./geometry";
import { instanceGraphic } from "./graphic";

export interface CanvasNode {
  instanceId: string;
  pkg: ComponentPackage;
  position: Point;
}

interface Props {
  node: CanvasNode;
  selected: boolean;
  onPointerDown: (event: PointerEvent, node: CanvasNode) => void;
  onSelect: (id: string) => void;
  onKeyDown: (event: KeyboardEvent, node: CanvasNode) => void;
}

export default function NodeRenderer(props: Props) {
  let host!: SVGGElement;
  const pkg = createMemo(() => props.node.pkg);
  const instanceId = createMemo(() => props.node.instanceId);
  createEffect(() => {
    const graphic = instanceGraphic(pkg().svg, instanceId());
    host.replaceChildren(graphic.element);
    onCleanup(() => graphic.element.remove());
  });

  return (
    <g data-instance-id={props.node.instanceId} data-x={props.node.position.x} data-y={props.node.position.y}
      transform={`translate(${props.node.position.x} ${props.node.position.y})`}
      role="button" tabIndex={0} aria-label={`${props.node.pkg.manifest.component.name} instance ${props.node.instanceId}`}
      aria-pressed={props.selected} class="canvas-node" classList={{ "is-selected": props.selected }}
      onPointerDown={(event) => props.onPointerDown(event, props.node)}
      onClick={() => props.onSelect(props.node.instanceId)}
      onFocus={() => props.onSelect(props.node.instanceId)}
      onKeyDown={(event) => props.onKeyDown(event, props.node)}>
      <g ref={host} />
    </g>
  );
}
