interface Props {
  zoom: number;
  minZoom: number;
  maxZoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onResetView: () => void;
  onFitToView: () => void;
}

export default function CanvasNavigation(props: Props) {
  return (
    <nav
      aria-label="Canvas view navigation"
      class="absolute bottom-5 right-5 z-20 flex items-center gap-1 rounded-lg border border-zinc-700/80 bg-zinc-900/90 p-1.5 shadow-xl backdrop-blur-md"
    >
      <button
        type="button"
        class="flex h-7 w-7 items-center justify-center rounded text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-emerald-400 focus-visible:outline-2 focus-visible:outline-emerald-400"
        aria-label="Fit to view"
        title="Fit all components to screen (Space)"
        onClick={() => props.onFitToView()}
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
        </svg>
      </button>

      <div class="h-4 w-px bg-zinc-700" />

      <button
        type="button"
        class="flex h-7 w-7 items-center justify-center rounded text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-emerald-400 disabled:opacity-30 focus-visible:outline-2 focus-visible:outline-emerald-400"
        aria-label="Zoom in"
        title="Zoom In (+)"
        disabled={props.zoom >= props.maxZoom}
        onClick={() => props.onZoomIn()}
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>

      <output
        class="min-w-12 select-none text-center font-mono text-xs text-zinc-300"
        aria-label="Zoom level"
      >
        {Math.round(props.zoom * 100)}%
      </output>

      <button
        type="button"
        class="flex h-7 w-7 items-center justify-center rounded text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-emerald-400 disabled:opacity-30 focus-visible:outline-2 focus-visible:outline-emerald-400"
        aria-label="Zoom out"
        title="Zoom Out (-)"
        disabled={props.zoom <= props.minZoom}
        onClick={() => props.onZoomOut()}
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>

      <div class="h-4 w-px bg-zinc-700" />

      <button
        type="button"
        class="flex h-7 w-7 items-center justify-center rounded text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-emerald-400 focus-visible:outline-2 focus-visible:outline-emerald-400"
        aria-label="Reset view"
        title="Reset zoom to 100%"
        onClick={() => props.onResetView()}
      >
        <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
          <path d="M3 3v5h5" />
        </svg>
      </button>
    </nav>
  );
}
