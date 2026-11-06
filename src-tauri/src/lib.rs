mod error;
mod ipc;
pub mod sandbox;

pub use error::NodusError;

pub fn run() -> Result<(), NodusError> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ipc::commands::list_components])
        .run(tauri::generate_context!())?;
    Ok(())
}
