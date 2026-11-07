use super::circuit::ManagedCircuit;
use super::frame_streamer::{FrameStreamer, SimulationFrame};
use super::simulation_worker::{run_thread, ThreadCmd};
use crate::compiler::CompilerController;
use crate::engine::bus::SerialBuffer;
use crate::engine::logic_analyzer::LogicAnalyzer;
use crate::engine::{RunState, SimTime};
use crate::sandbox::lua_host::InstanceDiagnostic;
use crate::NodusError;
use crossbeam_channel::{bounded, Sender};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationAction {
    Start,
    Pause,
    Resume,
    Stop,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimulationStatus {
    pub state: RunState,
    pub sim_time_ns: u64,
    pub diagnostics: Vec<InstanceDiagnostic>,
}

struct Worker {
    tx: Sender<ThreadCmd>,
    handle: JoinHandle<()>,
}

/// Manages the dedicated simulation OS thread.
#[derive(Default)]
pub struct SimulationController {
    worker: Mutex<Option<Worker>>,
}

impl SimulationController {
    pub fn control(
        &self,
        action: SimulationAction,
        circuit: &ManagedCircuit,
        streamer: &Arc<FrameStreamer>,
    ) -> Result<SimulationStatus, NodusError> {
        self.control_with_context(action, circuit, streamer, None, None)
    }

    pub fn control_with_context(
        &self,
        action: SimulationAction,
        circuit: &ManagedCircuit,
        streamer: &Arc<FrameStreamer>,
        compiler: Option<Arc<CompilerController>>,
        serial: Option<Arc<SerialBuffer>>,
    ) -> Result<SimulationStatus, NodusError> {
        self.control_with_all_context(action, circuit, streamer, compiler, serial, None)
    }

    pub fn control_with_all_context(
        &self,
        action: SimulationAction,
        circuit: &ManagedCircuit,
        streamer: &Arc<FrameStreamer>,
        compiler: Option<Arc<CompilerController>>,
        serial: Option<Arc<SerialBuffer>>,
        analyzer: Option<Arc<Mutex<LogicAnalyzer>>>,
    ) -> Result<SimulationStatus, NodusError> {
        let mut slot = self
            .worker
            .lock()
            .map_err(|_| NodusError::Circuit("Simulation controller lock poisoned".into()))?;
        match action {
            SimulationAction::Start => {
                if slot.is_some() {
                    return Err(NodusError::Circuit("Simulation is already running".into()));
                }
                let (c, s) = (Arc::clone(circuit), Arc::clone(streamer));
                Self::spawn(&mut slot, c, s, compiler, serial, analyzer)
            }
            SimulationAction::Pause | SimulationAction::Resume => {
                let worker = slot
                    .as_ref()
                    .ok_or_else(|| NodusError::Circuit("Simulation is not running".into()))?;
                let cmd = if action == SimulationAction::Pause {
                    ThreadCmd::Pause
                } else {
                    ThreadCmd::Resume
                };
                send_sync(&worker.tx, cmd)
            }
            SimulationAction::Stop | SimulationAction::Reset => {
                Self::stop_worker(&mut slot, circuit, streamer)
            }
        }
    }

    pub fn set_control_value(
        &self,
        instance_id: String,
        control_id: String,
        value: f64,
    ) -> Result<SimulationStatus, NodusError> {
        let slot = self
            .worker
            .lock()
            .map_err(|_| NodusError::Circuit("Simulation controller lock poisoned".into()))?;
        let worker = slot
            .as_ref()
            .ok_or_else(|| NodusError::Circuit("Simulation is not running".into()))?;
        send_sync(&worker.tx, |reply| {
            ThreadCmd::SetControl(instance_id, control_id, value, reply)
        })
    }

    pub fn is_active(&self) -> bool {
        self.worker.lock().is_ok_and(|slot| slot.is_some())
    }

    fn spawn(
        slot: &mut Option<Worker>,
        circuit: ManagedCircuit,
        streamer: Arc<FrameStreamer>,
        compiler: Option<Arc<CompilerController>>,
        serial: Option<Arc<SerialBuffer>>,
        analyzer: Option<Arc<Mutex<LogicAnalyzer>>>,
    ) -> Result<SimulationStatus, NodusError> {
        let ((cmd_tx, cmd_rx), (init_tx, init_rx)) = (bounded(16), bounded(1));
        let handle = thread::Builder::new()
            .name("nodus-sim".into())
            .spawn(move || {
                run_thread(
                    circuit, streamer, compiler, serial, analyzer, cmd_rx, init_tx,
                )
            })
            .map_err(|err| NodusError::Worker(err.to_string()))?;
        let status = init_rx
            .recv()
            .map_err(|_| NodusError::Worker("Simulation thread exited during start".into()))
            .and_then(|res| res);
        match status {
            Ok(status) => {
                *slot = Some(Worker { tx: cmd_tx, handle });
                Ok(status)
            }
            Err(err) => {
                let _ = handle.join();
                Err(err)
            }
        }
    }

    fn stop_worker(
        slot: &mut Option<Worker>,
        circuit: &ManagedCircuit,
        streamer: &FrameStreamer,
    ) -> Result<SimulationStatus, NodusError> {
        if let Some(worker) = slot.take() {
            let _ = worker.tx.send(ThreadCmd::Shutdown);
            let _ = worker.handle.join();
        }
        let mut guard = circuit
            .lock()
            .map_err(|_| NodusError::Circuit("Circuit state lock is poisoned".into()))?;
        guard.reset_all_pins();
        let _ = streamer.publish(&SimulationFrame {
            seq: streamer.next_seq(),
            run_state: RunState::Stopped,
            sim_time: SimTime::ZERO,
            speed_ratio: 1.0,
            instances: Vec::new(),
            nets: guard.resolve_nets(),
        });
        Ok(SimulationStatus {
            state: RunState::Stopped,
            sim_time_ns: 0,
            diagnostics: Vec::new(),
        })
    }
}

fn send_sync<F>(tx: &Sender<ThreadCmd>, make: F) -> Result<SimulationStatus, NodusError>
where
    F: FnOnce(Sender<Result<SimulationStatus, NodusError>>) -> ThreadCmd,
{
    let (reply_tx, reply_rx) = bounded(1);
    tx.send(make(reply_tx))
        .map_err(|_| NodusError::Worker("Simulation thread is not responding".into()))?;
    reply_rx
        .recv()
        .map_err(|_| NodusError::Worker("Simulation thread dropped reply".into()))?
}

#[cfg(test)]
#[path = "simulation_tests.rs"]
mod tests;
