use super::{
    pin::{resolve_net, LogicLevel, NetState, PinMode, PinRuntime},
    CircuitEdit, CircuitSnapshot, Endpoint, InstanceSnapshot, Net, Wire,
};
use crate::sandbox::{
    manifest::{Manifest, Pin, PinType},
    pkg_loader::RegistryCatalog,
};
use crate::NodusError;
use std::collections::{BTreeMap, BTreeSet};

const MAX_INSTANCES: usize = 256;
const MAX_WIRES: usize = 1024;

type Pins = BTreeMap<String, Pin>;

pub(crate) struct Definition {
    pub(crate) version: String,
    pub(crate) pins: Pins,
    pub(crate) manifest: Manifest,
    pub(crate) lua_source: String,
}

/// Everything the simulation runtime needs to start one component instance.
pub struct InstanceProgram {
    pub instance_id: String,
    pub manifest: Manifest,
    pub lua_source: String,
}

pub(crate) struct Instance {
    pub(crate) component_id: String,
    pub(crate) component_version: String,
    pub(crate) pins: Pins,
    pub(crate) runtime: BTreeMap<String, PinRuntime>,
}

/// Session topology only: no filesystem, presentation coordinates, or simulation.
#[derive(Default)]
pub struct CircuitState {
    pub(crate) catalog: BTreeMap<String, Definition>,
    pub(crate) instances: BTreeMap<String, Instance>,
    pub(crate) wires: BTreeMap<String, Wire>,
}

impl CircuitState {
    /// Only the Rust loader's successful scan is registered; existing pins stay owned.
    pub fn register_catalog(&mut self, catalog: &RegistryCatalog) {
        self.catalog = catalog
            .packages
            .iter()
            .map(|package| {
                let manifest = &package.manifest;
                (
                    manifest.component.id.clone(),
                    Definition {
                        version: manifest.component.version.clone(),
                        pins: manifest
                            .pins
                            .iter()
                            .map(|pin| (pin.id.clone(), pin.clone()))
                            .collect(),
                        manifest: manifest.clone(),
                        lua_source: package.lua_source.clone(),
                    },
                )
            })
            .collect();
    }

    /// Instances whose catalog definition still matches their pinned version.
    pub fn instance_programs(&self) -> Vec<InstanceProgram> {
        let mut programs = Vec::new();
        for (id, instance) in &self.instances {
            let Some(def) = self.catalog.get(&instance.component_id) else {
                continue;
            };
            if def.version == instance.component_version {
                programs.push(InstanceProgram {
                    instance_id: id.clone(),
                    manifest: def.manifest.clone(),
                    lua_source: def.lua_source.clone(),
                });
            }
        }
        programs
    }

    pub fn snapshot(&self) -> CircuitSnapshot {
        CircuitSnapshot {
            instances: self
                .instances
                .iter()
                .map(|(id, instance)| InstanceSnapshot {
                    instance_id: id.clone(),
                    component_id: instance.component_id.clone(),
                    component_version: instance.component_version.clone(),
                    pins: instance.pins.keys().cloned().collect(),
                })
                .collect(),
            wires: self.wires.values().cloned().collect(),
            nets: self.nets(),
        }
    }

    pub fn edit(&mut self, edit: CircuitEdit) -> Result<CircuitSnapshot, NodusError> {
        // Each branch finishes all fallible validation before its first write.
        match edit {
            CircuitEdit::AddInstance {
                instance_id,
                component_id,
                component_version,
            } => {
                validate_id(&instance_id, "node-")?;
                if self.instances.contains_key(&instance_id) {
                    return Err(circuit(format!("Duplicate instance ID: {instance_id}")));
                }
                if self.instances.len() >= MAX_INSTANCES {
                    return Err(circuit("Circuit exceeds 256 instances"));
                }
                let definition = self
                    .catalog
                    .get(&component_id)
                    .ok_or_else(|| circuit(format!("Unknown component: {component_id}")))?;
                if definition.version != component_version {
                    return Err(circuit(format!(
                        "Unsupported component version: {component_id}@{component_version}"
                    )));
                }
                let runtime = definition
                    .pins
                    .keys()
                    .map(|pin_id| (pin_id.clone(), PinRuntime::default()))
                    .collect();
                let instance = Instance {
                    component_id,
                    component_version,
                    pins: definition.pins.clone(),
                    runtime,
                };
                self.instances.insert(instance_id, instance);
            }
            CircuitEdit::RemoveInstance { instance_id } => {
                self.instance(&instance_id)?;
                self.instances.remove(&instance_id);
                self.wires.retain(|_, wire| {
                    wire.a.instance_id != instance_id && wire.b.instance_id != instance_id
                });
            }
            CircuitEdit::Connect {
                wire_id,
                a,
                b,
                color,
            } => {
                validate_id(&wire_id, "wire-")?;
                if self.wires.contains_key(&wire_id) {
                    return Err(circuit(format!("Duplicate wire ID: {wire_id}")));
                }
                if self.wires.len() >= MAX_WIRES {
                    return Err(circuit("Circuit exceeds 1024 wires"));
                }
                self.validate_endpoint(&a)?;
                self.validate_endpoint(&b)?;
                if a == b {
                    return Err(circuit("Wire endpoints must be distinct"));
                }
                if self
                    .wires
                    .values()
                    .any(|wire| (wire.a == a && wire.b == b) || (wire.a == b && wire.b == a))
                {
                    return Err(circuit("Endpoint pair is already connected"));
                }
                self.wires.insert(
                    wire_id.clone(),
                    Wire {
                        wire_id,
                        a,
                        b,
                        color,
                    },
                );
            }
            CircuitEdit::RemoveWire { wire_id } => {
                validate_id(&wire_id, "wire-")?;
                if self.wires.remove(&wire_id).is_none() {
                    return Err(circuit(format!("Unknown wire: {wire_id}")));
                }
            }
            CircuitEdit::SetWireColor { wire_id, color } => {
                validate_id(&wire_id, "wire-")?;
                let wire = self
                    .wires
                    .get_mut(&wire_id)
                    .ok_or_else(|| circuit(format!("Unknown wire: {wire_id}")))?;
                wire.color = color;
            }
        }
        Ok(self.snapshot())
    }

    fn instance(&self, id: &str) -> Result<&Instance, NodusError> {
        validate_id(id, "node-")?;
        self.instances
            .get(id)
            .ok_or_else(|| circuit(format!("Unknown instance: {id}")))
    }

    fn validate_endpoint(&self, endpoint: &Endpoint) -> Result<(), NodusError> {
        let instance = self.instance(&endpoint.instance_id)?;
        let pin = instance.pins.get(&endpoint.pin_id).ok_or_else(|| {
            circuit(format!(
                "Unknown pin: {}.{}",
                endpoint.instance_id, endpoint.pin_id
            ))
        })?;
        if matches!(pin.kind, PinType::Nc) {
            return Err(circuit(format!(
                "NC pin cannot connect: {}.{}",
                endpoint.instance_id, endpoint.pin_id
            )));
        }
        Ok(())
    }

    fn nets(&self) -> Vec<Net> {
        let mut adjacency: BTreeMap<Endpoint, BTreeSet<Endpoint>> = BTreeMap::new();
        for (id, instance) in &self.instances {
            for pin_id in instance.pins.keys() {
                adjacency.insert(
                    Endpoint {
                        instance_id: id.clone(),
                        pin_id: pin_id.clone(),
                    },
                    BTreeSet::new(),
                );
            }
        }
        for wire in self.wires.values() {
            adjacency
                .entry(wire.a.clone())
                .or_default()
                .insert(wire.b.clone());
            adjacency
                .entry(wire.b.clone())
                .or_default()
                .insert(wire.a.clone());
        }
        let mut unseen: BTreeSet<_> = adjacency.keys().cloned().collect();
        let mut nets = Vec::new();
        while let Some(start) = unseen.first().cloned() {
            let mut pending = BTreeSet::from([start]);
            let mut connected = BTreeSet::new();
            while let Some(endpoint) = pending.pop_first() {
                if !unseen.remove(&endpoint) {
                    continue;
                }
                if let Some(neighbors) = adjacency.get(&endpoint) {
                    pending.extend(
                        neighbors
                            .iter()
                            .filter(|neighbor| unseen.contains(*neighbor))
                            .cloned(),
                    );
                }
                connected.insert(endpoint);
            }
            nets.push(Net {
                endpoints: connected.into_iter().collect(),
            });
        }
        nets
    }

    pub fn set_pin_mode(&mut self, endpoint: &Endpoint, mode: PinMode) -> Result<(), NodusError> {
        self.validate_endpoint(endpoint)?;
        let instance = self
            .instances
            .get_mut(&endpoint.instance_id)
            .ok_or_else(|| circuit(format!("Unknown instance: {}", endpoint.instance_id)))?;
        if let Some(pin) = instance.runtime.get_mut(&endpoint.pin_id) {
            pin.mode = mode;
        }
        Ok(())
    }

    pub fn set_pin_output(&mut self, endpoint: &Endpoint, high: bool) -> Result<(), NodusError> {
        self.validate_endpoint(endpoint)?;
        let instance = self
            .instances
            .get_mut(&endpoint.instance_id)
            .ok_or_else(|| circuit(format!("Unknown instance: {}", endpoint.instance_id)))?;
        if let Some(pin) = instance.runtime.get_mut(&endpoint.pin_id) {
            pin.output_high = high;
        }
        Ok(())
    }

    pub fn release_instance_pins(&mut self, instance_id: &str) -> Result<(), NodusError> {
        validate_id(instance_id, "node-")?;
        let instance = self
            .instances
            .get_mut(instance_id)
            .ok_or_else(|| circuit(format!("Unknown instance: {instance_id}")))?;
        for pin in instance.runtime.values_mut() {
            *pin = PinRuntime::default();
        }
        Ok(())
    }

    pub fn reset_all_pins(&mut self) {
        for instance in self.instances.values_mut() {
            for pin in instance.runtime.values_mut() {
                *pin = PinRuntime::default();
            }
        }
    }

    pub fn resolve_nets(&self) -> Vec<NetState> {
        self.nets()
            .into_iter()
            .map(|net| {
                let items = net.endpoints.iter().map(|endpoint| {
                    let runtime = self
                        .instances
                        .get(&endpoint.instance_id)
                        .and_then(|inst| inst.runtime.get(&endpoint.pin_id))
                        .copied()
                        .unwrap_or_default();
                    (endpoint, runtime)
                });
                resolve_net(items)
            })
            .collect()
    }

    pub fn read_pin(&self, endpoint: &Endpoint) -> Result<LogicLevel, NodusError> {
        self.validate_endpoint(endpoint)?;
        for net in self.resolve_nets() {
            if net.endpoints.contains(endpoint) {
                return Ok(net.state);
            }
        }
        Ok(LogicLevel::HighZ)
    }
}

fn circuit(message: impl Into<String>) -> NodusError {
    NodusError::Circuit(message.into())
}

fn validate_id(id: &str, prefix: &str) -> Result<(), NodusError> {
    let uuid = id
        .strip_prefix(prefix)
        .ok_or_else(|| circuit(format!("ID must start with {prefix}")))?;
    if uuid.len() != 36
        || !uuid.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
    {
        return Err(circuit(format!(
            "ID must be {prefix} followed by a canonical lowercase ASCII UUID"
        )));
    }
    Ok(())
}
