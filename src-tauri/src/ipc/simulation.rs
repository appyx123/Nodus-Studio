use super::circuit::ManagedCircuit;
use super::frame_streamer::{
    FrameStreamer, InstanceVisualSnapshot, SimulationFrame, TARGET_FRAME_INTERVAL,
};
use crate::engine::{CircuitState, RunState, SimTime, SimulationRuntime};
use crate::sandbox::lua_host::InstanceDiagnostic;
use crate::NodusError;
use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
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

enum ThreadCmd {
    Pause(Sender<Result<SimulationStatus, NodusError>>),
    Resume(Sender<Result<SimulationStatus, NodusError>>),
    SetControl {
        instance_id: String,
        control_id: String,
        value: f64,
        reply: Sender<Result<SimulationStatus, NodusError>>,
    },
    Shutdown,
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
        let mut slot = self
            .worker
            .lock()
            .map_err(|_| NodusError::Circuit("Simulation controller lock poisoned".into()))?;
        match action {
            SimulationAction::Start => {
                if slot.is_some() {
                    return Err(NodusError::Circuit("Simulation is already running".into()));
                }
                Self::spawn(&mut slot, Arc::clone(circuit), Arc::clone(streamer))
            }
            SimulationAction::Pause => {
                let Some(worker) = slot.as_ref() else {
                    return Err(NodusError::Circuit("Simulation is not running".into()));
                };
                send_sync(&worker.tx, ThreadCmd::Pause)
            }
            SimulationAction::Resume => {
                let Some(worker) = slot.as_ref() else {
                    return Err(NodusError::Circuit("Simulation is not running".into()));
                };
                send_sync(&worker.tx, ThreadCmd::Resume)
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
        let Some(worker) = slot.as_ref() else {
            return Err(NodusError::Circuit("Simulation is not running".into()));
        };
        send_sync(&worker.tx, |reply| ThreadCmd::SetControl {
            instance_id,
            control_id,
            value,
            reply,
        })
    }

    pub fn is_active(&self) -> bool {
        self.worker.lock().is_ok_and(|slot| slot.is_some())
    }

    fn spawn(
        slot: &mut Option<Worker>,
        circuit: ManagedCircuit,
        streamer: Arc<FrameStreamer>,
    ) -> Result<SimulationStatus, NodusError> {
        let (cmd_tx, cmd_rx) = bounded(16);
        let (init_tx, init_rx) = bounded(1);
        let handle = thread::Builder::new()
            .name("nodus-sim".into())
            .spawn(move || run_thread(circuit, streamer, cmd_rx, init_tx))
            .map_err(|err| NodusError::Worker(err.to_string()))?;
        match init_rx.recv() {
            Ok(Ok(status)) => {
                *slot = Some(Worker { tx: cmd_tx, handle });
                Ok(status)
            }
            Ok(Err(err)) => {
                let _ = handle.join();
                Err(err)
            }
            Err(_) => {
                let _ = handle.join();
                Err(NodusError::Worker(
                    "Simulation thread exited during start".into(),
                ))
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

fn status_of(runtime: &SimulationRuntime, state: RunState) -> SimulationStatus {
    SimulationStatus {
        state,
        sim_time_ns: runtime.now().as_ns_floor(),
        diagnostics: runtime.diagnostics().to_vec(),
    }
}

fn emit_frame(
    runtime: &SimulationRuntime,
    circuit: &CircuitState,
    streamer: &FrameStreamer,
    state: RunState,
) {
    let instances = runtime
        .instance_states()
        .map(|(id, faulted, visuals)| InstanceVisualSnapshot {
            instance_id: id.to_owned(),
            faulted,
            visuals: visuals.clone(),
        })
        .collect();
    let _ = streamer.publish(&SimulationFrame {
        seq: streamer.next_seq(),
        run_state: state,
        sim_time: runtime.now(),
        speed_ratio: 1.0,
        instances,
        nets: circuit.resolve_nets(),
    });
}

fn run_thread(
    circuit: ManagedCircuit,
    streamer: Arc<FrameStreamer>,
    rx: Receiver<ThreadCmd>,
    init: Sender<Result<SimulationStatus, NodusError>>,
) {
    let mut runtime = {
        let Ok(mut guard) = circuit.lock() else {
            let _ = init.send(Err(NodusError::Circuit("Circuit lock poisoned".into())));
            return;
        };
        match SimulationRuntime::start(&mut guard) {
            Ok(mut rt) => {
                let _ = rt.advance(&mut guard, SimTime::ZERO);
                emit_frame(&rt, &guard, &streamer, RunState::Running);
                let _ = init.send(Ok(status_of(&rt, RunState::Running)));
                rt
            }
            Err(err) => {
                let _ = init.send(Err(err));
                return;
            }
        }
    };
    let step = SimTime::from_ns(TARGET_FRAME_INTERVAL.as_nanos() as u64).unwrap_or(SimTime::ZERO);
    let mut state = RunState::Running;
    loop {
        match rx.recv_timeout(TARGET_FRAME_INTERVAL) {
            Ok(ThreadCmd::Pause(reply)) => {
                let res = runtime.pause().map(|()| {
                    state = RunState::Paused;
                    if let Ok(guard) = circuit.lock() {
                        emit_frame(&runtime, &guard, &streamer, state);
                    }
                    status_of(&runtime, state)
                });
                let _ = reply.send(res);
            }
            Ok(ThreadCmd::Resume(reply)) => {
                let res = runtime.resume().map(|()| {
                    state = RunState::Running;
                    if let Ok(guard) = circuit.lock() {
                        emit_frame(&runtime, &guard, &streamer, state);
                    }
                    status_of(&runtime, state)
                });
                let _ = reply.send(res);
            }
            Ok(ThreadCmd::SetControl {
                instance_id,
                control_id,
                value,
                reply,
            }) => {
                let res = if let Ok(mut guard) = circuit.lock() {
                    let r = runtime
                        .set_control(&mut guard, &instance_id, &control_id, value)
                        .and_then(|()| runtime.advance(&mut guard, SimTime::ZERO));
                    emit_frame(&runtime, &guard, &streamer, state);
                    r.map(|()| status_of(&runtime, state))
                } else {
                    Err(NodusError::Circuit("Circuit lock poisoned".into()))
                };
                let _ = reply.send(res);
            }
            Ok(ThreadCmd::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                if state == RunState::Running {
                    if let Ok(mut guard) = circuit.lock() {
                        let _ = runtime.advance(&mut guard, step);
                        emit_frame(&runtime, &guard, &streamer, state);
                    }
                }
            }
        }
    }
    if let Ok(mut guard) = circuit.lock() {
        runtime.shutdown(&mut guard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{CircuitEdit, Endpoint, WireColor};
    use crate::ipc::frame_streamer::decode_frame;
    use crate::sandbox::pkg_loader::scan_registry;
    use std::path::Path;

    #[test]
    fn controller_lifecycle_leaves_no_stale_thread_or_pin_state() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../components-registry");
        let catalog = scan_registry(&root).unwrap();
        let circuit = ManagedCircuit::default();
        circuit.lock().unwrap().register_catalog(&catalog);
        let pwr = "node-00000000-0000-0000-0000-000000000001".to_string();
        let btn = "node-00000000-0000-0000-0000-000000000002".to_string();
        let led = "node-00000000-0000-0000-0000-000000000003".to_string();
        for (id, comp) in [
            (&pwr, "basic-power-source"),
            (&btn, "basic-pushbutton"),
            (&led, "basic-led"),
        ] {
            circuit
                .lock()
                .unwrap()
                .edit(CircuitEdit::AddInstance {
                    instance_id: id.clone(),
                    component_id: comp.into(),
                    component_version: "1.0.0".into(),
                })
                .unwrap();
        }
        for (idx, a, ap, b, bp) in [
            (1, &pwr, "VCC", &btn, "IN"),
            (2, &btn, "OUT", &led, "A"),
            (3, &led, "K", &pwr, "GND"),
        ] {
            circuit
                .lock()
                .unwrap()
                .edit(CircuitEdit::Connect {
                    wire_id: format!("wire-00000000-0000-0000-0000-00000000000{idx}"),
                    a: Endpoint {
                        instance_id: a.clone(),
                        pin_id: ap.into(),
                    },
                    b: Endpoint {
                        instance_id: b.clone(),
                        pin_id: bp.into(),
                    },
                    color: WireColor::Green,
                })
                .unwrap();
        }

        let streamer = Arc::new(FrameStreamer::default());
        let ctrl = SimulationController::default();
        assert!(!ctrl.is_active());

        let s = ctrl
            .control(SimulationAction::Start, &circuit, &streamer)
            .unwrap();
        assert_eq!(s.state, RunState::Running);
        assert!(ctrl.is_active());
        assert!(ctrl
            .control(SimulationAction::Start, &circuit, &streamer)
            .is_err());

        ctrl.set_control_value(btn, "pressed".into(), 1.0).unwrap();
        let frame = decode_frame(&streamer.read_latest_bytes().unwrap()).unwrap();
        let b = frame
            .instances
            .iter()
            .find(|i| i.instance_id == led)
            .unwrap()
            .visuals["brightness"];
        assert!((b - 1.0).abs() < 1e-6);

        assert_eq!(
            ctrl.control(SimulationAction::Pause, &circuit, &streamer)
                .unwrap()
                .state,
            RunState::Paused
        );
        assert_eq!(
            ctrl.control(SimulationAction::Resume, &circuit, &streamer)
                .unwrap()
                .state,
            RunState::Running
        );
        assert_eq!(
            ctrl.control(SimulationAction::Reset, &circuit, &streamer)
                .unwrap()
                .state,
            RunState::Stopped
        );
        assert!(!ctrl.is_active());
    }
}
