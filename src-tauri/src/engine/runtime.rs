use super::{CircuitState, Endpoint, EventPayload, LogicLevel, ScheduledEvent, Scheduler, SimTime};
use crate::sandbox::lua_host::{HostResult, InstanceDiagnostic, LuaSandboxInstance};
use crate::NodusError;
use std::collections::BTreeMap;

/// Single-threaded simulation core: Lua instances, scheduler and edge detection.
/// Lua states are not `Send`, so the owner must create and use this on one thread.
pub struct SimulationRuntime {
    instances: BTreeMap<String, LuaSandboxInstance>,
    scheduler: Scheduler,
    levels: BTreeMap<Endpoint, LogicLevel>,
    diagnostics: Vec<InstanceDiagnostic>,
}

impl SimulationRuntime {
    /// Resets all pin state, loads every instance's Lua script and runs `on_init`.
    pub fn start(circuit: &mut CircuitState) -> Result<Self, NodusError> {
        circuit.reset_all_pins();
        let mut scheduler = Scheduler::default();
        scheduler.start();
        let mut runtime = Self {
            instances: BTreeMap::new(),
            scheduler,
            levels: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        for program in circuit.instance_programs() {
            let id = program.instance_id;
            match LuaSandboxInstance::new(id.clone(), &program.manifest, &program.lua_source) {
                Ok(instance) => {
                    runtime.instances.insert(id, instance);
                }
                Err(diagnostic) => runtime.diagnostics.push(diagnostic),
            }
        }
        let ids: Vec<String> = runtime.instances.keys().cloned().collect();
        for id in ids {
            runtime.call(&id, circuit, |inst, c, s| inst.call_init(c, s));
        }
        runtime.detect_edges(circuit)?;
        Ok(runtime)
    }

    pub fn now(&self) -> SimTime {
        self.scheduler.now()
    }

    pub fn pending_events(&self) -> usize {
        self.scheduler.pending_count()
    }

    pub fn pause(&mut self) -> Result<(), NodusError> {
        self.scheduler.pause()
    }

    pub fn resume(&mut self) -> Result<(), NodusError> {
        self.scheduler.resume()
    }

    pub fn diagnostics(&self) -> &[InstanceDiagnostic] {
        &self.diagnostics
    }

    /// `(instance_id, faulted, visual values)` for every running instance.
    pub fn instance_states(&self) -> impl Iterator<Item = (&str, bool, &BTreeMap<String, f64>)> {
        self.instances
            .iter()
            .map(|(id, inst)| (id.as_str(), inst.fault().is_some(), inst.visual_values()))
    }

    /// Processes all events up to `now + delta`. No-op while paused.
    pub fn advance(
        &mut self,
        circuit: &mut CircuitState,
        delta: SimTime,
    ) -> Result<(), NodusError> {
        let target = self.scheduler.now().checked_add(delta)?;
        while let Some(event) = self.scheduler.pop_next_until(target)? {
            self.dispatch(event, circuit);
            self.detect_edges(circuit)?;
        }
        Ok(())
    }

    pub fn set_control(
        &mut self,
        circuit: &mut CircuitState,
        instance_id: &str,
        control_id: &str,
        value: f64,
    ) -> Result<(), NodusError> {
        if !self.instances.contains_key(instance_id) {
            return Err(NodusError::Circuit(format!(
                "Unknown simulation instance: {instance_id}"
            )));
        }
        self.call(instance_id, circuit, |inst, c, s| {
            inst.call_control_change(control_id, value, c, s)
        });
        self.detect_edges(circuit)
    }

    /// Drops every instance and event and releases all pins.
    pub fn shutdown(mut self, circuit: &mut CircuitState) {
        self.scheduler.reset();
        circuit.reset_all_pins();
    }

    fn call<F>(&mut self, id: &str, circuit: &mut CircuitState, run: F)
    where
        F: FnOnce(&mut LuaSandboxInstance, &mut CircuitState, &mut Scheduler) -> HostResult,
    {
        let Some(instance) = self.instances.get_mut(id) else {
            return;
        };
        if instance.fault().is_some() {
            return;
        }
        if let Err(diagnostic) = run(instance, circuit, &mut self.scheduler) {
            self.diagnostics.push(diagnostic);
        }
    }

    fn dispatch(&mut self, event: ScheduledEvent, circuit: &mut CircuitState) {
        let ScheduledEvent {
            handle,
            timestamp,
            payload,
        } = event;
        match payload {
            EventPayload::LuaTimer {
                instance_id,
                callback,
            } => self.call(&instance_id, circuit, |inst, c, s| {
                inst.call_timer(handle, &callback, timestamp, c, s)
            }),
            EventPayload::InputEdge {
                instance_id,
                pin_id,
            } => {
                let endpoint = Endpoint {
                    instance_id: instance_id.clone(),
                    pin_id: pin_id.clone(),
                };
                let level = circuit.read_pin(&endpoint).unwrap_or(LogicLevel::HighZ);
                self.call(&instance_id, circuit, |inst, c, s| {
                    inst.call_pin_edge(&pin_id, level, timestamp, c, s)
                });
            }
        }
    }

    /// Schedules a zero-delay `InputEdge` for every pin whose resolved level changed.
    fn detect_edges(&mut self, circuit: &CircuitState) -> Result<(), NodusError> {
        let mut current = BTreeMap::new();
        for net in circuit.resolve_nets() {
            for endpoint in net.endpoints {
                current.insert(endpoint, net.state);
            }
        }
        for (endpoint, level) in &current {
            let before = self
                .levels
                .get(endpoint)
                .copied()
                .unwrap_or(LogicLevel::HighZ);
            let live = self
                .instances
                .get(&endpoint.instance_id)
                .is_some_and(|inst| inst.fault().is_none());
            if before != *level && live {
                self.scheduler.schedule_after(
                    SimTime::ZERO,
                    EventPayload::InputEdge {
                        instance_id: endpoint.instance_id.clone(),
                        pin_id: endpoint.pin_id.clone(),
                    },
                )?;
            }
        }
        self.levels = current;
        Ok(())
    }
}
