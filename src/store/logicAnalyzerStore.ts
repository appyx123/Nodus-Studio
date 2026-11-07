import { invoke, isTauri } from "@tauri-apps/api/core";
import { createSignal } from "solid-js";
import type { Endpoint } from "../types/nodpkg";

export interface LogicSample {
  time_half_ns: number;
  level: number; // 0 = Low, 1 = High, 2 = HighZ, 3 = Unknown
}

export interface ChannelSnapshot {
  channel_id: number;
  name: string;
  endpoint?: Endpoint;
  samples: LogicSample[];
}

export interface LogicAnalyzerSnapshot {
  current_sim_time_half_ns: number;
  channels: ChannelSnapshot[];
}

export interface AttachProbeRequest {
  channel_id: number;
  name: string;
  endpoint: Endpoint;
}

const [analyzerOpen, setAnalyzerOpen] = createSignal(false);
const [analyzerSnapshot, setAnalyzerSnapshot] = createSignal<LogicAnalyzerSnapshot>({
  current_sim_time_half_ns: 0,
  channels: [],
});

export { analyzerOpen, setAnalyzerOpen, analyzerSnapshot, setAnalyzerSnapshot };

export async function fetchLogicAnalyzerData(): Promise<LogicAnalyzerSnapshot | undefined> {
  if (!isTauri()) return undefined;
  try {
    const snap = await invoke<LogicAnalyzerSnapshot>("get_logic_analyzer_data");
    setAnalyzerSnapshot(snap);
    return snap;
  } catch {
    return undefined;
  }
}

export async function attachLogicProbe(request: AttachProbeRequest): Promise<void> {
  if (!isTauri()) return;
  await invoke("attach_logic_probe", { request });
  await fetchLogicAnalyzerData();
}

export async function detachLogicProbe(channelId: number): Promise<void> {
  if (!isTauri()) return;
  await invoke("detach_logic_probe", { channelId });
  await fetchLogicAnalyzerData();
}
