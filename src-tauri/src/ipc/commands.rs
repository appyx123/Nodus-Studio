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

#[tauri::command]
pub async fn list_components(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedCircuit>,
) -> Result<RegistryCatalog, NodusError> {
    let root = if cfg!(dev) {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../components-registry")
    } else {
        resource_registry(
            &app.path()
                .resource_dir()
                .map_err(|error| NodusError::PackageDiscovery(error.to_string()))?,
        )
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
    action: SimulationAction,
) -> Result<SimulationStatus, NodusError> {
    controller.control_with_context(
        action,
        &circuit,
        &streamer,
        Some(Arc::clone(&compiler)),
        Some(Arc::clone(&serial)),
    )
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
