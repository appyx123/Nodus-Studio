import type { Endpoint } from "../../types/nodpkg";
import {
  dragPosition,
  fitViewToNodes,
  GRID,
  INITIAL_VIEWPORT,
  snapPoint,
  zoomAt,
} from "./geometry";
import type { Point, Viewport } from "./geometry";
import type { CanvasNode } from "./NodeRenderer";

export type Gesture = {
  pointerId: number;
  start: Point;
  viewport: Viewport;
  node?: { id: string; position: Point };
};

export interface GestureContext {
  getStage: () => SVGSVGElement | undefined;
  viewport: () => Viewport;
  setViewport: (v: Viewport | ((prev: Viewport) => Viewport)) => void;
  nodes: () => CanvasNode[];
  setNodes: (setter: (items: CanvasNode[]) => CanvasNode[]) => void;
  selected: () => string | undefined;
  setSelected: (id: string | undefined) => void;
  selectedWire: () => string | undefined;
  setSelectedWire: (id: string | undefined) => void;
  pendingPin: () => Endpoint | undefined;
  setPendingPin: (ep: Endpoint | undefined) => void;
  setCursorWorld: (p: Point | undefined) => void;
  setMessage: (msg: string) => void;
  onUndo: () => void;
  onRedo: () => void;
  onRotate: () => void;
  onDeleteSelected: () => void;
}

export function createWorkbenchGestures(ctx: GestureContext) {
  let gesture: Gesture | undefined;

  function localPoint(event: { clientX: number; clientY: number }): Point {
    const stage = ctx.getStage();
    if (!stage) return { x: 0, y: 0 };
    const bounds = stage.getBoundingClientRect();
    return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  }

  function moveNode(id: string, position: Point) {
    ctx.setNodes((items) =>
      items.map((node) => (node.instanceId === id ? { ...node, position } : node))
    );
  }

  function startGesture(event: PointerEvent, node?: CanvasNode) {
    const stage = ctx.getStage();
    if (!stage || gesture || (event.button !== 0 && event.button !== 1)) return;
    event.preventDefault();
    event.stopPropagation();
    const movingNode = node && event.button === 0 ? node : undefined;
    gesture = {
      pointerId: event.pointerId,
      start: localPoint(event),
      viewport: { ...ctx.viewport() },
      node: movingNode ? { id: movingNode.instanceId, position: { ...movingNode.position } } : undefined,
    };
    if (movingNode) {
      ctx.setSelectedWire(undefined);
      ctx.setSelected(movingNode.instanceId);
      (event.currentTarget as SVGGElement).focus();
    } else {
      if (event.button === 0) {
        ctx.setSelected(undefined);
        ctx.setSelectedWire(undefined);
        ctx.setPendingPin(undefined);
      }
      stage.focus();
    }
    if (event.isTrusted) stage.setPointerCapture(event.pointerId);
  }

  function moveGesture(event: PointerEvent) {
    const point = localPoint(event);
    if (ctx.pendingPin()) {
      const v = ctx.viewport();
      ctx.setCursorWorld({ x: (point.x - v.x) / v.zoom, y: (point.y - v.y) / v.zoom });
    }
    if (!gesture || event.pointerId !== gesture.pointerId) return;
    if (gesture.node) {
      moveNode(gesture.node.id, dragPosition(gesture.node.position, gesture.start, point, gesture.viewport));
    } else {
      ctx.setViewport({
        ...gesture.viewport,
        x: gesture.viewport.x + point.x - gesture.start.x,
        y: gesture.viewport.y + point.y - gesture.start.y,
      });
    }
  }

  function endGesture(event: PointerEvent, cancel = false) {
    const stage = ctx.getStage();
    if (!gesture || event.pointerId !== gesture.pointerId) return;
    if (cancel) {
      if (gesture.node) moveNode(gesture.node.id, gesture.node.position);
      else ctx.setViewport(gesture.viewport);
    } else moveGesture(event);
    gesture = undefined;
    if (stage && stage.hasPointerCapture(event.pointerId)) {
      stage.releasePointerCapture(event.pointerId);
    }
  }

  function changeZoom(factor: number, point?: Point) {
    if (gesture) return;
    const stage = ctx.getStage();
    if (!stage) return;
    const bounds = stage.getBoundingClientRect();
    ctx.setViewport((view) =>
      zoomAt(view, point ?? { x: bounds.width / 2, y: bounds.height / 2 }, view.zoom * factor)
    );
  }

  function fitToView() {
    const stage = ctx.getStage();
    if (!stage) return;
    const bounds = stage.getBoundingClientRect();
    const next = fitViewToNodes(
      ctx.nodes().map((n) => n.position),
      bounds.width || 800,
      bounds.height || 600
    );
    ctx.setViewport(next);
    ctx.setMessage("Fitted view to components.");
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault();
    const stage = ctx.getStage();
    if (!stage) return;
    const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? stage.clientHeight : 1;
    changeZoom(Math.exp(-Math.max(-300, Math.min(300, event.deltaY * unit)) * 0.002), localPoint(event));
  }

  function stageKey(event: KeyboardEvent) {
    if (event.ctrlKey && (event.key === "z" || event.key === "Z")) {
      event.preventDefault();
      ctx.onUndo();
    } else if (event.ctrlKey && (event.key === "y" || event.key === "Y")) {
      event.preventDefault();
      ctx.onRedo();
    } else if ((event.key === "r" || event.key === "R") && ctx.selected()) {
      event.preventDefault();
      ctx.onRotate();
    } else if (event.key === "Escape") {
      event.preventDefault();
      if (ctx.pendingPin()) {
        ctx.setPendingPin(undefined);
        ctx.setMessage("Wiring cancelled.");
      } else if (ctx.selected() || ctx.selectedWire()) {
        ctx.setSelected(undefined);
        ctx.setSelectedWire(undefined);
      }
    } else if ((event.key === "Delete" || event.key === "Backspace") && (ctx.selected() || ctx.selectedWire())) {
      event.preventDefault();
      ctx.onDeleteSelected();
    }
  }

  function nodeKey(event: KeyboardEvent, node: CanvasNode) {
    if (gesture) return;
    const directions: Record<string, Point> = {
      ArrowLeft: { x: -GRID, y: 0 },
      ArrowRight: { x: GRID, y: 0 },
      ArrowUp: { x: 0, y: -GRID },
      ArrowDown: { x: 0, y: GRID },
    };
    if (directions[event.key]) {
      event.preventDefault();
      event.stopPropagation();
      const step = directions[event.key];
      ctx.setSelectedWire(undefined);
      ctx.setSelected(node.instanceId);
      moveNode(node.instanceId, snapPoint({ x: node.position.x + step.x, y: node.position.y + step.y }));
    } else if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      event.stopPropagation();
      ctx.setSelectedWire(undefined);
      ctx.setSelected(node.instanceId);
      ctx.onDeleteSelected();
    }
  }

  function exportSchematic() {
    const stage = ctx.getStage();
    if (!stage) return;
    const svgData = new XMLSerializer().serializeToString(stage);
    const blob = new Blob([svgData], { type: "image/svg+xml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "nodus-schematic.svg";
    a.click();
    URL.revokeObjectURL(url);
    ctx.setMessage("Schematic exported as SVG.");
  }

  return {
    localPoint,
    moveNode,
    startGesture,
    moveGesture,
    endGesture,
    changeZoom,
    fitToView,
    onWheel,
    stageKey,
    nodeKey,
    exportSchematic,
    hasActiveGesture: () => Boolean(gesture),
  };
}
