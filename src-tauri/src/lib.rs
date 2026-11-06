pub mod engine;
mod error;
mod ipc;
pub mod sandbox;

pub use error::NodusError;

pub fn run() -> Result<(), NodusError> {
    tauri::Builder::default()
        .manage(ipc::circuit::ManagedCircuit::default())
        .invoke_handler(tauri::generate_handler![
            ipc::commands::list_components,
            ipc::commands::get_circuit,
            ipc::commands::edit_circuit
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
