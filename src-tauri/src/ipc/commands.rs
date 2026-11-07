use super::circuit::{self, ManagedCircuit};
use super::frame_streamer::FrameStreamer;
use super::simulation::{SimulationAction, SimulationController, SimulationStatus};
use crate::compiler::{CompileRequest, CompileResponse, CompilerController};
use crate::engine::bus::{SerialBuffer, SerialRequest, SerialSnapshot};
use crate::engine::{CircuitEdit, CircuitSnapshot};
use crate::sandbox::pkg_loader::{scan_registry, RegistryCatalog};
use crate::NodusError;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Manager;

fn resource_registry(resource_dir: &Path) -> PathBuf {
    resource_dir.join("components-registry")
}

fn resolve_existing_registry(default_root: PathBuf) -> PathBuf {
    if default_root.is_dir() {
        return default_root;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for rel in [
                "../share/nodus-studio/components-registry",
                "../components-registry",
                "components-registry",
            ] {
                let candidate = parent.join(rel);
                if candidate.is_dir() {
                    return candidate;
                }
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let user_registry = PathBuf::from(home).join(".local/share/nodus-studio/components-registry");
        if user_registry.is_dir() {
            return user_registry;
        }
    }
    for system_path in [
        "/usr/share/nodus-studio/components-registry",
        "/usr/local/share/nodus-studio/components-registry",
        "components-registry",
    ] {
        let p = PathBuf::from(system_path);
        if p.is_dir() {
            return p;
        }
    }
    default_root
}

#[tauri::command]
pub async fn list_components(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedCircuit>,
) -> Result<RegistryCatalog, NodusError> {
    let root = if cfg!(dev) {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../components-registry")
    } else {
        let default_root = resource_registry(
            &app.path()
                .resource_dir()
                .map_err(|error| NodusError::PackageDiscovery(error.to_string()))?,
        );
        resolve_existing_registry(default_root)
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        scan_registry(&root)
            .map_err(|error| NodusError::PackageDiscovery(format!("{}: {error}", root.display())))
    })
    .await
    .map_err(|error| NodusError::Worker(error.to_string()))?;
    circuit::register_scan(&state, result)
}

#[tauri::command]
pub async fn get_circuit(
    state: tauri::State<'_, ManagedCircuit>,
) -> Result<CircuitSnapshot, NodusError> {
    circuit::snapshot(&state)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn edit_circuit(
    controller: tauri::State<'_, SimulationController>,
    state: tauri::State<'_, ManagedCircuit>,
    edit: CircuitEdit,
) -> Result<CircuitSnapshot, NodusError> {
    if controller.is_active() {
        return Err(NodusError::Circuit(
            "Stop the simulation before editing the circuit".into(),
        ));
    }
    circuit::edit(&state, edit)
}

#[tauri::command]
pub async fn get_simulation_frame(
    streamer: tauri::State<'_, Arc<FrameStreamer>>,
) -> Result<tauri::ipc::Response, NodusError> {
    streamer.read_latest_bytes().map(tauri::ipc::Response::new)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn control_simulation(
    controller: tauri::State<'_, SimulationController>,
    circuit: tauri::State<'_, ManagedCircuit>,
    streamer: tauri::State<'_, Arc<FrameStreamer>>,
    compiler: tauri::State<'_, Arc<CompilerController>>,
    serial: tauri::State<'_, Arc<SerialBuffer>>,
    analyzer: tauri::State<'_, Arc<std::sync::Mutex<crate::engine::logic_analyzer::LogicAnalyzer>>>,
    action: SimulationAction,
) -> Result<SimulationStatus, NodusError> {
    controller.control_with_all_context(
        action,
        &circuit,
        &streamer,
        Some(Arc::clone(&compiler)),
        Some(Arc::clone(&serial)),
        Some(Arc::clone(&analyzer)),
    )
}

#[tauri::command]
pub async fn get_logic_analyzer_data(
    analyzer: tauri::State<'_, Arc<std::sync::Mutex<crate::engine::logic_analyzer::LogicAnalyzer>>>,
) -> Result<crate::engine::logic_analyzer::LogicAnalyzerSnapshot, NodusError> {
    let guard = analyzer
        .lock()
        .map_err(|_| NodusError::Circuit("Logic analyzer lock poisoned".into()))?;
    Ok(guard.snapshot(crate::engine::SimTime::ZERO))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn attach_logic_probe(
    analyzer: tauri::State<'_, Arc<std::sync::Mutex<crate::engine::logic_analyzer::LogicAnalyzer>>>,
    request: crate::engine::logic_analyzer::AttachProbeRequest,
) -> Result<(), NodusError> {
    let mut guard = analyzer
        .lock()
        .map_err(|_| NodusError::Circuit("Logic analyzer lock poisoned".into()))?;
    guard.attach_probe(request.channel_id, request.name, request.endpoint);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn detach_logic_probe(
    analyzer: tauri::State<'_, Arc<std::sync::Mutex<crate::engine::logic_analyzer::LogicAnalyzer>>>,
    channel_id: usize,
) -> Result<(), NodusError> {
    let mut guard = analyzer
        .lock()
        .map_err(|_| NodusError::Circuit("Logic analyzer lock poisoned".into()))?;
    guard.detach_probe(channel_id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn set_control_value(
    controller: tauri::State<'_, SimulationController>,
    instance_id: String,
    control_id: String,
    value: f64,
) -> Result<SimulationStatus, NodusError> {
    controller.set_control_value(instance_id, control_id, value)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn compile_sketch(
    compiler: tauri::State<'_, Arc<CompilerController>>,
    request: CompileRequest,
) -> Result<CompileResponse, NodusError> {
    let compiler = Arc::clone(&compiler);
    tauri::async_runtime::spawn_blocking(move || compiler.compile(request))
        .await
        .map_err(|error| NodusError::Worker(error.to_string()))?
}

#[tauri::command]
pub async fn cancel_compilation(
    compiler: tauri::State<'_, Arc<CompilerController>>,
) -> Result<(), NodusError> {
    compiler.cancel();
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn serial_io(
    serial: tauri::State<'_, Arc<SerialBuffer>>,
    request: SerialRequest,
) -> Result<SerialSnapshot, NodusError> {
    serial.handle_request(request)
}

#[tauri::command]
pub async fn get_mqtt_config(
    mqtt: tauri::State<'_, Arc<crate::mqtt::MqttBridgeController>>,
) -> Result<crate::mqtt::MqttConfig, NodusError> {
    Ok(mqtt.config())
}

#[tauri::command]
pub async fn set_mqtt_config(
    mqtt: tauri::State<'_, Arc<crate::mqtt::MqttBridgeController>>,
    config: crate::mqtt::MqttConfig,
) -> Result<(), NodusError> {
    mqtt.set_config(config);
    Ok(())
}

#[tauri::command]
pub async fn get_mqtt_status(
    mqtt: tauri::State<'_, Arc<crate::mqtt::MqttBridgeController>>,
) -> Result<crate::mqtt::MqttStatus, NodusError> {
    Ok(mqtt.status())
}

#[tauri::command]
pub async fn publish_mqtt_message(
    mqtt: tauri::State<'_, Arc<crate::mqtt::MqttBridgeController>>,
    topic: String,
    payload: String,
) -> Result<(), NodusError> {
    mqtt.publish(topic, payload);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_project(
    path: String,
    project: crate::project::NodusProjectFile,
) -> Result<(), NodusError> {
    crate::project::save_project_atomic(Path::new(&path), &project)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn load_project(
    controller: tauri::State<'_, SimulationController>,
    circuit: tauri::State<'_, ManagedCircuit>,
    path: String,
) -> Result<crate::project::LoadProjectResponse, NodusError> {
    if controller.is_active() {
        return Err(NodusError::Project(
            "Stop the simulation before loading a project".into(),
        ));
    }
    circuit::load_project(&circuit, Path::new(&path))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn export_project_json(
    project: crate::project::NodusProjectFile,
) -> Result<String, NodusError> {
    let clean = crate::project::sanitize_project(project);
    serde_json::to_string_pretty(&clean)
        .map_err(|err| NodusError::Project(format!("Failed to serialize project: {err}")))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn import_project_json(
    controller: tauri::State<'_, SimulationController>,
    circuit: tauri::State<'_, ManagedCircuit>,
    json_content: String,
) -> Result<crate::project::LoadProjectResponse, NodusError> {
    if controller.is_active() {
        return Err(NodusError::Project(
            "Stop the simulation before importing a project".into(),
        ));
    }
    let project = crate::project::parse_project_bytes(json_content.as_bytes())?;
    circuit::import_project(&circuit, &project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_resources_are_relative_to_the_runtime_resource_directory() {
        let resources = Path::new("relocated-app").join("resources");
        assert_eq!(
            resource_registry(&resources),
            resources.join("components-registry")
        );
    }
}
