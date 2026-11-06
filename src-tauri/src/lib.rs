pub mod engine;
mod error;
mod ipc;
pub mod sandbox;

pub use error::NodusError;

pub fn run() -> Result<(), NodusError> {
    tauri::Builder::default()
        .manage(ipc::circuit::ManagedCircuit::default())
        .manage(ipc::frame_streamer::FrameStreamer::default())
        .invoke_handler(tauri::generate_handler![
            ipc::commands::list_components,
            ipc::commands::get_circuit,
            ipc::commands::edit_circuit,
            ipc::commands::get_simulation_frame
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
