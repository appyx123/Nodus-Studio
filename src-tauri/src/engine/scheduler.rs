use crate::NodusError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Exact fixed-point simulation time in half-nanoseconds (1 unit = 0.5 ns).
/// At 16 MHz, 1 MCU cycle = 62.5 ns = 125 half-ns exactly, eliminating integer drift.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
pub struct SimTime(u64);

pub const HALF_NS_PER_NS: u64 = 2;
pub const HALF_NS_PER_16MHZ_CYCLE: u64 = 125;
pub const HALF_NS_PER_SECOND: u64 = 2_000_000_000;
pub const MAX_PENDING_EVENTS: usize = 4_096;
pub const MAX_ZERO_ADVANCE_EVENTS: usize = 1_024;

impl SimTime {
    pub const ZERO: Self = Self(0);

    pub const fn from_half_ns(half_ns: u64) -> Self {
        Self(half_ns)
    }

    pub const fn half_ns(self) -> u64 {
        self.0
    }

    pub fn from_ns(ns: u64) -> Result<Self, NodusError> {
        ns.checked_mul(HALF_NS_PER_NS)
            .map(Self)
            .ok_or_else(|| NodusError::Circuit("Simulation timestamp overflow".into()))
    }

    pub fn from_cycles_16mhz(cycles: u64) -> Result<Self, NodusError> {
        cycles
            .checked_mul(HALF_NS_PER_16MHZ_CYCLE)
            .map(Self)
            .ok_or_else(|| NodusError::Circuit("Simulation cycle overflow".into()))
    }

    /// Converts internal half-nanoseconds to whole nanoseconds (floor) for `_ns` APIs
    /// without altering internal accumulator precision.
    pub const fn as_ns_floor(self) -> u64 {
        self.0 / HALF_NS_PER_NS
    }

    pub fn checked_add(self, delta: Self) -> Result<Self, NodusError> {
        self.0
            .checked_add(delta.0)
            .map(Self)
            .ok_or_else(|| NodusError::Circuit("Simulation time addition overflow".into()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Stopped,
    Running,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventPayload {
    LuaTimer {
        instance_id: String,
        callback: String,
    },
    InputEdge {
        instance_id: String,
        pin_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledEvent {
    pub handle: u64,
    pub timestamp: SimTime,
    pub payload: EventPayload,
}

pub struct Scheduler {
    now: SimTime,
    state: RunState,
    next_seq: u64,
    queue: BTreeMap<(SimTime, u64), ScheduledEvent>,
    handle_index: BTreeMap<u64, (SimTime, u64)>,
    zero_advance_streak: usize,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            now: SimTime::ZERO,
            state: RunState::Stopped,
            next_seq: 1,
            queue: BTreeMap::new(),
            handle_index: BTreeMap::new(),
            zero_advance_streak: 0,
        }
    }
}

impl Scheduler {
    pub fn now(&self) -> SimTime {
        self.now
    }

    pub fn state(&self) -> RunState {
        self.state
    }

    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    pub fn start(&mut self) {
        self.state = RunState::Running;
    }

    pub fn pause(&mut self) -> Result<(), NodusError> {
        if self.state != RunState::Running {
            return Err(NodusError::Circuit(
                "Cannot pause when simulation is not running".into(),
            ));
        }
        self.state = RunState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), NodusError> {
        if self.state != RunState::Paused {
            return Err(NodusError::Circuit(
                "Cannot resume when simulation is not paused".into(),
            ));
        }
        self.state = RunState::Running;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.state = RunState::Stopped;
        self.queue.clear();
        self.handle_index.clear();
        self.zero_advance_streak = 0;
    }

    pub fn reset(&mut self) {
        self.stop();
        self.now = SimTime::ZERO;
        self.next_seq = 1;
    }

    pub fn schedule_after(
        &mut self,
        delay: SimTime,
        payload: EventPayload,
    ) -> Result<u64, NodusError> {
        if self.queue.len() >= MAX_PENDING_EVENTS {
            return Err(NodusError::Circuit(
                "Scheduler event queue limit (4096) exceeded".into(),
            ));
        }
        let timestamp = self.now.checked_add(delay)?;
        let handle = self.next_seq;
        self.next_seq = self
            .next_seq
            .checked_add(1)
            .ok_or_else(|| NodusError::Circuit("Scheduler sequence overflow".into()))?;
        let key = (timestamp, handle);
        self.queue.insert(
            key,
            ScheduledEvent {
                handle,
                timestamp,
                payload,
            },
        );
        self.handle_index.insert(handle, key);
        Ok(handle)
    }

    pub fn cancel(&mut self, handle: u64) -> bool {
        if let Some(key) = self.handle_index.remove(&handle) {
            self.queue.remove(&key);
            true
        } else {
            false
        }
    }

    pub fn cancel_instance(&mut self, instance_id: &str) -> usize {
        let handles: Vec<u64> = self
            .queue
            .values()
            .filter(|event| match &event.payload {
                EventPayload::LuaTimer {
                    instance_id: owner, ..
                }
                | EventPayload::InputEdge {
                    instance_id: owner, ..
                } => owner == instance_id,
            })
            .map(|event| event.handle)
            .collect();
        for handle in &handles {
            self.cancel(*handle);
        }
        handles.len()
    }

    pub fn pop_next_until(
        &mut self,
        target: SimTime,
    ) -> Result<Option<ScheduledEvent>, NodusError> {
        if self.state != RunState::Running {
            return Ok(None);
        }
        let Some((&(timestamp, seq), _)) = self.queue.first_key_value() else {
            self.now = self.now.max(target);
            self.zero_advance_streak = 0;
            return Ok(None);
        };
        if timestamp > target {
            self.now = self.now.max(target);
            self.zero_advance_streak = 0;
            return Ok(None);
        }
        if timestamp == self.now {
            self.zero_advance_streak += 1;
            if self.zero_advance_streak > MAX_ZERO_ADVANCE_EVENTS {
                return Err(NodusError::Circuit(
                    "Exceeded maximum zero-advance events at identical timestamp".into(),
                ));
            }
        } else {
            self.now = timestamp;
            self.zero_advance_streak = 1;
        }
        let event = self.queue.remove(&(timestamp, seq));
        if let Some(ref item) = event {
            self.handle_index.remove(&item.handle);
        }
        Ok(event)
    }

    pub fn advance_cycles_16mhz(&mut self, cycles: u64) -> Result<SimTime, NodusError> {
        let delta = SimTime::from_cycles_16mhz(cycles)?;
        let target = self.now.checked_add(delta)?;
        self.now = target;
        self.zero_advance_streak = 0;
        Ok(self.now)
    }
}
