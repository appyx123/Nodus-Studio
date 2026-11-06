use super::test_support::{connect, endpoint, node, wire};
use super::*;
use crate::sandbox::pkg_loader::scan_registry;
use std::path::Path;

fn registry_state() -> CircuitState {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../components-registry");
    let catalog = scan_registry(&root).unwrap();
    assert!(catalog.diagnostics.is_empty(), "{:?}", catalog.diagnostics);
    assert_eq!(catalog.packages.len(), 3);
    let mut state = CircuitState::default();
    state.register_catalog(&catalog);
    state
}

fn add_part(state: &mut CircuitState, index: usize, id: &str) {
    state
        .edit(CircuitEdit::AddInstance {
            instance_id: node(index),
            component_id: id.into(),
            component_version: "1.0.0".into(),
        })
        .unwrap();
}

fn brightness(runtime: &SimulationRuntime, id: &str) -> f64 {
    runtime
        .instance_states()
        .find(|(inst, _, _)| *inst == id)
        .and_then(|(_, _, v)| v.get("brightness").copied())
        .unwrap()
}

#[test]
fn power_pushbutton_led_circuit_responds_to_button_and_reset_clears_state() {
    let mut circuit = registry_state();
    add_part(&mut circuit, 0, "basic-power-source");
    add_part(&mut circuit, 1, "basic-pushbutton");
    add_part(&mut circuit, 2, "basic-led");

    // VCC -> button IN, button OUT -> LED A, LED K -> GND.
    circuit
        .edit(connect(0, endpoint(0, "VCC"), endpoint(1, "IN")))
        .unwrap();
    circuit
        .edit(connect(1, endpoint(1, "OUT"), endpoint(2, "A")))
        .unwrap();
    circuit
        .edit(connect(2, endpoint(2, "K"), endpoint(0, "GND")))
        .unwrap();

    let mut runtime = SimulationRuntime::start(&mut circuit).unwrap();
    runtime.advance(&mut circuit, SimTime::ZERO).unwrap();
    assert!((brightness(&runtime, &node(2)) - 0.15).abs() < 1e-6);

    runtime
        .set_control(&mut circuit, &node(1), "pressed", 1.0)
        .unwrap();
    runtime.advance(&mut circuit, SimTime::ZERO).unwrap();
    assert!((brightness(&runtime, &node(2)) - 1.0).abs() < 1e-6);

    runtime
        .set_control(&mut circuit, &node(1), "pressed", 0.0)
        .unwrap();
    runtime
        .advance(&mut circuit, SimTime::from_ns(10_000_000).unwrap())
        .unwrap();
    assert!((brightness(&runtime, &node(2)) - 0.15).abs() < 1e-6);

    // Pause blocks time; resume restores it; shutdown releases all pins.
    runtime.pause().unwrap();
    assert!(runtime.pause().is_err());
    runtime
        .set_control(&mut circuit, &node(1), "pressed", 1.0)
        .unwrap();
    runtime
        .advance(&mut circuit, SimTime::from_ns(1_000).unwrap())
        .unwrap();
    assert!((brightness(&runtime, &node(2)) - 0.15).abs() < 1e-6);
    runtime.resume().unwrap();
    runtime.advance(&mut circuit, SimTime::ZERO).unwrap();
    assert!((brightness(&runtime, &node(2)) - 1.0).abs() < 1e-6);

    runtime.shutdown(&mut circuit);
    assert_eq!(
        circuit.read_pin(&endpoint(0, "VCC")).unwrap(),
        LogicLevel::HighZ
    );

    // Fresh start after reset reinitializes the button as unpressed.
    let mut fresh = SimulationRuntime::start(&mut circuit).unwrap();
    fresh.advance(&mut circuit, SimTime::ZERO).unwrap();
    assert!((brightness(&fresh, &node(2)) - 0.15).abs() < 1e-6);
}

#[test]
fn led_glow_tracks_pwm_duty_cycle_and_pin_conflict_turns_led_off() {
    let mut circuit = registry_state();
    add_part(&mut circuit, 0, "basic-power-source");
    add_part(&mut circuit, 1, "basic-pushbutton");
    add_part(&mut circuit, 2, "basic-led");
    circuit
        .edit(connect(0, endpoint(0, "VCC"), endpoint(1, "IN")))
        .unwrap();
    circuit
        .edit(connect(1, endpoint(1, "OUT"), endpoint(2, "A")))
        .unwrap();
    circuit
        .edit(connect(2, endpoint(2, "K"), endpoint(0, "GND")))
        .unwrap();

    let mut runtime = SimulationRuntime::start(&mut circuit).unwrap();
    let half_period = SimTime::from_ns(500_000).unwrap();
    for _ in 0..6 {
        runtime
            .set_control(&mut circuit, &node(1), "pressed", 1.0)
            .unwrap();
        runtime.advance(&mut circuit, half_period).unwrap();
        runtime
            .set_control(&mut circuit, &node(1), "pressed", 0.0)
            .unwrap();
        runtime.advance(&mut circuit, half_period).unwrap();
    }
    let mid = brightness(&runtime, &node(2));
    assert!(mid > 0.45 && mid < 0.7, "expected ~50% glow, got {mid}");

    // Shorting VCC to GND creates a reported conflict and drives the net UNKNOWN.
    circuit
        .edit(CircuitEdit::Connect {
            wire_id: wire(3),
            a: endpoint(0, "VCC"),
            b: endpoint(0, "GND"),
            color: WireColor::Red,
        })
        .unwrap();
    runtime
        .set_control(&mut circuit, &node(1), "pressed", 1.0)
        .unwrap();
    runtime
        .advance(&mut circuit, SimTime::from_ns(10_000_000).unwrap())
        .unwrap();
    assert!(circuit.resolve_nets().iter().any(|net| net.conflict));
    assert!((brightness(&runtime, &node(2)) - 0.15).abs() < 1e-6);
}
