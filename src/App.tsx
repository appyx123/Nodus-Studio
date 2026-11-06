import ComponentCatalog from "./components/library/ComponentCatalog";

export default function App() {
  return (
    <div class="flex min-h-screen flex-col bg-zinc-950 text-zinc-100">
      <header class="flex flex-wrap items-center justify-between gap-4 border-b border-zinc-800 px-6 py-4">
        <div>
          <p class="text-xs font-semibold uppercase tracking-[0.25em] text-emerald-400">
            Nodus
          </p>
          <h1 class="text-lg font-semibold">Nodus Studio</h1>
        </div>
        <span class="rounded-full border border-zinc-700 px-3 py-1 text-xs text-zinc-300">
          Phase 1 · Component packages
        </span>
      </header>

      <div class="grid flex-1 lg:grid-cols-[320px_1fr]">
        <aside aria-label="Component packages" class="border-b border-zinc-800 lg:border-r lg:border-b-0">
          <ComponentCatalog />
        </aside>

        <main class="flex items-center justify-center px-6 py-16">
          <section aria-labelledby="welcome-heading" class="max-w-xl">
            <p class="text-sm font-medium text-emerald-400">Open-source IoT workbench</p>
            <h2 id="welcome-heading" class="mt-3 text-4xl font-semibold tracking-tight">
              A foundation for connected ideas.
            </h2>
            <p class="mt-5 leading-7 text-zinc-400">
              Component packages are validated by Rust and shown in the library.
              The interactive canvas, wiring, and simulation are not implemented yet.
            </p>
            <div class="mt-8 rounded-xl border border-zinc-800 bg-zinc-900/60 p-5">
              <h3 class="text-sm font-semibold">Current checkpoint</h3>
              <p class="mt-2 text-sm leading-6 text-zinc-400">
                Inspect the package previews in the library. Lua is not executed; the next step is the interactive canvas.
              </p>
            </div>
          </section>
        </main>
      </div>

      <footer class="border-t border-zinc-800 px-6 py-3 text-xs text-zinc-400">
        SolidJS interface · Rust backend · No simulation running
      </footer>
    </div>
  );
}
