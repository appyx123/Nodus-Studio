import { For, Show } from "solid-js";
import {
  controlSimulation,
  setControlValue,
  simulationDiagnostics,
  simulationFrame,
  simulationState,
  type SimulationAction,
} from "../../store/simulationStore";
import type { CanvasNode } from "./NodeRenderer";

interface Props {
  selectedNode: CanvasNode | undefined;
  onError: (msg: string) => void;
  onStatus: (msg: string) => void;
}

const btn = "workbench-button";

export default function SimulationBar(props: Props) {
  const state = simulationState;
  const frame = simulationFrame;

  async function act(action: SimulationAction, label: string) {
    try {
      await controlSimulation(action);
      props.onError("");
      props.onStatus(label);
    } catch (error) {
      props.onError(String(error));
    }
  }

  async function changeControl(instanceId: string, controlId: string, value: number) {
    try {
      await setControlValue(instanceId, controlId, value);
      props.onError("");
    } catch (error) {
      props.onError(String(error));
    }
  }

  return (
    <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Simulation controls">
      <button type="button" class={btn} disabled={state() !== "stopped"}
        onClick={() => { void act("start", "Simulation started."); }}>Start</button>
      <button type="button" class={btn} disabled={state() !== "running"}
        onClick={() => { void act("pause", "Simulation paused."); }}>Pause</button>
      <button type="button" class={btn} disabled={state() !== "paused"}
        onClick={() => { void act("resume", "Simulation resumed."); }}>Resume</button>
      <button type="button" class={btn} disabled={state() === "stopped"}
        onClick={() => { void act("stop", "Simulation stopped."); }}>Stop</button>
      <button type="button" class={btn} disabled={state() === "stopped"}
        onClick={() => { void act("reset", "Simulation reset."); }}>Reset</button>
      <output class="font-mono text-xs text-zinc-300" aria-label="Simulation status">
        Sim: {state()}{frame() ? ` · ${(frame()!.sim_time_ns / 1e6).toFixed(1)} ms` : ""}
        {frame()?.has_conflict ? " · CONFLICT" : ""}
      </output>
      <Show when={props.selectedNode && state() !== "stopped" && props.selectedNode.pkg.manifest.controls.length > 0}>
        <For each={props.selectedNode!.pkg.manifest.controls}>{(ctrl) => (
          <label class="flex items-center gap-1 font-mono text-xs text-zinc-200">
            <span>{ctrl.label}:</span>
            <input type="range" min={ctrl.min} max={ctrl.max} step={ctrl.step} value={ctrl.default}
              aria-label={`${ctrl.label} on ${props.selectedNode!.instanceId}`}
              onInput={(e) => {
                void changeControl(props.selectedNode!.instanceId, ctrl.id, Number(e.currentTarget.value));
              }} />
            <button type="button" class={btn}
              aria-label={`Toggle ${ctrl.label} on ${props.selectedNode!.instanceId}`}
              onClick={() => {
                const input = document.querySelector<HTMLInputElement>(
                  `input[aria-label="${ctrl.label} on ${props.selectedNode!.instanceId}"]`,
                );
                const next = input && Number(input.value) >= (ctrl.min + ctrl.max) / 2 ? ctrl.min : ctrl.max;
                if (input) input.value = String(next);
                void changeControl(props.selectedNode!.instanceId, ctrl.id, next);
              }}>{ctrl.label}</button>
          </label>
        )}</For>
      </Show>
      <Show when={simulationDiagnostics().length > 0}>
        <span class="font-mono text-xs text-amber-300" role="status">
          Lua fault: {simulationDiagnostics()[0].component_id} ({simulationDiagnostics()[0].callback})
        </span>
      </Show>
    </div>
  );
}
