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
    assert_eq!(st(SimulationAction::Stop).unwrap(), RunState::Stopped);
    assert!(!ctrl.is_active());
}
