import { invoke, isTauri } from "@tauri-apps/api/core";
import { createSignal } from "solid-js";

export type DiagnosticSeverity = "error" | "warning" | "note";

export interface CompilerDiagnostic {
  line: number;
  column: number;
  severity: DiagnosticSeverity;
  message: string;
}

export interface CompileResponse {
  success: boolean;
  status: string;
  fqbn: string;
  diagnostics: CompilerDiagnostic[];
  stdout: string;
  stderr: string;
  hex_len: number;
  elf_len: number;
}

export interface SerialSnapshot {
  text: string;
  tx_bytes: number;
  rx_pending: number;
}

export const DEFAULT_BLINK_SKETCH = `void setup() {
  Serial.begin(9600);
  pinMode(13, OUTPUT);
  Serial.println("Nodus Uno Ready");
}

void loop() {
  digitalWrite(13, HIGH);
  delay(100);
  digitalWrite(13, LOW);
  delay(100);
}
`;

const [editorOpen, setEditorOpen] = createSignal(false);
const [sketchSource, setSketchSource] = createSignal(DEFAULT_BLINK_SKETCH);
const [compiling, setCompiling] = createSignal(false);
const [compileStatus, setCompileStatus] = createSignal("Ready to compile (arduino:avr:uno)");
const [diagnostics, setDiagnostics] = createSignal<CompilerDiagnostic[]>([]);
const [compileLog, setCompileLog] = createSignal("");
const [serialSnapshot, setSerialSnapshot] = createSignal<SerialSnapshot>({
  text: "",
  tx_bytes: 0,
  rx_pending: 0,
});

export {
  editorOpen,
  setEditorOpen,
  sketchSource,
  setSketchSource,
  compiling,
  compileStatus,
  diagnostics,
  compileLog,
  serialSnapshot,
};

export async function runCompileSketch(): Promise<CompileResponse | undefined> {
  if (!isTauri()) {
    setCompileStatus("Compiler requires the native Tauri desktop runtime.");
    return undefined;
  }
  setCompiling(true);
  setCompileStatus("Compiling for arduino:avr:uno...");
  try {
    const response = await invoke<CompileResponse>("compile_sketch", {
      request: {
        sketch_source: sketchSource(),
        fqbn: "arduino:avr:uno",
      },
    });
    setDiagnostics(response.diagnostics);
    setCompileStatus(response.status);
    setCompileLog([response.stdout, response.stderr].filter(Boolean).join("\n"));
    return response;
  } catch (error) {
    const text = String(error);
    setCompileStatus(text);
    setCompileLog(text);
    return undefined;
  } finally {
    setCompiling(false);
  }
}

export async function cancelSketchCompilation(): Promise<void> {
  if (!isTauri()) return;
  await invoke("cancel_compilation");
  setCompileStatus("Cancelling compilation...");
}

export async function refreshSerialMonitor(): Promise<SerialSnapshot | undefined> {
  if (!isTauri()) return undefined;
  try {
    const snap = await invoke<SerialSnapshot>("serial_io", { request: {} });
    setSerialSnapshot(snap);
    return snap;
  } catch {
    return undefined;
  }
}

export async function sendSerialInput(input: string): Promise<SerialSnapshot | undefined> {
  if (!isTauri()) return undefined;
  const snap = await invoke<SerialSnapshot>("serial_io", {
    request: { rx_input: input.endsWith("\n") ? input : `${input}\n` },
  });
  setSerialSnapshot(snap);
  return snap;
}

export async function clearSerialMonitor(): Promise<SerialSnapshot | undefined> {
  if (!isTauri()) return undefined;
  const snap = await invoke<SerialSnapshot>("serial_io", {
    request: { clear: true },
  });
  setSerialSnapshot(snap);
  return snap;
}
