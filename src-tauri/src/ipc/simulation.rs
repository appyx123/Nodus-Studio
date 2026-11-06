use super::circuit::ManagedCircuit;
use super::frame_streamer::{
    FrameStreamer, InstanceVisualSnapshot, SimulationFrame, TARGET_FRAME_INTERVAL,
};
use crate::compiler::CompilerController;
use crate::engine::bus::SerialBuffer;
use crate::engine::{CircuitState, RunState, SimTime, SimulationRuntime};
use crate::mcu::{AvrUno, Microcontroller};
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

type Reply = Sender<Result<SimulationStatus, NodusError>>;
enum ThreadCmd {
    Pause(Reply),
    Resume(Reply),
    SetControl(String, String, f64, Reply),
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
                Self::spawn(&mut slot, c, s, compiler, serial)
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
    ) -> Result<SimulationStatus, NodusError> {
        let ((cmd_tx, cmd_rx), (init_tx, init_rx)) = (bounded(16), bounded(1));
        let handle = thread::Builder::new()
            .name("nodus-sim".into())
            .spawn(move || run_thread(circuit, streamer, compiler, serial, cmd_rx, init_tx))
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

fn status_of(runtime: &SimulationRuntime, state: RunState) -> SimulationStatus {
    let sim_time_ns = runtime.now().as_ns_floor();
    let diagnostics = runtime.diagnostics().to_vec();
    SimulationStatus {
        state,
        sim_time_ns,
        diagnostics,
    }
}

fn emit_frame(
    runtime: &SimulationRuntime,
    circuit: &CircuitState,
    streamer: &FrameStreamer,
    run_state: RunState,
) {
    let instances = runtime
        .instance_states()
        .map(|(id, faulted, v)| InstanceVisualSnapshot {
            instance_id: id.to_owned(),
            faulted,
            visuals: v.clone(),
        })
        .collect();
    let frame = SimulationFrame {
        seq: streamer.next_seq(),
        run_state,
        sim_time: runtime.now(),
        speed_ratio: 1.0,
        instances,
        nets: circuit.resolve_nets(),
    };
    let _ = streamer.publish(&frame);
}

fn attach_firmware(
    rt: &mut SimulationRuntime,
    guard: &mut CircuitState,
    compiler: Option<&Arc<CompilerController>>,
) {
    let Some(art) = compiler.and_then(|c| c.latest_artifact()) else {
        return;
    };
    for prog in guard.instance_programs() {
        if prog.manifest.component.id == "mcu-arduino-uno" {
            let mut uno = AvrUno::new();
            let ok = if !art.hex.is_empty() {
                uno.load_hex(&art.hex).is_ok()
            } else {
                uno.load_elf(&art.elf).is_ok()
            };
            if ok {
                let _ = rt.attach_mcu(prog.instance_id, Box::new(uno), guard);
            }
        }
    }
}

fn run_thread(
    circuit: ManagedCircuit,
    streamer: Arc<FrameStreamer>,
    compiler: Option<Arc<CompilerController>>,
    serial: Option<Arc<SerialBuffer>>,
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
                attach_firmware(&mut rt, &mut guard, compiler.as_ref());
                let _ = rt.advance_with_serial(&mut guard, SimTime::ZERO, serial.as_deref());
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
            Ok(ThreadCmd::SetControl(id, cid, val, reply)) => {
                let res = if let Ok(mut guard) = circuit.lock() {
                    let r = runtime
                        .set_control(&mut guard, &id, &cid, val)
                        .and_then(|()| {
                            runtime.advance_with_serial(
                                &mut guard,
                                SimTime::ZERO,
                                serial.as_deref(),
                            )
                        });
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
                        let _ = runtime.advance_with_serial(&mut guard, step, serial.as_deref());
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
        let id = |n| format!("node-00000000-0000-0000-0000-00000000000{n}");
        let (pwr, btn, led) = (id(1), id(2), id(3));
        let mut g = circuit.lock().unwrap();
        for (inst, comp) in [
            (&pwr, "basic-power-source"),
            (&btn, "basic-pushbutton"),
            (&led, "basic-led"),
        ] {
            let add = CircuitEdit::AddInstance {
                instance_id: inst.clone(),
                component_id: comp.into(),
                component_version: "1.0.0".into(),
            };
            g.edit(add).unwrap();
        }
        let ep = |i: &String, p: &str| Endpoint {
            instance_id: i.clone(),
            pin_id: p.into(),
        };
        for (idx, a, b) in [
            (1, ep(&pwr, "VCC"), ep(&btn, "IN")),
            (2, ep(&btn, "OUT"), ep(&led, "A")),
            (3, ep(&led, "K"), ep(&pwr, "GND")),
        ] {
            let wid = format!("wire-00000000-0000-0000-0000-00000000000{idx}");
            let conn = CircuitEdit::Connect {
                wire_id: wid,
                a,
                b,
                color: WireColor::Green,
            };
            g.edit(conn).unwrap();
        }
        drop(g);

        let streamer = Arc::new(FrameStreamer::default());
        let ctrl = SimulationController::default();
        let st = |a| ctrl.control(a, &circuit, &streamer).map(|s| s.state);
        assert!(!ctrl.is_active());
        assert_eq!(st(SimulationAction::Start).unwrap(), RunState::Running);
        assert!(ctrl.is_active());
        assert!(st(SimulationAction::Start).is_err());

        ctrl.set_control_value(btn, "pressed".into(), 1.0).unwrap();
        let frame = decode_frame(&streamer.read_latest_bytes().unwrap()).unwrap();
        let led_inst = frame.instances.iter().find(|i| i.instance_id == led);
        assert!((led_inst.unwrap().visuals["brightness"] - 1.0).abs() < 1e-6);
        assert_eq!(st(SimulationAction::Pause).unwrap(), RunState::Paused);
        assert_eq!(st(SimulationAction::Resume).unwrap(), RunState::Running);
        assert_eq!(st(SimulationAction::Reset).unwrap(), RunState::Stopped);
        assert!(!ctrl.is_active());
    }
}
