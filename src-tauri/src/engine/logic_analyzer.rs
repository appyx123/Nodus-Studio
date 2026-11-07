use super::pin::LogicLevel;
use super::scheduler::SimTime;
use super::types::Endpoint;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_SAMPLES_PER_CHANNEL: usize = 4096;
pub const MAX_CHANNELS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicSample {
    pub time_half_ns: u64,
    pub level: u8, // 0 = Low, 1 = High, 2 = HighZ, 3 = Conflict
}

pub fn logic_level_to_u8(level: LogicLevel) -> u8 {
    match level {
        LogicLevel::Low => 0,
        LogicLevel::High => 1,
        LogicLevel::HighZ => 2,
        LogicLevel::Unknown => 3,
    }
}

pub fn u8_to_logic_level(val: u8) -> LogicLevel {
    match val {
        0 => LogicLevel::Low,
        1 => LogicLevel::High,
        2 => LogicLevel::HighZ,
        _ => LogicLevel::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelSnapshot {
    pub channel_id: usize,
    pub name: String,
    pub endpoint: Option<Endpoint>,
    pub samples: Vec<LogicSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicAnalyzerSnapshot {
    pub current_sim_time_half_ns: u64,
    pub channels: Vec<ChannelSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachProbeRequest {
    pub channel_id: usize,
    pub name: String,
    pub endpoint: Endpoint,
}

#[derive(Debug, Clone)]
struct ChannelState {
    name: String,
    endpoint: Option<Endpoint>,
    last_level: Option<LogicLevel>,
    samples: VecDeque<LogicSample>,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            name: String::new(),
            endpoint: None,
            last_level: None,
            samples: VecDeque::with_capacity(MAX_SAMPLES_PER_CHANNEL),
        }
    }
}

pub struct LogicAnalyzer {
    channels: Vec<ChannelState>,
}

impl Default for LogicAnalyzer {
    fn default() -> Self {
        let mut channels = Vec::with_capacity(MAX_CHANNELS);
        for i in 0..MAX_CHANNELS {
            channels.push(ChannelState {
                name: format!("CH{}", i + 1),
                endpoint: None,
                last_level: None,
                samples: VecDeque::with_capacity(MAX_SAMPLES_PER_CHANNEL),
            });
        }
        Self { channels }
    }
}

impl LogicAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach_probe(&mut self, channel_id: usize, name: String, endpoint: Endpoint) {
        if channel_id < self.channels.len() {
            self.channels[channel_id].name = name;
            self.channels[channel_id].endpoint = Some(endpoint);
            self.channels[channel_id].last_level = None;
            self.channels[channel_id].samples.clear();
        }
    }

    pub fn detach_probe(&mut self, channel_id: usize) {
        if channel_id < self.channels.len() {
            self.channels[channel_id].endpoint = None;
            self.channels[channel_id].last_level = None;
            self.channels[channel_id].samples.clear();
        }
    }

    pub fn clear(&mut self) {
        for ch in &mut self.channels {
            ch.last_level = None;
            ch.samples.clear();
        }
    }

    pub fn record_edge(&mut self, endpoint: &Endpoint, level: LogicLevel, time: SimTime) {
        let half_ns = time.half_ns();
        let val = logic_level_to_u8(level);

        for ch in &mut self.channels {
            if ch.endpoint.as_ref() == Some(endpoint) && ch.last_level != Some(level) {
                if ch.samples.len() >= MAX_SAMPLES_PER_CHANNEL {
                    ch.samples.pop_front();
                }
                ch.samples.push_back(LogicSample {
                    time_half_ns: half_ns,
                    level: val,
                });
                ch.last_level = Some(level);
            }
        }
    }

    pub fn snapshot(&self, current_time: SimTime) -> LogicAnalyzerSnapshot {
        let mut channels = Vec::with_capacity(self.channels.len());
        for (i, ch) in self.channels.iter().enumerate() {
            channels.push(ChannelSnapshot {
                channel_id: i,
                name: ch.name.clone(),
                endpoint: ch.endpoint.clone(),
                samples: ch.samples.iter().cloned().collect(),
            });
        }
        LogicAnalyzerSnapshot {
            current_sim_time_half_ns: current_time.half_ns(),
            channels,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logic_analyzer_records_transitions_and_enforces_buffer_limit() {
        let mut analyzer = LogicAnalyzer::new();
        let ep = Endpoint {
            instance_id: "mcu-1".into(),
            pin_id: "13".into(),
        };

        analyzer.attach_probe(0, "D13".into(), ep.clone());

        // Record alternating edges
        for i in 0..(MAX_SAMPLES_PER_CHANNEL + 50) {
            let lvl = if i % 2 == 0 {
                LogicLevel::High
            } else {
                LogicLevel::Low
            };
            analyzer.record_edge(&ep, lvl, SimTime::from_ns(i as u64 * 100).unwrap());
        }

        let snap = analyzer.snapshot(SimTime::from_ns(500000).unwrap());
        assert_eq!(snap.channels[0].samples.len(), MAX_SAMPLES_PER_CHANNEL);
        // Verify buffer popped older samples and kept latest
        let last = snap.channels[0].samples.last().unwrap();
        assert_eq!(
            last.time_half_ns,
            (MAX_SAMPLES_PER_CHANNEL + 49) as u64 * 100 * 2
        );
    }

    #[test]
    fn unmonitored_endpoints_are_ignored() {
        let mut analyzer = LogicAnalyzer::new();
        let ep = Endpoint {
            instance_id: "sensor".into(),
            pin_id: "data".into(),
        };
        analyzer.record_edge(&ep, LogicLevel::High, SimTime::ZERO);
        let snap = analyzer.snapshot(SimTime::ZERO);
        assert!(snap.channels.iter().all(|c| c.samples.is_empty()));
    }
}
