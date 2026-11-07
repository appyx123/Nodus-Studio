use super::packet::*;
use super::types::{MqttConfig, MqttMessage, MqttStatus};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct MqttClient {
    config: MqttConfig,
    status: Arc<Mutex<MqttStatus>>,
    stop_signal: Arc<AtomicBool>,
    publish_queue: Arc<Mutex<Vec<MqttMessage>>>,
    incoming_queue: Arc<Mutex<Vec<MqttMessage>>>,
}

impl MqttClient {
    pub fn new(config: MqttConfig) -> Self {
        Self {
            config,
            status: Arc::new(Mutex::new(MqttStatus::default())),
            stop_signal: Arc::new(AtomicBool::new(false)),
            publish_queue: Arc::new(Mutex::new(Vec::new())),
            incoming_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn status(&self) -> MqttStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn publish(&self, topic: String, payload: String) {
        if let Ok(mut q) = self.publish_queue.lock() {
            if q.len() < 1000 {
                q.push(MqttMessage { topic, payload });
            }
        }
    }

    pub fn drain_incoming(&self) -> Vec<MqttMessage> {
        self.incoming_queue
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default()
    }

    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::SeqCst);
    }

    pub fn start(&self) {
        let config = self.config.clone();
        let status = Arc::clone(&self.status);
        let stop_signal = Arc::clone(&self.stop_signal);
        let pub_queue = Arc::clone(&self.publish_queue);
        let in_queue = Arc::clone(&self.incoming_queue);

        thread::spawn(move || {
            let addr = format!("{}:{}", config.host, config.port);
            let mut stream = match TcpStream::connect_timeout(
                &match addr.parse() {
                    Ok(a) => a,
                    Err(_) => {
                        if let Ok(mut s) = status.lock() {
                            s.last_error = Some("Invalid broker address".into());
                        }
                        return;
                    }
                },
                Duration::from_secs(3),
            ) {
                Ok(s) => s,
                Err(e) => {
                    if let Ok(mut s) = status.lock() {
                        s.last_error = Some(format!("Connection failed: {e}"));
                    }
                    return;
                }
            };

            let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

            // Send CONNECT packet
            let connect_pkt = encode_connect(&config.client_id);
            if stream.write_all(&connect_pkt).is_err() {
                if let Ok(mut s) = status.lock() {
                    s.last_error = Some("Failed to send CONNECT packet".into());
                }
                return;
            }

            // Read CONNACK
            let mut rx_buf = Vec::new();
            let mut chunk = [0u8; 1024];
            let mut connected = false;

            for _ in 0..60 {
                if stop_signal.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(n) = stream.read(&mut chunk) {
                    if n > 0 {
                        rx_buf.extend_from_slice(&chunk[..n]);
                        if let Ok(Some((MqttPacket::Connack { code, .. }, consumed))) =
                            decode_packet(&rx_buf)
                        {
                            rx_buf.drain(..consumed);
                            if code == 0 {
                                connected = true;
                                break;
                            } else {
                                if let Ok(mut s) = status.lock() {
                                    s.last_error = Some(format!("CONNACK error code: {code}"));
                                }
                                return;
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(50));
            }

            if !connected {
                if let Ok(mut s) = status.lock() {
                    s.last_error = Some("CONNACK timeout".into());
                }
                return;
            }

            // Subscribe to prefix/#
            let sub_topic = format!("{}/#", config.topic_prefix.trim_end_matches('/'));
            let sub_pkt = encode_subscribe(&sub_topic, 1, config.qos);
            let _ = stream.write_all(&sub_pkt);

            if let Ok(mut s) = status.lock() {
                s.connected = true;
                s.last_error = None;
            }

            let mut last_ping = std::time::Instant::now();

            while !stop_signal.load(Ordering::SeqCst) {
                // Handle outgoing publishes
                let to_send: Vec<MqttMessage> = pub_queue
                    .lock()
                    .map(|mut q| std::mem::take(&mut *q))
                    .unwrap_or_default();

                for msg in to_send {
                    let pkt =
                        encode_publish(&msg.topic, msg.payload.as_bytes(), config.qos, Some(2));
                    if stream.write_all(&pkt).is_ok() {
                        if let Ok(mut s) = status.lock() {
                            s.published_count = s.published_count.saturating_add(1);
                        }
                    }
                }

                // Handle incoming packets
                if let Ok(n) = stream.read(&mut chunk) {
                    if n > 0 {
                        rx_buf.extend_from_slice(&chunk[..n]);
                        while let Ok(Some((pkt, consumed))) = decode_packet(&rx_buf) {
                            rx_buf.drain(..consumed);
                            match pkt {
                                MqttPacket::Publish { topic, payload, .. } => {
                                    let body = String::from_utf8_lossy(&payload).into_owned();
                                    if let Ok(mut q) = in_queue.lock() {
                                        if q.len() < 1000 {
                                            q.push(MqttMessage {
                                                topic,
                                                payload: body,
                                            });
                                        }
                                    }
                                    if let Ok(mut s) = status.lock() {
                                        s.received_count = s.received_count.saturating_add(1);
                                    }
                                }
                                MqttPacket::Pingresp => {}
                                _ => {}
                            }
                        }
                    }
                }

                // Periodic PINGREQ every 30s
                if last_ping.elapsed() >= Duration::from_secs(30) {
                    let ping = encode_pingreq();
                    let _ = stream.write_all(&ping);
                    last_ping = std::time::Instant::now();
                }

                thread::sleep(Duration::from_millis(20));
            }

            let disc = encode_disconnect();
            let _ = stream.write_all(&disc);

            if let Ok(mut s) = status.lock() {
                s.connected = false;
            }
        });
    }
}
