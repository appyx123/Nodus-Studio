import { getCircuit } from "../../store/circuitStore";
import { setSketchSource } from "../../store/editorStore";
import { importProjectJson, projectName, saveProject } from "../../store/projectStore";
import type { CircuitSnapshot, ComponentPackage } from "../../types/nodpkg";
import type { NodusProjectFile } from "../../types/project";
import type { CanvasNode } from "./NodeRenderer";
import type { Viewport } from "./geometry";
import { instanceGraphic } from "./graphic";

export async function saveWorkbenchProject(
  nodes: CanvasNode[],
  circuit: CircuitSnapshot,
  viewport: Viewport,
  sketchSource: string
): Promise<string> {
  const filename = prompt("Enter project file name or path:", `${projectName()}.nodus`);
  if (!filename) return "";

  const project: NodusProjectFile = {
    format: "nodus-project",
    version: "1.0.0",
    app_id: "org.nodus.Studio",
    metadata: {
      name: projectName(),
      created_at: Date.now(),
      modified_at: Date.now(),
    },
    viewport: {
      pan: { x: viewport.x, y: viewport.y },
      zoom: viewport.zoom,
    },
    circuit: {
      instances: nodes.map((n) => ({
        instance_id: n.instanceId,
        component_id: n.pkg.manifest.component.id,
        component_version: n.pkg.manifest.component.version,
        position: { x: n.position.x, y: n.position.y },
        control_values: {},
      })),
      wires: circuit.wires.map((w) => ({
        wire_id: w.wire_id,
        a: { instance_id: w.a.instance_id, pin_id: w.a.pin_id },
        b: { instance_id: w.b.instance_id, pin_id: w.b.pin_id },
        color: w.color,
      })),
    },
    firmware: {
      sketch_source: sketchSource,
      fqbn: "arduino:avr:uno",
    },
  };

  await saveProject(filename, project);
  return filename;
}

export interface RestoredProjectResult {
  nodes: CanvasNode[];
  circuit: CircuitSnapshot;
  viewport: Viewport;
  projectName: string;
  warnings: string[];
}

export async function openWorkbenchProject(
  fileText: string,
  availablePackages: ComponentPackage[]
): Promise<RestoredProjectResult> {
  const response = await importProjectJson(fileText);
  const proj = response.project;

  if (proj.firmware) {
    setSketchSource(proj.firmware.sketch_source);
  }

  const restoredNodes: CanvasNode[] = [];
  for (const inst of proj.circuit.instances) {
    const pkg = availablePackages.find((p) => p.manifest.component.id === inst.component_id);
    if (pkg) {
      const graphic = instanceGraphic(pkg.svg, inst.instance_id, pkg.manifest.pins);
      restoredNodes.push({
        instanceId: inst.instance_id,
        pkg,
        position: { x: inst.position.x, y: inst.position.y },
        viewBoxOrigin: graphic.viewBoxOrigin,
        pinPositions: graphic.pinPositions,
      });
    }
  }

  const snap = await getCircuit();

  return {
    nodes: restoredNodes,
    circuit: snap,
    viewport: {
      x: proj.viewport.pan.x,
      y: proj.viewport.pan.y,
      zoom: proj.viewport.zoom,
    },
    projectName: proj.metadata.name,
    warnings: response.warnings.map((w) => w.message),
  };
}

export async function handleSaveAction(
  nodes: CanvasNode[],
  circuit: CircuitSnapshot,
  viewport: Viewport,
  sketchSource: string,
  onStatus: (msg: string) => void,
  onError: (err: string) => void
): Promise<void> {
  try {
    const file = await saveWorkbenchProject(nodes, circuit, viewport, sketchSource);
    if (file) {
      onStatus(`Project saved to ${file}`);
      onError("");
    }
  } catch (err) {
    const msg = String(err);
    onError(msg);
    onStatus(msg);
  }
}

export async function handleOpenAction(
  event: Event,
  packages: ComponentPackage[],
  onRestore: (res: RestoredProjectResult) => void,
  onStatus: (msg: string) => void,
  onError: (err: string) => void
): Promise<void> {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0];
  if (!file) return;
  try {
    const text = await file.text();
    const res = await openWorkbenchProject(text, packages);
    onRestore(res);
    if (res.warnings.length > 0) {
      onStatus(`Project loaded with warnings: ${res.warnings.join("; ")}`);
    } else {
      onStatus(`Project '${res.projectName}' loaded.`);
    }
    onError("");
  } catch (err) {
    const msg = String(err);
    onError(msg);
    onStatus(msg);
  } finally {
    target.value = "";
  }
}
