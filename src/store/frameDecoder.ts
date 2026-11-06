export const FRAME_PROTOCOL_VERSION = 1;
const HEADER_BYTES = 32;
const RUN_STATES = ["stopped", "running", "paused"] as const;
const LEVELS = ["LOW", "HIGH", "HIGH_Z", "UNKNOWN"] as const;

export type RunState = (typeof RUN_STATES)[number];
export type FrameLevel = (typeof LEVELS)[number];

export interface FrameInstance {
  instance_id: string;
  faulted: boolean;
  visuals: Record<string, number>;
}

export interface FrameNet {
  state: FrameLevel;
  conflict: boolean;
  endpoints: { instance_id: string; pin_id: string }[];
}

export interface SimulationFrame {
  seq: bigint;
  run_state: RunState;
  has_conflict: boolean;
  has_fault: boolean;
  sim_time_ns: number;
  speed_ratio: number;
  instances: FrameInstance[];
  nets: FrameNet[];
}

export function decodeFrame(buffer: ArrayBuffer): SimulationFrame {
  if (buffer.byteLength < HEADER_BYTES) throw new Error("Frame shorter than header");
  const view = new DataView(buffer);
  const bytes = new Uint8Array(buffer);
  if (String.fromCharCode(...bytes.subarray(0, 4)) !== "NDFR") throw new Error("Invalid frame magic");
  if (view.getUint8(4) !== FRAME_PROTOCOL_VERSION) throw new Error("Unsupported frame version");
  const run_state = RUN_STATES[view.getUint8(5)];
  if (!run_state) throw new Error("Invalid run state");
  const flags = view.getUint16(6, true);
  const seq = view.getBigUint64(8, true);
  const halfNs = view.getBigUint64(16, true);
  const speed_ratio = view.getFloat32(24, true);
  const instCount = view.getUint16(28, true);
  const netCount = view.getUint16(30, true);
  let pos = HEADER_BYTES;
  const decoder = new TextDecoder("utf-8", { fatal: true });
  const need = (n: number) => {
    if (pos + n > buffer.byteLength) throw new Error("Truncated frame");
  };
  const u8 = () => (need(1), view.getUint8(pos++));
  const u16 = () => (need(2), ((pos += 2), view.getUint16(pos - 2, true)));
  const f64 = () => (need(8), ((pos += 8), view.getFloat64(pos - 8, true)));
  const str = () => {
    const len = u16();
    if (len === 0) throw new Error("Empty identifier");
    need(len);
    pos += len;
    return decoder.decode(bytes.subarray(pos - len, pos));
  };
  const flag = () => {
    const v = u8();
    if (v > 1) throw new Error("Invalid boolean flag");
    return v === 1;
  };

  const instances: FrameInstance[] = [];
  for (let i = 0; i < instCount; i++) {
    const instance_id = str();
    const faulted = flag();
    const visuals: Record<string, number> = {};
    for (let v = u16(); v > 0; v--) visuals[str()] = f64();
    instances.push({ instance_id, faulted, visuals });
  }
  const nets: FrameNet[] = [];
  for (let i = 0; i < netCount; i++) {
    const state = LEVELS[u8()];
    if (!state) throw new Error("Invalid logic level");
    const conflict = flag();
    const endpoints: FrameNet["endpoints"] = [];
    for (let e = u16(); e > 0; e--) endpoints.push({ instance_id: str(), pin_id: str() });
    nets.push({ state, conflict, endpoints });
  }
  if (pos !== buffer.byteLength) throw new Error("Trailing bytes in frame");
  return {
    seq,
    run_state,
    has_conflict: (flags & 1) !== 0,
    has_fault: (flags & 2) !== 0,
    sim_time_ns: Number(halfNs) / 2,
    speed_ratio,
    instances,
    nets,
  };
}
