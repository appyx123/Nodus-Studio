import { For } from "solid-js";
import { WIRE_COLORS } from "../../store/circuitStore";
import type { WireColor } from "../../types/nodpkg";

export interface WorkbenchToolbarProps {
  wireColor: WireColor;
  activeWireColor?: WireColor;
  onChooseColor: (color: WireColor) => void;
  zoom: number;
  minZoom: number;
  maxZoom: number;
  onChangeZoom: (factor: number) => void;
  onResetView: () => void;
  canDelete: boolean;
  onDeleteSelected: () => void;
  editorOpen: boolean;
  onToggleEditor: () => void;
  mqttOpen: boolean;
  onToggleMqtt: () => void;
  onSaveProject: () => void;
  onOpenProject: (event: Event) => void;
}

const buttonClass = "workbench-button";

export function WorkbenchToolbar(props: WorkbenchToolbarProps) {
  let fileInputRef: HTMLInputElement | undefined;

  return (
    <>
      <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Project actions">
        <button
          type="button"
          class={buttonClass}
          onClick={() => props.onSaveProject()}
        >
          Save .nodus
        </button>
        <button
          type="button"
          class={buttonClass}
          onClick={() => fileInputRef?.click()}
        >
          Open .nodus
        </button>
        <input
          ref={fileInputRef}
          type="file"
          accept=".nodus,application/json"
          class="hidden"
          onChange={(e) => props.onOpenProject(e)}
        />
      </div>

      <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Wire color controls">
        <span class="text-xs text-zinc-400">Wire color:</span>
        <For each={WIRE_COLORS}>
          {(color) => (
            <button
              type="button"
              class={buttonClass}
              data-color-option={color.value}
              aria-label={`Wire color ${color.label}`}
              aria-pressed={(props.activeWireColor ?? props.wireColor) === color.value}
              onClick={() => props.onChooseColor(color.value)}
            >
              <span
                class="inline-block h-2 w-2 rounded-full mr-1"
                style={{ background: color.hex }}
              />
              {color.label}
            </button>
          )}
        </For>
      </div>

      <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Viewport controls">
        <button
          class={buttonClass}
          aria-label="Zoom out"
          disabled={props.zoom <= props.minZoom}
          onClick={() => props.onChangeZoom(1 / 1.25)}
        >
          −
        </button>
        <output class="min-w-14 text-center font-mono text-xs" aria-label="Zoom level">
          {Math.round(props.zoom * 100)}%
        </output>
        <button
          class={buttonClass}
          aria-label="Zoom in"
          disabled={props.zoom >= props.maxZoom}
          onClick={() => props.onChangeZoom(1.25)}
        >
          +
        </button>
        <button class={buttonClass} onClick={() => props.onResetView()}>
          Reset view
        </button>
        <button
          class={buttonClass}
          disabled={!props.canDelete}
          onClick={() => props.onDeleteSelected()}
        >
          Delete selected
        </button>
        <button
          class={buttonClass}
          aria-pressed={props.editorOpen}
          onClick={() => props.onToggleEditor()}
        >
          Firmware &amp; Serial
        </button>
        <button
          class={buttonClass}
          aria-pressed={props.mqttOpen}
          onClick={() => props.onToggleMqtt()}
        >
          MQTT Bridge
        </button>
      </div>
    </>
  );
}
