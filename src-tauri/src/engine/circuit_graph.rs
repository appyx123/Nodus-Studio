use super::{CircuitEdit, CircuitSnapshot, Endpoint, InstanceSnapshot, Net, Wire};
use crate::sandbox::{
    manifest::{Pin, PinType},
    pkg_loader::RegistryCatalog,
};
use crate::NodusError;
use std::collections::{BTreeMap, BTreeSet};

const MAX_INSTANCES: usize = 256;
const MAX_WIRES: usize = 1024;

type Pins = BTreeMap<String, Pin>;

struct Definition {
    version: String,
    pins: Pins,
}

struct Instance {
    component_id: String,
    component_version: String,
    pins: Pins,
}

/// Session topology only: no filesystem, presentation coordinates, or simulation.
#[derive(Default)]
pub struct CircuitState {
    catalog: BTreeMap<String, Definition>,
    instances: BTreeMap<String, Instance>,
    wires: BTreeMap<String, Wire>,
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
                    },
                )
            })
            .collect();
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
                let instance = Instance {
                    component_id,
                    component_version,
                    pins: definition.pins.clone(),
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
