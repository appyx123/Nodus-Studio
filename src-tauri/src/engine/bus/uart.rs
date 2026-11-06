use crate::NodusError;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

pub const MAX_TX_BUFFER_BYTES: usize = 64 * 1024;
pub const MAX_RX_QUEUE_BYTES: usize = 4 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SerialRequest {
    pub rx_input: Option<String>,
    pub clear: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialSnapshot {
    pub text: String,
    pub tx_bytes: u64,
    pub rx_pending: usize,
}

#[derive(Debug, Default)]
struct InnerSerial {
    tx_log: VecDeque<u8>,
    rx_queue: VecDeque<u8>,
    tx_bytes: u64,
}

#[derive(Debug, Default)]
pub struct SerialBuffer {
    inner: Mutex<InnerSerial>,
}

impl SerialBuffer {
    pub fn push_tx_byte(&self, byte: u8) {
        if let Ok(mut guard) = self.inner.lock() {
            if guard.tx_log.len() >= MAX_TX_BUFFER_BYTES {
                let _ = guard.tx_log.pop_front();
            }
            guard.tx_log.push_back(byte);
            guard.tx_bytes = guard.tx_bytes.saturating_add(1);
        }
    }

    pub fn push_tx_bytes(&self, bytes: &[u8]) {
        for &b in bytes {
            self.push_tx_byte(b);
        }
    }

    pub fn push_rx_input(&self, input: &str) -> Result<(), NodusError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| NodusError::Circuit("Serial buffer lock poisoned".into()))?;
        for &b in input.as_bytes() {
            if guard.rx_queue.len() >= MAX_RX_QUEUE_BYTES {
                return Err(NodusError::Circuit("Serial RX buffer is full.".into()));
            }
            guard.rx_queue.push_back(b);
        }
        Ok(())
    }

    pub fn pop_rx_byte(&self) -> Option<u8> {
        self.inner
            .lock()
            .ok()
            .and_then(|mut guard| guard.rx_queue.pop_front())
    }

    pub fn has_rx_byte(&self) -> bool {
        self.inner
            .lock()
            .ok()
            .is_some_and(|guard| !guard.rx_queue.is_empty())
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.tx_log.clear();
            guard.rx_queue.clear();
            guard.tx_bytes = 0;
        }
    }

    pub fn snapshot(&self) -> Result<SerialSnapshot, NodusError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| NodusError::Circuit("Serial buffer lock poisoned".into()))?;
        let raw: Vec<u8> = guard.tx_log.iter().copied().collect();
        Ok(SerialSnapshot {
            text: String::from_utf8_lossy(&raw).into_owned(),
            tx_bytes: guard.tx_bytes,
            rx_pending: guard.rx_queue.len(),
        })
    }

    pub fn handle_request(&self, req: SerialRequest) -> Result<SerialSnapshot, NodusError> {
        if req.clear.unwrap_or(false) {
            self.clear();
        }
        if let Some(input) = req.rx_input {
            if !input.is_empty() {
                self.push_rx_input(&input)?;
            }
        }
        self.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_buffer_tracks_tx_rx_and_clear() {
        let buf = SerialBuffer::default();
        buf.push_tx_bytes(b"Hello\r\n");
        buf.push_rx_input("A\n").unwrap();
        let snap = buf.snapshot().unwrap();
        assert_eq!(snap.text, "Hello\r\n");
        assert_eq!(snap.tx_bytes, 7);
        assert_eq!(snap.rx_pending, 2);
        assert_eq!(buf.pop_rx_byte(), Some(b'A'));
        assert_eq!(buf.pop_rx_byte(), Some(b'\n'));
        assert_eq!(buf.pop_rx_byte(), None);
        buf.clear();
        assert_eq!(buf.snapshot().unwrap().tx_bytes, 0);
    }
}
