use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MqttConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub client_id: String,
    pub topic_prefix: String,
    pub qos: u8,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".into(),
            port: 1883,
            client_id: "nodus-workbench".into(),
            topic_prefix: "nodus/workbench".into(),
            qos: 0,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MqttStatus {
    pub connected: bool,
    pub last_error: Option<String>,
    pub published_count: u64,
    pub received_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: String,
}
