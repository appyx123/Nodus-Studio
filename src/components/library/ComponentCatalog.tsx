import { invoke, isTauri } from "@tauri-apps/api/core";
import { createResource, For, Show } from "solid-js";
import type { RegistryCatalog } from "../../types/nodpkg";

async function loadCatalog(): Promise<RegistryCatalog> {
  if (!isTauri()) {
    throw new Error("Component discovery requires the Tauri desktop app. Browser preview has no filesystem access.");
  }
  return invoke<RegistryCatalog>("list_components");
}

export default function ComponentCatalog() {
  const [catalog, { refetch }] = createResource(loadCatalog);

  return (
    <section aria-labelledby="library-heading" class="p-6">
      <div class="flex items-center justify-between gap-3">
        <h2 id="library-heading" class="text-sm font-semibold">Component library</h2>
        <button type="button" onClick={() => refetch()} disabled={catalog.loading}
          class="rounded border border-zinc-600 px-2 py-1 text-xs text-zinc-200 hover:border-emerald-400 focus-visible:outline-2 focus-visible:outline-emerald-400 disabled:opacity-50">
          Reload
        </button>
      </div>
      <p class="mt-2 text-xs leading-5 text-zinc-400">Validated .nodpkg packages · visual preview only</p>
      <Show when={catalog.loading}>
        <p role="status" class="mt-5 text-sm text-zinc-400">Loading component packages…</p>
      </Show>
      <Show when={catalog.error}>
        <p role="alert" class="mt-5 break-words text-sm text-red-300">{String(catalog.error)}</p>
      </Show>
      <Show when={!catalog.error && catalog()}>{(data) => (
        <>
          <p role="status" class="mt-5 text-xs text-emerald-400">{data().packages.length} package(s) loaded</p>
          <Show when={data().packages.length === 0}>
            <p class="mt-4 text-sm text-zinc-400">No valid component packages found.</p>
          </Show>
          <ul class="mt-4 space-y-4">
            <For each={data().packages}>{(pkg) => (
              <li class="rounded-xl border border-zinc-700 bg-zinc-900 p-4">
                <img src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(pkg.svg)}`}
                  alt={`${pkg.manifest.component.name} package graphic`} width="100" height="134"
                  class="mx-auto h-34 w-25 object-contain" />
                <h3 class="mt-3 text-sm font-semibold">{pkg.manifest.component.name}</h3>
                <p class="mt-1 font-mono text-xs text-emerald-400">{pkg.manifest.component.id}</p>
                <p class="mt-2 text-xs leading-5 text-zinc-400">{pkg.manifest.component.description}</p>
                <p class="mt-2 text-xs text-zinc-400">{pkg.manifest.pins.length} pins · v{pkg.manifest.component.version}</p>
              </li>
            )}</For>
          </ul>
          <Show when={data().diagnostics.length > 0}>
            <div class="mt-5" role="alert">
              <h3 class="text-sm font-semibold text-amber-300">Rejected packages</h3>
              <ul class="mt-2 space-y-2 text-xs text-amber-200">
                <For each={data().diagnostics}>{(diagnostic) => (
                  <li class="break-words"><span class="font-mono">{diagnostic.path}</span>: {diagnostic.message}</li>
                )}</For>
              </ul>
            </div>
          </Show>
        </>
      )}</Show>
    </section>
  );
}
