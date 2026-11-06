pub mod compiler;
pub mod engine;
mod error;
pub mod ipc;
pub mod mcu;
pub mod sandbox;

pub use error::NodusError;

pub fn run() -> Result<(), NodusError> {
    tauri::Builder::default()
        .manage(ipc::circuit::ManagedCircuit::default())
        .manage(std::sync::Arc::new(
            ipc::frame_streamer::FrameStreamer::default(),
        ))
        .manage(std::sync::Arc::new(compiler::CompilerController::default()))
        .manage(std::sync::Arc::new(engine::bus::SerialBuffer::default()))
        .manage(ipc::simulation::SimulationController::default())
        .invoke_handler(tauri::generate_handler![
            ipc::commands::list_components,
            ipc::commands::get_circuit,
            ipc::commands::edit_circuit,
            ipc::commands::get_simulation_frame,
            ipc::commands::control_simulation,
            ipc::commands::set_control_value,
            ipc::commands::compile_sketch,
            ipc::commands::cancel_compilation,
            ipc::commands::serial_io
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
