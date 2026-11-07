import { invoke, isTauri } from "@tauri-apps/api/core";
import { createSignal } from "solid-js";
import type { LoadProjectResponse, NodusProjectFile, ProjectWarning } from "../types/project";

const [projectName, setProjectName] = createSignal("Untitled Project");
const [projectWarnings, setProjectWarnings] = createSignal<ProjectWarning[]>([]);

export { projectName, setProjectName, projectWarnings, setProjectWarnings };

export async function saveProject(path: string, project: NodusProjectFile): Promise<void> {
  if (isTauri()) {
    await invoke("save_project", { path, project });
  } else {
    // Web fallback: download as file
    const content = JSON.stringify(project, null, 2);
    const blob = new Blob([content], { type: "application/x-nodus-project+json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = path.endsWith(".nodus") ? path : `${path}.nodus`;
    a.click();
    URL.revokeObjectURL(url);
  }
}

export async function loadProject(path: string): Promise<LoadProjectResponse> {
  if (!isTauri()) {
    throw new Error("Loading from file path requires native desktop app.");
  }
  const response = await invoke<LoadProjectResponse>("load_project", { path });
  setProjectName(response.project.metadata.name);
  setProjectWarnings(response.warnings);
  return response;
}

export async function exportProjectJson(project: NodusProjectFile): Promise<string> {
  if (isTauri()) {
    return invoke<string>("export_project_json", { project });
  }
  return JSON.stringify(project, null, 2);
}

export async function importProjectJson(jsonContent: string): Promise<LoadProjectResponse> {
  if (isTauri()) {
    const response = await invoke<LoadProjectResponse>("import_project_json", {
      jsonContent,
    });
    setProjectName(response.project.metadata.name);
    setProjectWarnings(response.warnings);
    return response;
  }
  const parsed = JSON.parse(jsonContent) as NodusProjectFile;
  const res: LoadProjectResponse = {
    project: parsed,
    warnings: [],
  };
  setProjectName(parsed.metadata.name);
  return res;
}
