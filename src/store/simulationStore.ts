import { invoke, isTauri } from "@tauri-apps/api/core";
import { createSignal } from "solid-js";
import { decodeFrame, type SimulationFrame } from "./frameDecoder";

export { decodeFrame } from "./frameDecoder";
export type { SimulationFrame } from "./frameDecoder";

const FRAME_INTERVAL_MS = 1000 / 60;

const [frame, setFrame] = createSignal<SimulationFrame | null>(null);
export const simulationFrame = frame;

let timer: number | undefined;
let lastSeq = -1n;

async function pollFrame(): Promise<void> {
  const raw = await invoke<ArrayBuffer>("get_simulation_frame");
  const next = decodeFrame(raw);
  if (next.seq !== lastSeq) {
    lastSeq = next.seq;
    setFrame(next);
  }
}

/** Starts a 60 Hz poll of the Rust binary frame. Visual state only; no simulation logic. */
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
