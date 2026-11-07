pub mod client;
pub mod packet;
pub mod types;

pub use client::MqttClient;
pub use types::{MqttConfig, MqttMessage, MqttStatus};

use std::sync::{Arc, Mutex};

pub struct MqttBridgeController {
    config: Arc<Mutex<MqttConfig>>,
    client: Arc<Mutex<Option<MqttClient>>>,
}

impl Default for MqttBridgeController {
    fn default() -> Self {
        Self {
            config: Arc::new(Mutex::new(MqttConfig::default())),
            client: Arc::new(Mutex::new(None)),
        }
    }
}

impl MqttBridgeController {
    pub fn config(&self) -> MqttConfig {
        self.config.lock().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn set_config(&self, new_config: MqttConfig) {
        if let Ok(mut current) = self.config.lock() {
            *current = new_config.clone();
        }
        if let Ok(mut slot) = self.client.lock() {
            if let Some(old) = slot.take() {
                old.stop();
            }
            if new_config.enabled {
                let client = MqttClient::new(new_config);
                client.start();
                *slot = Some(client);
            }
        }
    }

    pub fn status(&self) -> MqttStatus {
        if let Ok(slot) = self.client.lock() {
            if let Some(c) = slot.as_ref() {
                return c.status();
            }
        }
        MqttStatus::default()
    }

    pub fn publish(&self, topic: String, payload: String) {
        if let Ok(slot) = self.client.lock() {
            if let Some(c) = slot.as_ref() {
                c.publish(topic, payload);
            }
        }
    }

    pub fn drain_incoming(&self) -> Vec<MqttMessage> {
        if let Ok(slot) = self.client.lock() {
            if let Some(c) = slot.as_ref() {
                return c.drain_incoming();
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
