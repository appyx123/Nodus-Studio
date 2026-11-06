use super::circuit::{self, ManagedCircuit};
use crate::engine::{CircuitEdit, CircuitSnapshot};
use crate::sandbox::pkg_loader::{scan_registry, RegistryCatalog};
use crate::NodusError;
use std::path::{Path, PathBuf};
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
    state: tauri::State<'_, ManagedCircuit>,
    edit: CircuitEdit,
) -> Result<CircuitSnapshot, NodusError> {
    circuit::edit(&state, edit)
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
