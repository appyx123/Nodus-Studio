use crate::engine::WireColor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NODUS_PROJECT_FORMAT: &str = "nodus-project";
pub const CURRENT_SCHEMA_VERSION: &str = "1.0.0";
pub const MAX_PROJECT_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MiB

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub created_at: u64,
    pub modified_at: u64,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceRecord {
    pub instance_id: String,
    pub component_id: String,
    pub component_version: String,
    pub position: Point,
    #[serde(default)]
    pub control_values: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointRecord {
    pub instance_id: String,
    pub pin_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireRecord {
    pub wire_id: String,
    pub a: EndpointRecord,
    pub b: EndpointRecord,
    pub color: WireColor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircuitSection {
    pub instances: Vec<InstanceRecord>,
    pub wires: Vec<WireRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewportSection {
    pub pan: Point,
    pub zoom: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirmwareSection {
    pub sketch_source: String,
    pub fqbn: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MqttBridgeSection {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub client_id: String,
    #[serde(default)]
    pub sub_topics: Vec<String>,
    #[serde(default)]
    pub pub_topics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodusProjectFile {
    pub format: String,
    pub version: String,
    pub app_id: String,
    pub metadata: ProjectMetadata,
    pub viewport: ViewportSection,
    pub circuit: CircuitSection,
    pub firmware: FirmwareSection,
    #[serde(default)]
    pub mqtt_bridge: Option<MqttBridgeSection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectWarning {
    pub level: WarningLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadProjectResponse {
    pub project: NodusProjectFile,
    pub warnings: Vec<ProjectWarning>,
}
