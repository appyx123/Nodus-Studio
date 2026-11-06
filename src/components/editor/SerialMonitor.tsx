import { createSignal, onCleanup, onMount } from "solid-js";
import {
  clearSerialMonitor,
  refreshSerialMonitor,
  sendSerialInput,
  serialSnapshot,
} from "../../store/editorStore";

const buttonClass = "workbench-button";

export default function SerialMonitor() {
  const [rxInput, setRxInput] = createSignal("");

  onMount(() => {
    void refreshSerialMonitor();
    const timer = window.setInterval(() => {
      void refreshSerialMonitor();
    }, 200);
    onCleanup(() => window.clearInterval(timer));
  });

  async function onSend(event: SubmitEvent) {
    event.preventDefault();
    const value = rxInput().trim();
    if (!value) return;
    setRxInput("");
    await sendSerialInput(value);
  }

  return (
    <section class="serial-section" aria-label="Serial monitor">
      <div class="editor-header">
        <div>
          <h3 class="text-xs font-semibold uppercase tracking-wider text-emerald-400">
            Serial Monitor (UART 9600 baud)
          </h3>
          <p class="text-[11px] text-zinc-400">
            TX: {serialSnapshot().tx_bytes} byte(s) · RX queued: {serialSnapshot().rx_pending}
          </p>
        </div>
        <button
          type="button"
          class={buttonClass}
          onClick={() => {
            void clearSerialMonitor();
          }}
        >
          Clear serial
        </button>
      </div>

      <pre
        class="serial-output font-mono text-xs"
        aria-label="Serial monitor output"
        role="log"
      >
        {serialSnapshot().text || "No serial output yet."}
      </pre>

      <form class="serial-form" onSubmit={(event) => void onSend(event)}>
        <input
          type="text"
          aria-label="Serial monitor input"
          placeholder="Send UART line to RX..."
          class="serial-input font-mono text-xs"
          value={rxInput()}
          onInput={(event) => setRxInput(event.currentTarget.value)}
        />
        <button type="submit" class={buttonClass}>
          Send serial
        </button>
      </form>
    </section>
  );
}
