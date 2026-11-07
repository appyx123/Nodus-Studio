use super::circuit::ManagedCircuit;
use super::frame_streamer::{
    FrameStreamer, InstanceVisualSnapshot, SimulationFrame, TARGET_FRAME_INTERVAL,
};
use super::simulation::SimulationStatus;
use crate::compiler::CompilerController;
use crate::engine::bus::SerialBuffer;
use crate::engine::logic_analyzer::LogicAnalyzer;
use crate::engine::{CircuitState, RunState, SimTime, SimulationRuntime};
use crate::mcu::{AvrUno, Microcontroller};
use crate::NodusError;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};

pub type Reply = Sender<Result<SimulationStatus, NodusError>>;

pub enum ThreadCmd {
    Pause(Reply),
    Resume(Reply),
    SetControl(String, String, f64, Reply),
    Shutdown,
}

pub fn status_of(runtime: &SimulationRuntime, state: RunState) -> SimulationStatus {
    let sim_time_ns = runtime.now().as_ns_floor();
    let diagnostics = runtime.diagnostics().to_vec();
    SimulationStatus {
        state,
        sim_time_ns,
        diagnostics,
    }
}

pub fn emit_frame(
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
        if prog.manifest.mcu.is_some() {
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

pub fn run_thread(
    circuit: ManagedCircuit,
    streamer: Arc<FrameStreamer>,
    compiler: Option<Arc<CompilerController>>,
    serial: Option<Arc<SerialBuffer>>,
    analyzer: Option<Arc<Mutex<LogicAnalyzer>>>,
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
                if let Some(la) = analyzer {
                    rt.attach_logic_analyzer(la);
                }
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
