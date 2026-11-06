fn main() -> Result<(), Box<dyn std::error::Error>> {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "list_components",
            "get_circuit",
            "edit_circuit",
        ]));
    tauri_build::try_build(attributes)?;
    Ok(())
}
