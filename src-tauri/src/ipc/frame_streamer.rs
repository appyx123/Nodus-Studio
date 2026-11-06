// Producer side (simulation thread) is wired in Phase 2 Step 4.
#![allow(dead_code)]

use crate::engine::{Endpoint, LogicLevel, NetState, RunState, SimTime};
use crate::NodusError;
use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

pub const FRAME_MAGIC: &[u8; 4] = b"NDFR";
pub const FRAME_PROTOCOL_VERSION: u8 = 1;
pub const HEADER_BYTES: usize = 32;
pub const TARGET_FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);
pub const MAX_FRAME_INSTANCES: usize = 256;
pub const MAX_FRAME_VISUALS: usize = 64;
pub const MAX_FRAME_NETS: usize = 1024;
pub const MAX_FRAME_ENDPOINTS: usize = 512;
pub const MAX_ID_BYTES: usize = 128;

pub const FLAG_HAS_CONFLICT: u16 = 0x0001;
pub const FLAG_HAS_FAULT: u16 = 0x0002;

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceVisualSnapshot {
    pub instance_id: String,
    pub faulted: bool,
    pub visuals: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimulationFrame {
    pub seq: u64,
    pub run_state: RunState,
    pub sim_time: SimTime,
    pub speed_ratio: f32,
    pub instances: Vec<InstanceVisualSnapshot>,
    pub nets: Vec<NetState>,
}

impl Default for SimulationFrame {
    fn default() -> Self {
        Self {
            seq: 0,
            run_state: RunState::Stopped,
            sim_time: SimTime::ZERO,
            speed_ratio: 1.0,
            instances: Vec::new(),
            nets: Vec::new(),
        }
    }
}

fn frame_err(msg: impl Into<String>) -> NodusError {
    NodusError::Circuit(msg.into())
}

fn write_str(buf: &mut Vec<u8>, s: &str) -> Result<(), NodusError> {
    if s.is_empty() || s.len() > MAX_ID_BYTES {
        return Err(frame_err(format!(
            "Identifier length out of bounds: {}",
            s.len()
        )));
    }
    buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
    Ok(())
}

fn read_u8(bytes: &[u8], pos: &mut usize) -> Result<u8, NodusError> {
    let Some(&val) = bytes.get(*pos) else {
        return Err(frame_err("Truncated frame reading u8"));
    };
    *pos += 1;
    Ok(val)
}

fn read_u16(bytes: &[u8], pos: &mut usize) -> Result<u16, NodusError> {
    let Some(slice) = bytes.get(*pos..*pos + 2) else {
        return Err(frame_err("Truncated frame reading u16"));
    };
    *pos += 2;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u64(bytes: &[u8], pos: &mut usize) -> Result<u64, NodusError> {
    let Some(slice) = bytes.get(*pos..*pos + 8) else {
        return Err(frame_err("Truncated frame reading u64"));
    };
    *pos += 8;
    let mut arr = [0u8; 8];
    arr.copy_from_slice(slice);
    Ok(u64::from_le_bytes(arr))
}

fn read_str(bytes: &[u8], pos: &mut usize, max_len: usize) -> Result<String, NodusError> {
    let len = read_u16(bytes, pos)? as usize;
    if len == 0 || len > max_len {
        return Err(frame_err(format!("Invalid string length in frame: {len}")));
    }
    let Some(slice) = bytes.get(*pos..*pos + len) else {
        return Err(frame_err("Truncated frame reading string"));
    };
    *pos += len;
    std::str::from_utf8(slice)
        .map(str::to_owned)
        .map_err(|_| frame_err("Invalid UTF-8 string in binary frame"))
}

pub fn encode_frame(frame: &SimulationFrame) -> Result<Vec<u8>, NodusError> {
    if frame.instances.len() > MAX_FRAME_INSTANCES {
        return Err(frame_err("Frame instance count exceeds limit (256)"));
    }
    if frame.nets.len() > MAX_FRAME_NETS {
        return Err(frame_err("Frame net count exceeds limit (1024)"));
    }
    if !frame.speed_ratio.is_finite() || frame.speed_ratio < 0.0 {
        return Err(frame_err("Frame speed_ratio must be finite and >= 0"));
    }

    let mut flags = 0u16;
    if frame.nets.iter().any(|n| n.conflict) {
        flags |= FLAG_HAS_CONFLICT;
    }
    if frame.instances.iter().any(|i| i.faulted) {
        flags |= FLAG_HAS_FAULT;
    }
    let state_code = match frame.run_state {
        RunState::Stopped => 0u8,
        RunState::Running => 1u8,
        RunState::Paused => 2u8,
    };

    let mut buf = Vec::with_capacity(HEADER_BYTES + frame.instances.len() * 64);
    buf.extend_from_slice(FRAME_MAGIC);
    buf.push(FRAME_PROTOCOL_VERSION);
    buf.push(state_code);
    buf.extend_from_slice(&flags.to_le_bytes());
    buf.extend_from_slice(&frame.seq.to_le_bytes());
    buf.extend_from_slice(&frame.sim_time.half_ns().to_le_bytes());
    buf.extend_from_slice(&frame.speed_ratio.to_le_bytes());
    buf.extend_from_slice(&(frame.instances.len() as u16).to_le_bytes());
    buf.extend_from_slice(&(frame.nets.len() as u16).to_le_bytes());

    for inst in &frame.instances {
        if inst.visuals.len() > MAX_FRAME_VISUALS {
            return Err(frame_err("Instance visual count exceeds limit (64)"));
        }
        write_str(&mut buf, &inst.instance_id)?;
        buf.push(u8::from(inst.faulted));
        buf.extend_from_slice(&(inst.visuals.len() as u16).to_le_bytes());
        for (field, &val) in &inst.visuals {
            if !val.is_finite() {
                return Err(frame_err(format!("Non-finite visual value for {field}")));
            }
            write_str(&mut buf, field)?;
            buf.extend_from_slice(&val.to_le_bytes());
        }
    }

    for net in &frame.nets {
        if net.endpoints.len() > MAX_FRAME_ENDPOINTS {
            return Err(frame_err("Net endpoint count exceeds limit (512)"));
        }
        let level_code = match net.state {
            LogicLevel::Low => 0u8,
            LogicLevel::High => 1u8,
            LogicLevel::HighZ => 2u8,
            LogicLevel::Unknown => 3u8,
        };
        buf.push(level_code);
        buf.push(u8::from(net.conflict));
        buf.extend_from_slice(&(net.endpoints.len() as u16).to_le_bytes());
        for ep in &net.endpoints {
            write_str(&mut buf, &ep.instance_id)?;
            write_str(&mut buf, &ep.pin_id)?;
        }
    }
    Ok(buf)
}

pub fn decode_frame(bytes: &[u8]) -> Result<SimulationFrame, NodusError> {
    if bytes.len() < HEADER_BYTES {
        return Err(frame_err("Frame shorter than 32-byte header"));
    }
    if &bytes[0..4] != FRAME_MAGIC {
        return Err(frame_err("Invalid binary frame magic"));
    }
    let version = bytes[4];
    if version != FRAME_PROTOCOL_VERSION {
        return Err(frame_err(format!("Unsupported frame version: {version}")));
    }
    let run_state = match bytes[5] {
        0 => RunState::Stopped,
        1 => RunState::Running,
        2 => RunState::Paused,
        other => return Err(frame_err(format!("Invalid run_state code: {other}"))),
    };
    let mut pos = 6;
    let _flags = read_u16(bytes, &mut pos)?;
    let seq = read_u64(bytes, &mut pos)?;
    let half_ns = read_u64(bytes, &mut pos)?;
    let Some(speed_slice) = bytes.get(pos..pos + 4) else {
        return Err(frame_err("Truncated speed_ratio"));
    };
    pos += 4;
    let speed_ratio = f32::from_le_bytes([
        speed_slice[0],
        speed_slice[1],
        speed_slice[2],
        speed_slice[3],
    ]);
    if !speed_ratio.is_finite() || speed_ratio < 0.0 {
        return Err(frame_err("Invalid speed_ratio in binary frame"));
    }
    let inst_count = read_u16(bytes, &mut pos)? as usize;
    let net_count = read_u16(bytes, &mut pos)? as usize;
    if inst_count > MAX_FRAME_INSTANCES || net_count > MAX_FRAME_NETS {
        return Err(frame_err("Frame counts exceed architectural limits"));
    }

    let mut instances = Vec::with_capacity(inst_count);
    for _ in 0..inst_count {
        let instance_id = read_str(bytes, &mut pos, MAX_ID_BYTES)?;
        let faulted = match read_u8(bytes, &mut pos)? {
            0 => false,
            1 => true,
            other => return Err(frame_err(format!("Invalid fault flag: {other}"))),
        };
        let vis_count = read_u16(bytes, &mut pos)? as usize;
        if vis_count > MAX_FRAME_VISUALS {
            return Err(frame_err("Visual count exceeds limit"));
        }
        let mut visuals = BTreeMap::new();
        for _ in 0..vis_count {
            let field = read_str(bytes, &mut pos, 64)?;
            let val = f64::from_bits(read_u64(bytes, &mut pos)?);
            if !val.is_finite() {
                return Err(frame_err("Non-finite visual float in frame"));
            }
            visuals.insert(field, val);
        }
        instances.push(InstanceVisualSnapshot {
            instance_id,
            faulted,
            visuals,
        });
    }

    let mut nets = Vec::with_capacity(net_count);
    for _ in 0..net_count {
        let state = match read_u8(bytes, &mut pos)? {
            0 => LogicLevel::Low,
            1 => LogicLevel::High,
            2 => LogicLevel::HighZ,
            3 => LogicLevel::Unknown,
            other => return Err(frame_err(format!("Invalid logic state code: {other}"))),
        };
        let conflict = match read_u8(bytes, &mut pos)? {
            0 => false,
            1 => true,
            other => return Err(frame_err(format!("Invalid conflict flag: {other}"))),
        };
        let ep_count = read_u16(bytes, &mut pos)? as usize;
        if ep_count > MAX_FRAME_ENDPOINTS {
            return Err(frame_err("Endpoint count exceeds limit"));
        }
        let mut endpoints = Vec::with_capacity(ep_count);
        for _ in 0..ep_count {
            let instance_id = read_str(bytes, &mut pos, MAX_ID_BYTES)?;
            let pin_id = read_str(bytes, &mut pos, 64)?;
            endpoints.push(Endpoint {
                instance_id,
                pin_id,
            });
        }
        nets.push(NetState {
            endpoints,
            state,
            conflict,
            drivers: Vec::new(),
        });
    }

    if pos != bytes.len() {
        return Err(frame_err("Trailing unconsumed bytes in frame"));
    }

    Ok(SimulationFrame {
        seq,
        run_state,
        sim_time: SimTime::from_half_ns(half_ns),
        speed_ratio,
        instances,
        nets,
    })
}

/// Bounded visual frame streamer with latest-state-wins non-blocking semantics.
pub struct FrameStreamer {
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
    latest: Mutex<Vec<u8>>,
    next_seq: AtomicU64,
    replaced_count: AtomicU64,
}

impl Default for FrameStreamer {
    fn default() -> Self {
        let (tx, rx) = bounded(1);
        let initial = encode_frame(&SimulationFrame::default()).unwrap_or_default();
        Self {
            tx,
            rx,
            latest: Mutex::new(initial),
            next_seq: AtomicU64::new(1),
            replaced_count: AtomicU64::new(0),
        }
    }
}

impl FrameStreamer {
    pub fn next_seq(&self) -> u64 {
        self.next_seq.fetch_add(1, Ordering::Relaxed)
    }

    pub fn replaced_frames(&self) -> u64 {
        self.replaced_count.load(Ordering::Relaxed)
    }

    /// Publishes a frame without blocking the simulation thread.
    /// If the bounded visual queue is full, replaces the older frame (latest-state-wins).
    pub fn publish(&self, frame: &SimulationFrame) -> Result<(), NodusError> {
        let bytes = encode_frame(frame)?;
        if let Ok(mut guard) = self.latest.lock() {
            guard.clone_from(&bytes);
        }
        match self.tx.try_send(bytes) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(fresh)) => {
                let _ = self.rx.try_recv();
                let _ = self.tx.try_send(fresh);
                self.replaced_count.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(TrySendError::Disconnected(_)) => {
                Err(frame_err("Visual frame channel disconnected"))
            }
        }
    }

    /// Reads the latest binary frame for Tauri binary IPC (`tauri::ipc::Response`).
    /// Consumes the pending queue item if present, or returns the full cached snapshot for resync.
    pub fn read_latest_bytes(&self) -> Result<Vec<u8>, NodusError> {
        if let Ok(bytes) = self.rx.try_recv() {
            return Ok(bytes);
        }
        self.latest
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| frame_err("Frame streamer lock poisoned"))
    }
}

#[cfg(test)]
#[path = "frame_streamer_tests.rs"]
mod tests;
