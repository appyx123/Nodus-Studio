import { createMemo, For, Show } from "solid-js";
import { setControlValue, simulationFrame, simulationState } from "../../store/simulationStore";
import type { CanvasNode } from "./NodeRenderer";

interface Props {
  node: CanvasNode | undefined;
  onClose: () => void;
  onRotate: () => void;
  onDelete: () => void;
  onUpdateLabel?: (label: string) => void;
  onError: (msg: string) => void;
}

export default function ComponentInspector(props: Props) {
  const node = () => props.node;
  const pkg = createMemo(() => node()?.pkg);
  const frame = simulationFrame;
  const state = simulationState;

  const instanceState = createMemo(() => {
    const id = node()?.instanceId;
    return id ? frame()?.instances.find((i) => i.instance_id === id) : undefined;
  });

  async function updateControl(controlId: string, value: number) {
    const n = node();
    if (!n) return;
    try {
      await setControlValue(n.instanceId, controlId, value);
      props.onError("");
    } catch (err) {
      props.onError(String(err));
    }
  }

  return (
    <Show when={node() && pkg()}>
      <aside
        aria-label="Component properties inspector"
        class="absolute left-6 top-6 z-20 w-80 rounded-xl border border-zinc-700/80 bg-zinc-900/95 p-4 shadow-2xl backdrop-blur-md transition-all"
      >
        <div class="flex items-start justify-between gap-3 border-b border-zinc-800 pb-3">
          <div class="flex items-center gap-3">
            <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-950 p-1">
              <img
                src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(pkg()!.svg)}`}
                alt={pkg()!.manifest.component.name}
                class="h-full w-full object-contain"
              />
            </div>
            <div>
              <h3 class="text-sm font-semibold text-zinc-100">
                {node()!.label || pkg()!.manifest.component.name}
              </h3>
              <p class="font-mono text-[10px] text-emerald-400">
                {pkg()!.manifest.component.id} · v{pkg()!.manifest.component.version}
              </p>
            </div>
          </div>
          <button
            type="button"
            class="rounded p-1 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
            aria-label="Close inspector"
            title="Deselect (Escape)"
            onClick={() => props.onClose()}
          >
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div class="mt-3 flex items-center justify-between gap-2 border-b border-zinc-800 pb-3 text-xs">
          <span class="text-zinc-400 font-mono text-[11px] truncate max-w-[140px]">
            {node()!.instanceId}
          </span>
          <div class="flex items-center gap-1.5">
            <button
              type="button"
              class="flex items-center gap-1 rounded border border-zinc-700 bg-zinc-800 px-2 py-1 text-[11px] text-zinc-300 hover:border-emerald-400 hover:text-emerald-400"
              title="Rotate 90 degrees (R)"
              onClick={() => props.onRotate()}
            >
              <svg class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.83 6.72 2.24L21 8" />
                <path d="M21 3v5h-5" />
              </svg>
              <span>{node()!.rotation ?? 0}°</span>
            </button>
            <button
              type="button"
              class="flex items-center gap-1 rounded border border-red-900/60 bg-red-950/40 px-2 py-1 text-[11px] text-red-300 hover:border-red-500 hover:bg-red-900/50"
              title="Delete component (Delete/Backspace)"
              onClick={() => props.onDelete()}
            >
              <svg class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6" />
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              </svg>
            </button>
          </div>
        </div>

        <Show when={pkg()!.manifest.controls.length > 0}>
          <div class="mt-3 space-y-3 border-b border-zinc-800 pb-3">
            <p class="text-[11px] font-semibold uppercase tracking-wider text-zinc-400">
              Interactive Controls
            </p>
            <For each={pkg()!.manifest.controls}>
              {(ctrl) => (
                <div class="space-y-1">
                  <div class="flex items-center justify-between text-xs text-zinc-300">
                    <span>{ctrl.label}</span>
                    <span class="font-mono text-emerald-400">
                      {ctrl.step && ctrl.step < 1 ? ctrl.default.toFixed(1) : ctrl.default}
                    </span>
                  </div>
                  <input
                    type="range"
                    min={ctrl.min}
                    max={ctrl.max}
                    step={ctrl.step}
                    value={ctrl.default}
                    class="w-full accent-emerald-500 h-1.5 bg-zinc-800 rounded-lg cursor-pointer"
                    aria-label={`${ctrl.label} on ${node()!.instanceId}`}
                    onInput={(e) => {
                      void updateControl(ctrl.id, Number(e.currentTarget.value));
                    }}
                  />
                  <div class="flex justify-between text-[10px] text-zinc-500 font-mono">
                    <span>{ctrl.min}</span>
                    <span>{ctrl.max}</span>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>

        <div class="mt-3">
          <p class="text-[11px] font-semibold uppercase tracking-wider text-zinc-400">
            Pins ({pkg()!.manifest.pins.length})
          </p>
          <div class="mt-2 max-h-36 overflow-y-auto space-y-1 pr-1 font-mono text-[11px]">
            <For each={pkg()!.manifest.pins}>
              {(pin) => (
                <div class="flex items-center justify-between rounded bg-zinc-950/60 px-2 py-1">
                  <span class="text-zinc-300 font-medium">{pin.id}</span>
                  <div class="flex items-center gap-2">
                    <span class="text-[10px] text-zinc-500">{pin.type}</span>
                    <span class="rounded px-1.5 py-0.2 text-[9px] font-semibold bg-zinc-800 text-zinc-400">
                      {instanceState() ? "LIVE" : "READY"}
                    </span>
                  </div>
                </div>
              )}
            </For>
          </div>
        </div>
      </aside>
    </Show>
  );
}
