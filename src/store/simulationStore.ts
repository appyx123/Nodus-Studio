import { invoke, isTauri } from "@tauri-apps/api/core";
import { createSignal } from "solid-js";
import { decodeFrame, type RunState, type SimulationFrame } from "./frameDecoder";

export { decodeFrame } from "./frameDecoder";
export type { RunState, SimulationFrame } from "./frameDecoder";

export type SimulationAction = "start" | "pause" | "resume" | "stop" | "reset";

export interface InstanceDiagnostic {
  instance_id: string;
  component_id: string;
  callback: string;
  message: string;
}

export interface SimulationStatus {
  state: RunState;
  sim_time_ns: number;
  diagnostics: InstanceDiagnostic[];
}

const FRAME_INTERVAL_MS = 1000 / 60;

const [frame, setFrame] = createSignal<SimulationFrame | null>(null);
const [runState, setRunState] = createSignal<RunState>("stopped");
const [diagnostics, setDiagnostics] = createSignal<InstanceDiagnostic[]>([]);

export const simulationFrame = frame;
export const simulationState = runState;
export const simulationDiagnostics = diagnostics;

let timer: number | undefined;
let lastSeq = -1n;

export async function pollFrame(): Promise<void> {
  if (!isTauri()) return;
  const raw = await invoke<ArrayBuffer>("get_simulation_frame");
  const next = decodeFrame(raw instanceof ArrayBuffer ? raw : new Uint8Array(raw).buffer);
  if (next.seq !== lastSeq) {
    lastSeq = next.seq;
    setFrame(next);
    setRunState(next.run_state);
  }
}

export function startFrameStream(): () => void {
  if (!isTauri() || timer !== undefined) return stopFrameStream;
  timer = window.setInterval(() => {
    pollFrame().catch(() => undefined);
  }, FRAME_INTERVAL_MS);
  return stopFrameStream;
}

export function stopFrameStream(): void {
  if (timer !== undefined) window.clearInterval(timer);
  timer = undefined;
}

export async function controlSimulation(action: SimulationAction): Promise<SimulationStatus> {
  if (!isTauri()) throw new Error("Simulation requires the Tauri desktop app.");
  const status = await invoke<SimulationStatus>("control_simulation", { action });
  setRunState(status.state);
  setDiagnostics(status.diagnostics);
  if (status.state === "running") {
    startFrameStream();
  } else {
    stopFrameStream();
  }
  await pollFrame().catch(() => undefined);
  if (status.state === "stopped") setFrame(null);
  return status;
}

export async function setControlValue(
  instanceId: string,
  controlId: string,
  value: number,
): Promise<SimulationStatus> {
  if (!isTauri()) throw new Error("Simulation requires the Tauri desktop app.");
  const status = await invoke<SimulationStatus>("set_control_value", {
    instance_id: instanceId,
    control_id: controlId,
    value,
  });
  setDiagnostics(status.diagnostics);
  await pollFrame().catch(() => undefined);
  return status;
}
