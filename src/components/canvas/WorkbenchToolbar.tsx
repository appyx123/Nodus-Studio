import { For, Show } from "solid-js";
import { WIRE_COLORS } from "../../store/circuitStore";
import { projectName, setProjectName } from "../../store/projectStore";
import { simulationState } from "../../store/simulationStore";
import type { WireColor } from "../../types/nodpkg";

export interface WorkbenchToolbarProps {
  wireColor: WireColor;
  activeWireColor?: WireColor;
  onChooseColor: (color: WireColor) => void;
  canRotate: boolean;
  onRotateSelected: () => void;
  canDelete: boolean;
  onDeleteSelected: () => void;
  canUndo: boolean;
  onUndo: () => void;
  canRedo: boolean;
  onRedo: () => void;
  editorOpen: boolean;
  onToggleEditor: () => void;
  mqttOpen: boolean;
  onToggleMqtt: () => void;
  analyzerOpen: boolean;
  onToggleAnalyzer: () => void;
  catalogOpen: boolean;
  onToggleCatalog: () => void;
  onStartSimulation: () => void;
  onStopSimulation: () => void;
  onSaveProject: () => void;
  onOpenProject: (event: Event) => void;
  onExportSchematic: () => void;
}

export function WorkbenchToolbar(props: WorkbenchToolbarProps) {
  let fileInputRef: HTMLInputElement | undefined;
  const state = simulationState;

  return (
    <header class="flex h-14 w-full shrink-0 items-center justify-between border-b border-zinc-800 bg-zinc-950 px-4">
      {/* Left: Project identity & Canvas editing tools */}
      <div class="flex items-center gap-3">
        <div class="flex items-center gap-2">
          <span class="flex h-7 w-7 items-center justify-center rounded-lg bg-emerald-500 font-mono text-xs font-black text-black">
            N
          </span>
          <input
            type="text"
            value={projectName()}
            onChange={(e) => setProjectName(e.currentTarget.value || "Untitled Circuit")}
            class="rounded border border-transparent bg-transparent px-1.5 py-0.5 text-xs font-semibold text-zinc-200 hover:border-zinc-700 focus:border-emerald-400 focus:bg-zinc-900 focus:outline-none"
            aria-label="Project name"
            title="Click to rename project"
          />
        </div>

        <div class="h-4 w-px bg-zinc-800" />

        {/* Edit actions: Rotate, Delete, Undo, Redo */}
        <div class="flex items-center gap-1" role="group" aria-label="Canvas editing actions">
          <button
            type="button"
            class="flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700/60 bg-zinc-900 text-zinc-300 transition-colors hover:border-emerald-400 hover:text-emerald-400 disabled:opacity-30"
            disabled={!props.canRotate}
            aria-label="Rotate component"
            title="Rotate Component (R)"
            onClick={() => props.onRotateSelected()}
          >
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.83 6.72 2.24L21 8" />
              <path d="M21 3v5h-5" />
            </svg>
          </button>

          <button
            type="button"
            class="flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700/60 bg-zinc-900 text-zinc-300 transition-colors hover:border-red-500 hover:text-red-400 disabled:opacity-30"
            disabled={!props.canDelete}
            aria-label="Delete selected"
            title="Delete Selected (Delete/Backspace)"
            onClick={() => props.onDeleteSelected()}
          >
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="3 6 5 6 21 6" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            </svg>
          </button>

          <button
            type="button"
            class="flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700/60 bg-zinc-900 text-zinc-300 transition-colors hover:border-emerald-400 hover:text-emerald-400 disabled:opacity-30"
            disabled={!props.canUndo}
            aria-label="Undo"
            title="Undo (Ctrl+Z)"
            onClick={() => props.onUndo()}
          >
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
          </button>

          <button
            type="button"
            class="flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700/60 bg-zinc-900 text-zinc-300 transition-colors hover:border-emerald-400 hover:text-emerald-400 disabled:opacity-30"
            disabled={!props.canRedo}
            aria-label="Redo"
            title="Redo (Ctrl+Y)"
            onClick={() => props.onRedo()}
          >
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="23 4 23 10 17 10" />
              <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
            </svg>
          </button>
        </div>

        <div class="h-4 w-px bg-zinc-800" />

        {/* Wire Color circular swatches */}
        <div class="flex items-center gap-1.5" role="group" aria-label="Wire color controls">
          <For each={WIRE_COLORS}>
            {(color) => {
              const isSelected = (props.activeWireColor ?? props.wireColor) === color.value;
              return (
                <button
                  type="button"
                  data-color-option={color.value}
                  aria-label={`Wire color ${color.label}`}
                  aria-pressed={isSelected}
                  title={`Wire color: ${color.label}`}
                  class="flex h-6 w-6 items-center justify-center rounded-full border transition-all"
                  classList={{
                    "border-white ring-2 ring-emerald-500 scale-110": isSelected,
                    "border-zinc-700/80 hover:scale-105 hover:border-zinc-400": !isSelected,
                  }}
                  style={{ "background-color": color.hex }}
                  onClick={() => props.onChooseColor(color.value)}
                />
              );
            }}
          </For>
        </div>
      </div>

      {/* Right: Simulation control, Panels, Persistence, Components drawer */}
      <div class="flex items-center gap-2">
        {/* Start / Stop Simulation */}
        <Show
          when={state() === "running" || state() === "paused"}
          fallback={
            <button
              type="button"
              class="flex items-center gap-1.5 rounded-md border border-emerald-500/80 bg-emerald-600/20 px-3 py-1.5 text-xs font-semibold text-emerald-400 transition-all hover:bg-emerald-600 hover:text-white"
              aria-label="Start simulation"
              title="Start deterministic 16 MHz simulation"
              onClick={() => props.onStartSimulation()}
            >
              <svg class="h-3.5 w-3.5 fill-current" viewBox="0 0 24 24">
                <polygon points="5 3 19 12 5 21 5 3" />
              </svg>
              <span>Start</span>
            </button>
          }
        >
          <button
            type="button"
            class="flex items-center gap-1.5 rounded-md border border-red-500/80 bg-red-600/20 px-3 py-1.5 text-xs font-semibold text-red-400 transition-all hover:bg-red-600 hover:text-white animate-pulse"
            aria-label="Stop simulation"
            title="Stop simulation"
            onClick={() => props.onStopSimulation()}
          >
            <svg class="h-3.5 w-3.5 fill-current" viewBox="0 0 24 24">
              <rect x="6" y="6" width="12" height="12" rx="1" />
            </svg>
            <span>Stop</span>
          </button>
        </Show>

        <div class="h-4 w-px bg-zinc-800" />

        {/* Code Editor */}
        <button
          type="button"
          class="flex h-8 items-center gap-1.5 rounded-md border px-2.5 text-xs transition-colors"
          classList={{
            "border-emerald-500 bg-emerald-950/60 text-emerald-300 font-semibold": props.editorOpen,
            "border-zinc-700/60 bg-zinc-900 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100": !props.editorOpen,
          }}
          aria-label="Code Editor"
          aria-pressed={props.editorOpen}
          title="Toggle Arduino Code Editor & Serial Monitor"
          onClick={() => props.onToggleEditor()}
        >
          <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="16 18 22 12 16 6" />
            <polyline points="8 6 2 12 8 18" />
          </svg>
          <span>Code</span>
        </button>

        {/* Logic Analyzer */}
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border transition-colors"
          classList={{
            "border-emerald-500 bg-emerald-950/60 text-emerald-300": props.analyzerOpen,
            "border-zinc-700/60 bg-zinc-900 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100": !props.analyzerOpen,
          }}
          aria-label="Logic Analyzer"
          aria-pressed={props.analyzerOpen}
          title="Toggle 8-Channel Digital Logic Analyzer"
          onClick={() => props.onToggleAnalyzer()}
        >
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
          </svg>
        </button>

        {/* MQTT Bridge */}
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border transition-colors"
          classList={{
            "border-emerald-500 bg-emerald-950/60 text-emerald-300": props.mqttOpen,
            "border-zinc-700/60 bg-zinc-900 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100": !props.mqttOpen,
          }}
          aria-label="MQTT Bridge"
          aria-pressed={props.mqttOpen}
          title="Toggle Virtual MQTT Bridge Panel"
          onClick={() => props.onToggleMqtt()}
        >
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="2" y1="12" x2="22" y2="12" />
            <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
          </svg>
        </button>

        <div class="h-4 w-px bg-zinc-800" />

        {/* Save & Open */}
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700/60 bg-zinc-900 text-zinc-300 transition-colors hover:border-emerald-400 hover:text-emerald-400"
          aria-label="Save project"
          title="Save .nodus Project (Ctrl+S)"
          onClick={() => props.onSaveProject()}
        >
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
            <polyline points="17 21 17 13 7 13 7 21" />
            <polyline points="7 3 7 8 15 8" />
          </svg>
        </button>

        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700/60 bg-zinc-900 text-zinc-300 transition-colors hover:border-emerald-400 hover:text-emerald-400"
          aria-label="Open project"
          title="Open .nodus Project (Ctrl+O)"
          onClick={() => fileInputRef?.click()}
        >
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
        </button>
        <input
          ref={fileInputRef}
          type="file"
          accept=".nodus,application/json"
          class="hidden"
          onChange={(e) => props.onOpenProject(e)}
        />

        {/* Export Schematic SVG */}
        <button
          type="button"
          class="flex h-8 w-8 items-center justify-center rounded-md border border-zinc-700/60 bg-zinc-900 text-zinc-300 transition-colors hover:border-emerald-400 hover:text-emerald-400"
          aria-label="Export schematic"
          title="Export Schematic as SVG"
          onClick={() => props.onExportSchematic()}
        >
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z" />
            <circle cx="12" cy="13" r="4" />
          </svg>
        </button>

        <div class="h-4 w-px bg-zinc-800" />

        {/* Toggle Components Drawer */}
        <button
          type="button"
          class="flex h-8 items-center gap-1.5 rounded-md border px-2.5 text-xs font-semibold transition-colors"
          classList={{
            "border-emerald-500 bg-emerald-600 text-white": props.catalogOpen,
            "border-zinc-700/60 bg-zinc-900 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100": !props.catalogOpen,
          }}
          aria-label="Components"
          aria-pressed={props.catalogOpen}
          title="Toggle Components Drawer"
          onClick={() => props.onToggleCatalog()}
        >
          <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="3" width="7" height="7" />
            <rect x="14" y="3" width="7" height="7" />
            <rect x="14" y="14" width="7" height="7" />
            <rect x="3" y="14" width="7" height="7" />
          </svg>
          <span>Components</span>
        </button>
      </div>
    </header>
  );
}
