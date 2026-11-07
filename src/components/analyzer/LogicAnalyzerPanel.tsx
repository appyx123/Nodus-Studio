import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import {
  analyzerOpen,
  analyzerSnapshot,
  attachLogicProbe,
  detachLogicProbe,
  fetchLogicAnalyzerData,
  setAnalyzerOpen,
} from "../../store/logicAnalyzerStore";
import type { ChannelSnapshot, LogicSample } from "../../store/logicAnalyzerStore";
import type { Endpoint } from "../../types/nodpkg";

export interface LogicAnalyzerPanelProps {
  selectedEndpoint?: Endpoint;
}

const TRACK_HEIGHT = 32;
const WAVE_WIDTH = 480;

function formatTime(halfNs: number): string {
  const ns = halfNs / 2;
  if (ns < 1_000) return `${ns.toFixed(0)} ns`;
  if (ns < 1_000_000) return `${(ns / 1_000).toFixed(1)} µs`;
  if (ns < 1_000_000_000) return `${(ns / 1_000_000).toFixed(2)} ms`;
  return `${(ns / 1_000_000_000).toFixed(3)} s`;
}

function buildWaveformPath(samples: LogicSample[], startTime: number, endTime: number): string {
  if (samples.length === 0) return "";
  const timeSpan = Math.max(1, endTime - startTime);
  const toX = (t: number) => {
    const clamped = Math.max(startTime, Math.min(endTime, t));
    return ((clamped - startTime) / timeSpan) * WAVE_WIDTH;
  };
  const toY = (lvl: number) => {
    if (lvl === 1) return 6; // High
    if (lvl === 0) return TRACK_HEIGHT - 6; // Low
    return TRACK_HEIGHT / 2; // HighZ / Unknown
  };

  let path = `M 0 ${toY(samples[0].level)}`;
  let lastX = 0;
  let lastY = toY(samples[0].level);

  for (let i = 0; i < samples.length; i++) {
    const s = samples[i];
    const x = toX(s.time_half_ns);
    const y = toY(s.level);
    if (x > lastX) {
      path += ` H ${x.toFixed(1)}`;
      lastX = x;
    }
    if (y !== lastY) {
      path += ` V ${y.toFixed(1)}`;
      lastY = y;
    }
  }

  // Extend to end of timeline
  if (lastX < WAVE_WIDTH) {
    path += ` H ${WAVE_WIDTH}`;
  }

  return path;
}

export default function LogicAnalyzerPanel(props: LogicAnalyzerPanelProps) {
  const [selectedChannel, setSelectedChannel] = createSignal(0);

  onMount(() => {
    void fetchLogicAnalyzerData();
    const timer = setInterval(() => {
      if (analyzerOpen()) {
        void fetchLogicAnalyzerData();
      }
    }, 150);
    onCleanup(() => clearInterval(timer));
  });

  const timeRange = () => {
    const snap = analyzerSnapshot();
    let minT = Number.MAX_SAFE_INTEGER;
    let maxT = snap.current_sim_time_half_ns;

    for (const ch of snap.channels) {
      if (ch.samples.length > 0) {
        minT = Math.min(minT, ch.samples[0].time_half_ns);
        maxT = Math.max(maxT, ch.samples[ch.samples.length - 1].time_half_ns);
      }
    }

    if (minT === Number.MAX_SAFE_INTEGER) {
      minT = Math.max(0, maxT - 20_000_000); // 10ms window default
    }

    return { start: minT, end: Math.max(minT + 1, maxT) };
  };

  async function handleAttach(channelId: number) {
    if (!props.selectedEndpoint) return;
    await attachLogicProbe({
      channel_id: channelId,
      name: `${props.selectedEndpoint.instance_id}.${props.selectedEndpoint.pin_id}`,
      endpoint: props.selectedEndpoint,
    });
  }

  return (
    <aside
      aria-label="Logic Analyzer"
      class="fixed bottom-0 left-0 right-0 z-40 max-h-80 border-t border-zinc-800 bg-zinc-950/95 p-4 shadow-2xl backdrop-blur transition-all flex flex-col"
    >
      <div class="mb-3 flex items-center justify-between border-b border-zinc-800 pb-2">
        <div class="flex items-center gap-3">
          <h3 class="text-sm font-semibold uppercase tracking-wider text-emerald-400">
            Digital Logic Analyzer
          </h3>
          <span class="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-300">
            Range: {formatTime(timeRange().start)} – {formatTime(timeRange().end)}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="workbench-button text-xs"
            onClick={() => void fetchLogicAnalyzerData()}
          >
            Refresh
          </button>
          <button
            type="button"
            class="workbench-button text-xs"
            onClick={() => setAnalyzerOpen(false)}
          >
            Close
          </button>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto space-y-2 pr-2">
        <For each={analyzerSnapshot().channels}>
          {(ch: ChannelSnapshot) => {
            const path = () => buildWaveformPath(ch.samples, timeRange().start, timeRange().end);
            const isProbed = () => Boolean(ch.endpoint);

            return (
              <div class="flex items-center gap-3 rounded border border-zinc-800/80 bg-zinc-900/60 p-2">
                <div class="w-44 flex-shrink-0">
                  <div class="flex items-center justify-between">
                    <span class="font-mono text-xs font-semibold text-zinc-200">
                      CH{ch.channel_id + 1}
                    </span>
                    <Show
                      when={isProbed()}
                      fallback={
                        <button
                          type="button"
                          class="rounded bg-emerald-600/30 px-1.5 py-0.5 text-[10px] text-emerald-300 hover:bg-emerald-600/50"
                          disabled={!props.selectedEndpoint}
                          onClick={() => void handleAttach(ch.channel_id)}
                        >
                          Attach
                        </button>
                      }
                    >
                      <button
                        type="button"
                        class="rounded bg-red-600/30 px-1.5 py-0.5 text-[10px] text-red-300 hover:bg-red-600/50"
                        onClick={() => void detachLogicProbe(ch.channel_id)}
                      >
                        Detach
                      </button>
                    </Show>
                  </div>
                  <p class="truncate text-[11px] text-zinc-400" title={ch.name}>
                    {ch.endpoint ? `${ch.endpoint.instance_id}.${ch.endpoint.pin_id}` : "(No probe)"}
                  </p>
                  <p class="text-[10px] text-zinc-500 font-mono">
                    {ch.samples.length} edges
                  </p>
                </div>

                <div class="relative flex-1 overflow-hidden rounded bg-black/70 border border-zinc-800 h-8">
                  <svg
                    viewBox={`0 0 ${WAVE_WIDTH} ${TRACK_HEIGHT}`}
                    preserveAspectRatio="none"
                    class="h-full w-full"
                  >
                    <line
                      x1="0"
                      y1={TRACK_HEIGHT / 2}
                      x2={WAVE_WIDTH}
                      y2={TRACK_HEIGHT / 2}
                      stroke="#27272a"
                      stroke-width="1"
                      stroke-dasharray="4 4"
                    />
                    <path
                      d={path()}
                      fill="none"
                      stroke="#34d399"
                      stroke-width="2"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </aside>
  );
}
