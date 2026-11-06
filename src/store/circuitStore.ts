import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CircuitEdit, CircuitSnapshot, Endpoint, Net, WireColor } from "../types/nodpkg";

export const WIRE_COLORS: readonly { value: WireColor; label: string; hex: string }[] = [
  { value: "green", label: "Green", hex: "#22c55e" },
  { value: "blue", label: "Blue", hex: "#3b82f6" },
  { value: "amber", label: "Amber", hex: "#f59e0b" },
  { value: "red", label: "Red", hex: "#ef4444" },
  { value: "violet", label: "Violet", hex: "#a855f7" },
];

export const EMPTY_CIRCUIT: CircuitSnapshot = {
  instances: [],
  wires: [],
  nets: [],
};

export function wireHex(color: WireColor): string {
  return WIRE_COLORS.find((item) => item.value === color)?.hex ?? "#22c55e";
}

export function sameEndpoint(a: Endpoint, b: Endpoint): boolean {
  return a.instance_id === b.instance_id && a.pin_id === b.pin_id;
}

export function connectedNetCount(nets: Net[]): number {
  return nets.filter((net) => net.endpoints.length > 1).length;
}

export async function getCircuit(): Promise<CircuitSnapshot> {
  if (!isTauri()) {
    return EMPTY_CIRCUIT;
  }
  return invoke<CircuitSnapshot>("get_circuit");
}

export async function editCircuit(edit: CircuitEdit): Promise<CircuitSnapshot> {
  if (!isTauri()) {
    throw new Error("Circuit topology requires the Tauri desktop app.");
  }
  return invoke<CircuitSnapshot>("edit_circuit", { edit });
}
