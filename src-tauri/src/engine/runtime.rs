use super::{
    bus::SerialBuffer,
    logic_analyzer::LogicAnalyzer,
    scheduler::{RunState, HALF_NS_PER_16MHZ_CYCLE},
    CircuitState, Endpoint, EventPayload, LogicLevel, PinMode, ScheduledEvent, Scheduler, SimTime,
};
use crate::mcu::Microcontroller;
use crate::sandbox::lua_host::{HostResult, InstanceDiagnostic, LuaSandboxInstance};
use crate::NodusError;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

const MCU_SLICE_CYCLES: u64 = 1_600;

/// Single-threaded simulation core: Lua instances, MCU cores, scheduler, and edge detection.
/// Lua states are not `Send`, so the owner must create and use this on one thread.
pub struct SimulationRuntime {
    instances: BTreeMap<String, LuaSandboxInstance>,
    mcus: BTreeMap<String, Box<dyn Microcontroller>>,
    mcu_time: SimTime,
    scheduler: Scheduler,
    levels: BTreeMap<Endpoint, LogicLevel>,
    diagnostics: Vec<InstanceDiagnostic>,
    analyzer: Option<Arc<Mutex<LogicAnalyzer>>>,
}

impl SimulationRuntime {
    /// Resets all pin state, loads every instance's Lua script and runs `on_init`.
    pub fn start(circuit: &mut CircuitState) -> Result<Self, NodusError> {
        circuit.reset_all_pins();
        let mut scheduler = Scheduler::default();
        scheduler.start();
        let mut runtime = Self {
            instances: BTreeMap::new(),
            mcus: BTreeMap::new(),
            mcu_time: SimTime::ZERO,
            scheduler,
            levels: BTreeMap::new(),
            diagnostics: Vec::new(),
            analyzer: None,
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

    pub fn attach_mcu(
        &mut self,
        instance_id: String,
        mut mcu: Box<dyn Microcontroller>,
        circuit: &mut CircuitState,
    ) -> Result<(), NodusError> {
        sync_mcu_to_circuit(&instance_id, mcu.as_mut(), circuit);
        self.mcus.insert(instance_id, mcu);
        self.detect_edges(circuit)
    }

    pub fn attach_logic_analyzer(&mut self, analyzer: Arc<Mutex<LogicAnalyzer>>) {
        if let Ok(mut lock) = analyzer.lock() {
            for (endpoint, level) in &self.levels {
                lock.record_edge(endpoint, *level, self.scheduler.now());
            }
        }
        self.analyzer = Some(analyzer);
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

    /// Processes all events up to `now + delta`, stepping attached MCUs in lockstep.
    pub fn advance(
        &mut self,
        circuit: &mut CircuitState,
        delta: SimTime,
    ) -> Result<(), NodusError> {
        self.advance_with_serial(circuit, delta, None)
    }

    pub fn advance_with_serial(
        &mut self,
        circuit: &mut CircuitState,
        delta: SimTime,
        serial: Option<&SerialBuffer>,
    ) -> Result<(), NodusError> {
        if self.scheduler.state() != RunState::Running {
            return Ok(());
        }
        let target = self.scheduler.now().checked_add(delta)?;
        if self.mcus.is_empty() {
            while let Some(event) = self.scheduler.pop_next_until(target)? {
                self.dispatch(event, circuit);
                self.detect_edges(circuit)?;
            }
            return Ok(());
        }
        let slice_delta = SimTime::from_cycles_16mhz(MCU_SLICE_CYCLES)?;
        while self.scheduler.now() < target {
            let chunk_end = self.scheduler.now().checked_add(slice_delta)?.min(target);
            self.step_mcus_until(chunk_end, circuit, serial)?;
            while let Some(event) = self.scheduler.pop_next_until(chunk_end)? {
                self.dispatch(event, circuit);
                self.detect_edges(circuit)?;
            }
        }
        while let Some(event) = self.scheduler.pop_next_until(target)? {
            self.dispatch(event, circuit);
            self.detect_edges(circuit)?;
        }
        Ok(())
    }

    fn step_mcus_until(
        &mut self,
        target: SimTime,
        circuit: &mut CircuitState,
        serial: Option<&SerialBuffer>,
    ) -> Result<(), NodusError> {
        let diff_half_ns = target.half_ns().saturating_sub(self.mcu_time.half_ns());
        let cycles = diff_half_ns / HALF_NS_PER_16MHZ_CYCLE;
        if cycles == 0 {
            return Ok(());
        }
        let mut rx_bytes = Vec::new();
        if let Some(buf) = serial {
            while let Some(b) = buf.pop_rx_byte() {
                rx_bytes.push(b);
            }
        }
        let mut actual_cycles = cycles;
        for (instance_id, mcu) in &mut self.mcus {
            if !rx_bytes.is_empty() {
                mcu.push_serial_rx(&rx_bytes);
            }
            for (endpoint, &level) in &self.levels {
                if endpoint.instance_id == *instance_id {
                    mcu.set_input_pin(&endpoint.pin_id, level);
                }
            }
            actual_cycles = mcu.step_cycles(cycles)?;
            sync_mcu_to_circuit(instance_id, mcu.as_mut(), circuit);
            if let Some(buf) = serial {
                let tx = mcu.drain_serial_tx();
                if !tx.is_empty() {
                    buf.push_tx_bytes(&tx);
                }
            }
        }
        let step_time = SimTime::from_cycles_16mhz(actual_cycles)?;
        self.mcu_time = self.mcu_time.checked_add(step_time)?;
        self.detect_edges(circuit)
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
        self.mcus.clear();
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
                if let Some(mcu) = self.mcus.get_mut(&instance_id) {
                    mcu.set_input_pin(&pin_id, level);
                }
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
            if before != *level {
                if let Some(analyzer) = &self.analyzer {
                    if let Ok(mut lock) = analyzer.lock() {
                        lock.record_edge(endpoint, *level, self.scheduler.now());
                    }
                }
            }
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

fn sync_mcu_to_circuit(instance_id: &str, mcu: &dyn Microcontroller, circuit: &mut CircuitState) {
    for drive in mcu.pin_drives() {
        let endpoint = Endpoint {
            instance_id: instance_id.to_owned(),
            pin_id: drive.pin_id.to_owned(),
        };
        let _ = circuit.set_pin_mode(&endpoint, drive.mode);
        if matches!(drive.mode, PinMode::Output | PinMode::OpenDrain) {
            let _ = circuit.set_pin_output(&endpoint, drive.level == LogicLevel::High);
        }
    }
}
