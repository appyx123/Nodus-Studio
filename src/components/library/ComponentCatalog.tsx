import { invoke, isTauri } from "@tauri-apps/api/core";
import { createMemo, createSignal, For, Show } from "solid-js";
import type { Resource } from "solid-js";
import type { ComponentPackage, RegistryCatalog } from "../../types/nodpkg";

export async function loadCatalog(): Promise<RegistryCatalog> {
  if (!isTauri()) {
    throw new Error(
      "Component discovery requires the Tauri desktop app. Browser preview has no filesystem access."
    );
  }
  return invoke<RegistryCatalog>("list_components");
}

interface Props {
  catalog: Resource<RegistryCatalog>;
  onReload: () => void;
  onPlace: (pkg: ComponentPackage) => void;
  onClose?: () => void;
}

type Category = "all" | "basic" | "sensors" | "actuators" | "mcu";

function matchCategory(pkg: ComponentPackage, category: Category): boolean {
  if (category === "all") return true;
  const id = pkg.manifest.component.id.toLowerCase();
  if (category === "basic") {
    return id.includes("basic") || id.includes("led") || id.includes("pushbutton") || id.includes("power");
  }
  if (category === "sensors") {
    return id.includes("sensor") || id.includes("dht") || id.includes("hcsr");
  }
  if (category === "actuators") {
    return id.includes("actuator") || id.includes("servo") || id.includes("motor");
  }
  if (category === "mcu") {
    return id.includes("mcu") || id.includes("arduino") || id.includes("uno");
  }
  return true;
}

export default function ComponentCatalog(props: Props) {
  const [search, setSearch] = createSignal("");
  const [category, setCategory] = createSignal<Category>("all");

  const packages = createMemo(() => {
    if (props.catalog.error) return [];
    try {
      return props.catalog()?.packages ?? [];
    } catch {
      return [];
    }
  });

  const filtered = createMemo(() => {
    const q = search().toLowerCase().trim();
    const cat = category();
    return packages().filter((pkg) => {
      const matchesText =
        !q ||
        pkg.manifest.component.name.toLowerCase().includes(q) ||
        pkg.manifest.component.id.toLowerCase().includes(q) ||
        pkg.manifest.component.description.toLowerCase().includes(q);
      return matchesText && matchCategory(pkg, cat);
    });
  });

  return (
    <section aria-labelledby="library-heading" class="flex h-full flex-col bg-zinc-900/95 text-zinc-100">
      <div class="flex items-center justify-between border-b border-zinc-800 p-4">
        <div>
          <h2 id="library-heading" class="text-sm font-semibold text-zinc-100">
            Components
          </h2>
          <p class="text-[11px] text-zinc-400">Tinkercad-style component library</p>
        </div>
        <div class="flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => props.onReload()}
            disabled={props.catalog.loading}
            aria-label="Reload"
            title="Reload registry"
            class="rounded border border-zinc-700 bg-zinc-800 p-1.5 text-zinc-300 hover:border-emerald-400 hover:text-emerald-400 disabled:opacity-40"
          >
            <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.83 6.72 2.24L21 8" />
              <path d="M21 3v5h-5" />
            </svg>
          </button>
          <Show when={props.onClose}>
            <button
              type="button"
              onClick={() => props.onClose?.()}
              aria-label="Close component library"
              class="rounded p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
            >
              <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </Show>
        </div>
      </div>

      <div class="border-b border-zinc-800 p-3 space-y-2">
        <div class="relative">
          <input
            type="search"
            value={search()}
            onInput={(e) => setSearch(e.currentTarget.value)}
            placeholder="Search components..."
            class="w-full rounded-md border border-zinc-700 bg-zinc-950 px-3 py-1.5 pl-8 text-xs text-zinc-200 placeholder-zinc-500 focus:border-emerald-400 focus:outline-none"
          />
          <svg
            class="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-zinc-500"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
        </div>

        <div class="flex flex-wrap gap-1 text-[11px]">
          {(
            [
              ["all", "All"],
              ["basic", "Basic"],
              ["sensors", "Sensors"],
              ["actuators", "Actuators"],
              ["mcu", "MCU"],
            ] as const
          ).map(([cat, label]) => (
            <button
              type="button"
              onClick={() => setCategory(cat)}
              class="rounded px-2 py-0.5 transition-colors"
              classList={{
                "bg-emerald-600 font-semibold text-white": category() === cat,
                "bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200": category() !== cat,
              }}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      <div class="flex-1 overflow-y-auto p-3">
        <Show when={props.catalog.loading}>
          <p role="status" class="py-8 text-center text-xs text-zinc-400">
            Loading components…
          </p>
        </Show>
        <Show when={props.catalog.error}>
          <p role="alert" class="break-words py-8 text-center text-xs text-red-400">
            {String(props.catalog.error)}
          </p>
        </Show>
        <Show when={!props.catalog.error && props.catalog()}>
          <p role="status" class="mb-2 text-xs text-emerald-400">
            {packages().length} package(s) loaded
          </p>
          <div class="grid grid-cols-2 gap-2.5">
            <For each={filtered()}>
              {(pkg) => (
                <div
                  class="group relative flex flex-col justify-between rounded-lg border border-zinc-800 bg-zinc-950/70 p-2.5 transition-all hover:border-emerald-500/60 hover:bg-zinc-950 hover:shadow-lg"
                  draggable={true}
                  onDragStart={(e) => {
                    if (!e.dataTransfer) return;
                    e.dataTransfer.effectAllowed = "copy";
                    e.dataTransfer.setData("application/x-nodus-component", pkg.manifest.component.id);
                  }}
                >
                  <div class="flex h-20 w-full items-center justify-center rounded bg-zinc-900/50 p-1">
                    <img
                      src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(pkg.svg)}`}
                      alt={pkg.manifest.component.name}
                      class="max-h-full max-w-full object-contain transition-transform group-hover:scale-105"
                    />
                  </div>
                  <div class="mt-2">
                    <h3 class="text-xs font-medium text-zinc-200 truncate">
                      {pkg.manifest.component.name}
                    </h3>
                    <p class="text-[10px] text-zinc-500 font-mono">
                      {pkg.manifest.pins.length} pins
                    </p>
                  </div>
                  <button
                    type="button"
                    class="mt-2 w-full rounded border border-zinc-700 bg-zinc-800/80 py-1 text-[11px] font-medium text-zinc-300 transition-colors hover:border-emerald-400 hover:bg-emerald-600 hover:text-white"
                    draggable={true}
                    data-component-id={pkg.manifest.component.id}
                    aria-label={`Add ${pkg.manifest.component.name}`}
                    onClick={() => props.onPlace(pkg)}
                    onDragStart={(e) => {
                      if (!e.dataTransfer) return;
                      e.dataTransfer.effectAllowed = "copy";
                      e.dataTransfer.setData("application/x-nodus-component", pkg.manifest.component.id);
                    }}
                  >
                    + Add
                  </button>
                </div>
              )}
            </For>
          </div>
          <Show when={filtered().length === 0 && !props.catalog.loading}>
            <p class="py-12 text-center text-xs text-zinc-500">
              No matching components found.
            </p>
          </Show>
        </Show>
      </div>
    </section>
  );
}
