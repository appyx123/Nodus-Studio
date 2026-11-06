use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub instance_id: String,
    pub pin_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WireColor {
    Green,
    Blue,
    Amber,
    Red,
    Violet,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CircuitEdit {
    AddInstance {
        instance_id: String,
        component_id: String,
        component_version: String,
    },
    RemoveInstance {
        instance_id: String,
    },
    Connect {
        wire_id: String,
        a: Endpoint,
        b: Endpoint,
        color: WireColor,
    },
    RemoveWire {
        wire_id: String,
    },
    SetWireColor {
        wire_id: String,
        color: WireColor,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstanceSnapshot {
    pub instance_id: String,
    pub component_id: String,
    pub component_version: String,
    pub pins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Wire {
    pub wire_id: String,
    pub a: Endpoint,
    pub b: Endpoint,
    pub color: WireColor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Net {
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CircuitSnapshot {
    pub instances: Vec<InstanceSnapshot>,
    pub wires: Vec<Wire>,
    pub nets: Vec<Net>,
}
