use super::Endpoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogicLevel {
    High,
    Low,
    HighZ,
    Unknown,
}

impl LogicLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "HIGH",
            Self::Low => "LOW",
            Self::HighZ => "HIGH_Z",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PinMode {
    Input,
    InputPullup,
    InputPulldown,
    Output,
    OpenDrain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveStrength {
    None,
    WeakPullUp,
    WeakPullDown,
    StrongHigh,
    StrongLow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct PinRuntime {
    pub mode: PinMode,
    pub output_high: bool,
}

impl Default for PinRuntime {
    fn default() -> Self {
        Self {
            mode: PinMode::Input,
            output_high: false,
        }
    }
}

impl PinRuntime {
    pub fn effective_drive(self) -> DriveStrength {
        match self.mode {
            PinMode::Input => DriveStrength::None,
            PinMode::InputPullup => DriveStrength::WeakPullUp,
            PinMode::InputPulldown => DriveStrength::WeakPullDown,
            PinMode::Output => {
                if self.output_high {
                    DriveStrength::StrongHigh
                } else {
                    DriveStrength::StrongLow
                }
            }
            PinMode::OpenDrain => {
                if self.output_high {
                    DriveStrength::None
                } else {
                    DriveStrength::StrongLow
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetState {
    pub endpoints: Vec<Endpoint>,
    pub state: LogicLevel,
    pub conflict: bool,
    pub drivers: Vec<Endpoint>,
}

pub fn resolve_net<'a>(pins: impl IntoIterator<Item = (&'a Endpoint, PinRuntime)>) -> NetState {
    let mut endpoints = Vec::new();
    let mut strong_high = Vec::new();
    let mut strong_low = Vec::new();
    let mut weak_up = false;
    let mut weak_down = false;

    for (endpoint, runtime) in pins {
        endpoints.push(endpoint.clone());
        match runtime.effective_drive() {
            DriveStrength::None => {}
            DriveStrength::WeakPullUp => weak_up = true,
            DriveStrength::WeakPullDown => weak_down = true,
            DriveStrength::StrongHigh => strong_high.push(endpoint.clone()),
            DriveStrength::StrongLow => strong_low.push(endpoint.clone()),
        }
    }

    let (state, conflict, mut drivers) = match (!strong_high.is_empty(), !strong_low.is_empty()) {
        (true, true) => {
            strong_high.extend(strong_low);
            (LogicLevel::Unknown, true, strong_high)
        }
        (true, false) => (LogicLevel::High, false, strong_high),
        (false, true) => (LogicLevel::Low, false, strong_low),
        (false, false) => match (weak_up, weak_down) {
            (true, true) => (LogicLevel::Unknown, false, Vec::new()),
            (true, false) => (LogicLevel::High, false, Vec::new()),
            (false, true) => (LogicLevel::Low, false, Vec::new()),
            (false, false) => (LogicLevel::HighZ, false, Vec::new()),
        },
    };
    drivers.sort();

    NetState {
        endpoints,
        state,
        conflict,
        drivers,
    }
}
