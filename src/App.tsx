import { createResource } from "solid-js";
import Workbench from "./components/canvas/Workbench";
import { loadCatalog } from "./components/library/ComponentCatalog";

export default function App() {
  const [catalog, { refetch }] = createResource(loadCatalog);

  return (
    <div class="app-shell bg-zinc-950 text-zinc-100">
      <header class="flex flex-wrap items-center justify-between gap-4 border-b border-zinc-800 px-6 py-4">
        <div>
          <p class="text-xs font-semibold uppercase tracking-[0.25em] text-emerald-400">Nodus</p>
          <h1 class="text-lg font-semibold">Nodus Studio</h1>
        </div>
        <span class="rounded-full border border-zinc-700 px-3 py-1 text-xs text-zinc-300">
          Phase 1 · Interactive workbench
        </span>
      </header>
      <Workbench catalog={catalog} onReload={() => { void refetch(); }} />
      <footer class="border-t border-zinc-800 px-6 py-3 text-xs text-zinc-400">
        SolidJS interface · Rust backend · No simulation running
      </footer>
    </div>
  );
}
