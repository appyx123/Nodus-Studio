import { createMemo, For, Show } from "solid-js";
import {
  cancelSketchCompilation,
  compileLog,
  compileStatus,
  compiling,
  diagnostics,
  runCompileSketch,
  setSketchSource,
  sketchSource,
} from "../../store/editorStore";

interface Props {
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

const buttonClass = "workbench-button";

export default function CodeEditor(props: Props) {
  const lines = createMemo(() => sketchSource().split("\n"));
  const diagnosticsByLine = createMemo(() => {
    const map = new Map<number, typeof diagnostics extends () => infer T ? T : never>();
    for (const item of diagnostics()) {
      const existing = map.get(item.line) ?? [];
      existing.push(item);
      map.set(item.line, existing);
    }
    return map;
  });

  async function onCompile() {
    props.onError("");
    const result = await runCompileSketch();
    if (!result) {
      props.onError(compileStatus());
      return;
    }
    if (result.success) {
      props.onStatus(`Compiled sketch (${result.hex_len} bytes HEX).`);
    } else {
      props.onError(result.status);
    }
  }

  async function onCancel() {
    await cancelSketchCompilation();
    props.onStatus("Compilation cancellation requested.");
  }

  return (
    <section class="editor-section" aria-label="Firmware code editor">
      <div class="editor-header">
        <div>
          <h3 class="text-xs font-semibold uppercase tracking-wider text-emerald-400">
            Monaco / Arduino C++ Editor
          </h3>
          <p class="text-[11px] text-zinc-400" role="status">
            {compileStatus()}
          </p>
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class={buttonClass}
            disabled={compiling()}
            onClick={() => {
              void onCompile();
            }}
          >
            Compile sketch
          </button>
          <button
            type="button"
            class={buttonClass}
            disabled={!compiling()}
            onClick={() => {
              void onCancel();
            }}
          >
            Cancel compile
          </button>
        </div>
      </div>

      <div class="editor-body">
        <div class="editor-gutter" aria-hidden="true">
          <For each={lines()}>
            {(_line, idx) => {
              const lineNumber = idx() + 1;
              const hasErr = () =>
                diagnosticsByLine()
                  .get(lineNumber)
                  ?.some((d) => d.severity === "error") ?? false;
              return (
                <div
                  class={hasErr() ? "gutter-line is-error-line" : "gutter-line"}
                  data-line-number={lineNumber}
                >
                  {lineNumber}
                </div>
              );
            }}
          </For>
        </div>
        <textarea
          aria-label="Arduino sketch editor"
          class="editor-textarea font-mono text-xs"
          spellcheck={false}
          value={sketchSource()}
          onInput={(event) => setSketchSource(event.currentTarget.value)}
        />
      </div>

      <Show when={diagnostics().length > 0}>
        <ul class="editor-diagnostics" aria-label="Compiler diagnostics">
          <For each={diagnostics()}>
            {(diag) => (
              <li
                class="editor-diagnostic-item font-mono text-xs"
                data-diagnostic-line={diag.line}
                data-diagnostic-severity={diag.severity}
                role={diag.severity === "error" ? "alert" : "status"}
              >
                Line {diag.line}:{diag.column} [{diag.severity}] {diag.message}
              </li>
            )}
          </For>
        </ul>
      </Show>

      <Show when={compileLog() && diagnostics().length === 0}>
        <pre class="editor-log font-mono text-[11px] text-zinc-400" aria-label="Compiler log">
          {compileLog()}
        </pre>
      </Show>
    </section>
  );
}
