#[derive(Debug, thiserror::Error)]
pub enum NodusError {
    #[error("Could not run Nodus Studio: {0}")]
    Application(#[from] tauri::Error),
    #[error("Component discovery failed: {0}")]
    PackageDiscovery(String),
    #[error("Component discovery worker failed: {0}")]
    Worker(String),
}

impl serde::Serialize for NodusError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
