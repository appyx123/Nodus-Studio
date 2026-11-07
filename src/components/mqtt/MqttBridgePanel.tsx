import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { mqttOpen, setMqttOpen } from "../../store/mqttStore";
import type { MqttConfig, MqttStatus } from "../../store/mqttStore";

export default function MqttBridgePanel() {
  const [config, setConfig] = createSignal<MqttConfig>({
    enabled: false,
    host: "127.0.0.1",
    port: 1883,
    client_id: "nodus-workbench",
    topic_prefix: "nodus/workbench",
    qos: 0,
  });

  const [status, setStatus] = createSignal<MqttStatus>({
    connected: false,
    last_error: null,
    published_count: 0,
    received_count: 0,
  });

  const [testTopic, setTestTopic] = createSignal("nodus/workbench/test");
  const [testPayload, setTestPayload] = createSignal("hello from nodus");
  const [busy, setBusy] = createSignal(false);

  async function pollStatus() {
    try {
      const res = await invoke<MqttStatus>("get_mqtt_status");
      setStatus(res);
    } catch {
      // Mock / browser fallback
    }
  }

  async function loadConfig() {
    try {
      const res = await invoke<MqttConfig>("get_mqtt_config");
      setConfig(res);
    } catch {
      // Mock fallback
    }
    await pollStatus();
  }

  async function toggleConnect() {
    setBusy(true);
    const updated: MqttConfig = { ...config(), enabled: !config().enabled };
    setConfig(updated);
    try {
      await invoke("set_mqtt_config", { config: updated });
    } catch {
      // Mock fallback
    }
    await pollStatus();
    setBusy(false);
  }

  async function handlePublish() {
    try {
      await invoke("publish_mqtt_message", {
        topic: testTopic(),
        payload: testPayload(),
      });
      await pollStatus();
    } catch {
      // Mock fallback
    }
  }

  onMount(() => {
    void loadConfig();
    const timer = setInterval(() => {
      void pollStatus();
    }, 1000);
    onCleanup(() => clearInterval(timer));
  });

  return (
    <Show when={mqttOpen()}>
      <div
        class="fixed inset-y-0 right-0 z-50 flex w-96 flex-col border-l border-zinc-800 bg-zinc-900 shadow-2xl"
        role="dialog"
        aria-label="MQTT Bridge Panel"
      >
        <div class="flex items-center justify-between border-b border-zinc-800 p-4">
          <div class="flex items-center gap-2">
            <h2 class="text-sm font-semibold text-zinc-100">Virtual MQTT Bridge</h2>
            <span
              class={`rounded px-2 py-0.5 text-[10px] font-semibold uppercase ${
                status().connected
                  ? "bg-emerald-950 text-emerald-400 border border-emerald-800"
                  : "bg-zinc-800 text-zinc-400"
              }`}
            >
              {status().connected ? "Connected" : "Disconnected"}
            </span>
          </div>
          <button
            type="button"
            class="rounded px-2 py-1 text-xs text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
            onClick={() => setMqttOpen(false)}
          >
            ✕
          </button>
        </div>

        <div class="flex-1 space-y-4 overflow-y-auto p-4 text-xs">
          <div class="space-y-2">
            <label class="block font-medium text-zinc-300">Broker Host</label>
            <input
              type="text"
              class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-zinc-200 outline-none focus:border-emerald-500"
              value={config().host}
              disabled={config().enabled}
              onInput={(e) => setConfig({ ...config(), host: e.currentTarget.value })}
            />
          </div>

          <div class="grid grid-cols-2 gap-2">
            <div>
              <label class="block font-medium text-zinc-300">Port</label>
              <input
                type="number"
                class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-zinc-200 outline-none focus:border-emerald-500"
                value={config().port}
                disabled={config().enabled}
                onInput={(e) =>
                  setConfig({ ...config(), port: Number(e.currentTarget.value) || 1883 })
                }
              />
            </div>
            <div>
              <label class="block font-medium text-zinc-300">QoS</label>
              <select
                class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-zinc-200 outline-none focus:border-emerald-500"
                value={config().qos}
                disabled={config().enabled}
                onChange={(e) =>
                  setConfig({ ...config(), qos: Number(e.currentTarget.value) || 0 })
                }
              >
                <option value="0">0 (At most once)</option>
                <option value="1">1 (At least once)</option>
              </select>
            </div>
          </div>

          <div class="space-y-2">
            <label class="block font-medium text-zinc-300">Client ID</label>
            <input
              type="text"
              class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-zinc-200 outline-none focus:border-emerald-500"
              value={config().client_id}
              disabled={config().enabled}
              onInput={(e) => setConfig({ ...config(), client_id: e.currentTarget.value })}
            />
          </div>

          <div class="space-y-2">
            <label class="block font-medium text-zinc-300">Topic Prefix</label>
            <input
              type="text"
              class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-zinc-200 outline-none focus:border-emerald-500"
              value={config().topic_prefix}
              disabled={config().enabled}
              onInput={(e) => setConfig({ ...config(), topic_prefix: e.currentTarget.value })}
            />
          </div>

          <button
            type="button"
            class={`w-full rounded px-4 py-2 font-semibold text-xs transition ${
              config().enabled
                ? "bg-rose-900/60 text-rose-300 hover:bg-rose-800"
                : "bg-emerald-600 text-white hover:bg-emerald-500"
            }`}
            disabled={busy()}
            onClick={() => { void toggleConnect(); }}
          >
            {config().enabled ? "Disconnect Bridge" : "Connect to Broker"}
          </button>

          <Show when={status().last_error}>
            <div class="rounded border border-rose-800/60 bg-rose-950/40 p-2 text-rose-300">
              Error: {status().last_error}
            </div>
          </Show>

          <div class="grid grid-cols-2 gap-2 border-t border-zinc-800 pt-3 text-center">
            <div class="rounded bg-zinc-950 p-2">
              <span class="block text-zinc-500 text-[10px]">Messages Sent</span>
              <span class="font-mono text-sm text-zinc-200">{status().published_count}</span>
            </div>
            <div class="rounded bg-zinc-950 p-2">
              <span class="block text-zinc-500 text-[10px]">Messages Received</span>
              <span class="font-mono text-sm text-zinc-200">{status().received_count}</span>
            </div>
          </div>

          <div class="border-t border-zinc-800 pt-3 space-y-2">
            <h3 class="font-semibold text-zinc-200">Publish Test Message</h3>
            <input
              type="text"
              class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-1 text-zinc-200"
              value={testTopic()}
              onInput={(e) => setTestTopic(e.currentTarget.value)}
              placeholder="Topic"
            />
            <input
              type="text"
              class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-1 text-zinc-200"
              value={testPayload()}
              onInput={(e) => setTestPayload(e.currentTarget.value)}
              placeholder="Payload"
            />
            <button
              type="button"
              class="w-full rounded bg-zinc-800 px-3 py-1.5 font-medium text-zinc-200 hover:bg-zinc-700"
              onClick={() => { void handlePublish(); }}
            >
              Send Message
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
}
