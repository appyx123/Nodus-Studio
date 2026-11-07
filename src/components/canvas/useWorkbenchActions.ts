import { editCircuit, sameEndpoint } from "../../store/circuitStore";
import type { CircuitEdit, CircuitSnapshot, ComponentPackage, Endpoint, WireColor } from "../../types/nodpkg";
import { GRID, MAX_INSTANCES, pinWorldPoint, screenToWorld, snapPoint } from "./geometry";
import type { Point, Viewport } from "./geometry";
import { instanceGraphic } from "./graphic";
import type { CanvasNode } from "./NodeRenderer";

export interface ActionsContext {
  viewport: () => Viewport;
  nodes: () => CanvasNode[];
  setNodes: (setter: (items: CanvasNode[]) => CanvasNode[]) => void;
  circuit: () => CircuitSnapshot;
  setCircuit: (snap: CircuitSnapshot) => void;
  selected: () => string | undefined;
  setSelected: (id: string | undefined) => void;
  selectedWire: () => string | undefined;
  setSelectedWire: (id: string | undefined) => void;
  wireColor: () => WireColor;
  setWireColor: (c: WireColor) => void;
  pendingPin: () => Endpoint | undefined;
  setPendingPin: (ep: Endpoint | undefined) => void;
  setCursorWorld: (p: Point | undefined) => void;
  setMessage: (msg: string) => void;
  setErrorMessage: (msg: string) => void;
  pushHistory: () => void;
  hasActiveGesture: () => boolean;
  focusStage: () => void;
}

export function createWorkbenchActions(ctx: ActionsContext) {
  function endpointPosition(endpoint: Endpoint): Point | undefined {
    const node = ctx.nodes().find((item) => item.instanceId === endpoint.instance_id);
    const localPin = node?.pinPositions.get(endpoint.pin_id);
    return node && localPin
      ? pinWorldPoint(node.position, localPin, node.viewBoxOrigin, node.rotation ?? 0)
      : undefined;
  }

  async function applyEdit(edit: CircuitEdit): Promise<CircuitSnapshot | undefined> {
    try {
      const snapshot = await editCircuit(edit);
      ctx.setCircuit(snapshot);
      ctx.setErrorMessage("");
      return snapshot;
    } catch (error) {
      const text = String(error);
      ctx.setErrorMessage(text);
      ctx.setMessage(text);
      return undefined;
    }
  }

  async function place(pkg: ComponentPackage, point?: Point) {
    if (ctx.nodes().length >= MAX_INSTANCES) {
      const text = "Canvas limit reached (256 instances). Delete an instance before adding another.";
      ctx.setErrorMessage(text);
      ctx.setMessage(text);
      return;
    }
    ctx.pushHistory();
    const center = screenToWorld({ x: 400, y: 300 }, ctx.viewport());
    const offset = (ctx.nodes().length % 6) * GRID;
    const position = snapPoint(point ?? { x: center.x + offset, y: center.y + offset });
    const instanceId = `node-${crypto.randomUUID()}`;
    const graphic = instanceGraphic(pkg.svg, instanceId, pkg.manifest.pins);
    const snapshot = await applyEdit({
      type: "add_instance",
      instance_id: instanceId,
      component_id: pkg.manifest.component.id,
      component_version: pkg.manifest.component.version,
    });
    if (!snapshot) return;
    ctx.setNodes((items) => [
      ...items,
      {
        instanceId,
        pkg,
        position,
        viewBoxOrigin: graphic.viewBoxOrigin,
        pinPositions: graphic.pinPositions,
        rotation: 0,
      },
    ]);
    ctx.setSelectedWire(undefined);
    ctx.setSelected(instanceId);
    ctx.setMessage(`${pkg.manifest.component.name} added.`);
  }

  async function activatePin(endpoint: Endpoint) {
    if (ctx.hasActiveGesture()) return;
    const start = ctx.pendingPin();
    if (!start) {
      ctx.setPendingPin(endpoint);
      ctx.setCursorWorld(endpointPosition(endpoint));
      ctx.setSelectedWire(undefined);
      ctx.setErrorMessage("");
      ctx.setMessage(`Wiring from ${endpoint.pin_id}. Click target pin or press Escape.`);
      return;
    }
    if (sameEndpoint(start, endpoint)) {
      ctx.setPendingPin(undefined);
      ctx.setMessage("Wiring cancelled.");
      return;
    }
    ctx.pushHistory();
    const wireId = `wire-${crypto.randomUUID()}`;
    const snapshot = await applyEdit({
      type: "connect",
      wire_id: wireId,
      a: start,
      b: endpoint,
      color: ctx.wireColor(),
    });
    ctx.setPendingPin(undefined);
    if (!snapshot) return;
    ctx.setSelectedWire(wireId);
    ctx.setMessage(`Connected ${start.pin_id} to ${endpoint.pin_id}.`);
  }

  async function chooseColor(color: WireColor) {
    ctx.setWireColor(color);
    const wireId = ctx.selectedWire();
    if (wireId) {
      ctx.pushHistory();
      if (await applyEdit({ type: "set_wire_color", wire_id: wireId, color })) {
        ctx.setMessage(`Wire color changed to ${color}.`);
      }
    }
  }

  async function removeWireById(wireId: string) {
    if (ctx.hasActiveGesture()) return;
    ctx.pushHistory();
    if (!(await applyEdit({ type: "remove_wire", wire_id: wireId }))) return;
    if (ctx.selectedWire() === wireId) ctx.setSelectedWire(undefined);
    ctx.setMessage("Selected wire deleted.");
    ctx.focusStage();
  }

  async function removeSelected() {
    if (ctx.hasActiveGesture()) return;
    const wireId = ctx.selectedWire();
    if (wireId) return removeWireById(wireId);
    const nodeId = ctx.selected();
    if (!nodeId) return;
    ctx.pushHistory();
    if (!(await applyEdit({ type: "remove_instance", instance_id: nodeId }))) return;
    ctx.setNodes((items) => items.filter((node) => node.instanceId !== nodeId));
    ctx.setSelected(undefined);
    if (ctx.pendingPin()?.instance_id === nodeId) ctx.setPendingPin(undefined);
    ctx.setMessage("Selected component deleted.");
    ctx.focusStage();
  }

  return {
    endpointPosition,
    applyEdit,
    place,
    activatePin,
    chooseColor,
    removeWireById,
    removeSelected,
  };
}
