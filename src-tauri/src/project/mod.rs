pub mod persistence;
pub mod types;

#[cfg(test)]
mod tests;

pub use persistence::{
    apply_project_to_circuit, load_project_from_path, parse_project_bytes, sanitize_project,
    save_project_atomic,
};
pub use types::{
    CircuitSection, EndpointRecord, FirmwareSection, InstanceRecord, LoadProjectResponse,
    MqttBridgeSection, NodusProjectFile, Point, ProjectMetadata, ProjectWarning, ViewportSection,
    WarningLevel, WireRecord, CURRENT_SCHEMA_VERSION, MAX_PROJECT_FILE_SIZE, NODUS_PROJECT_FORMAT,
};
