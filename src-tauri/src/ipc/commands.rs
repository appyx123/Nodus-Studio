use crate::sandbox::pkg_loader::{scan_registry, RegistryCatalog};
use crate::NodusError;
use std::path::{Path, PathBuf};
use tauri::Manager;

fn resource_registry(resource_dir: &Path) -> PathBuf {
    resource_dir.join("components-registry")
}

#[tauri::command]
pub async fn list_components(app: tauri::AppHandle) -> Result<RegistryCatalog, NodusError> {
    let root = if cfg!(dev) {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../components-registry")
    } else {
        resource_registry(
            &app.path()
                .resource_dir()
                .map_err(|error| NodusError::PackageDiscovery(error.to_string()))?,
        )
    };
    tauri::async_runtime::spawn_blocking(move || {
        scan_registry(&root)
            .map_err(|error| NodusError::PackageDiscovery(format!("{}: {error}", root.display())))
    })
    .await
    .map_err(|error| NodusError::Worker(error.to_string()))?
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
