use super::*;
use crate::sandbox::pkg_loader::{scan_registry, RegistryCatalog};
use serde_json::{json, Value};
use std::path::Path;

pub fn catalog(name: &str) -> RegistryCatalog {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let catalog = scan_registry(&root).unwrap();
    assert!(catalog.diagnostics.is_empty(), "{:?}", catalog.diagnostics);
    assert_eq!(catalog.packages.len(), 1);
    catalog
}

pub fn state() -> CircuitState {
    let mut state = CircuitState::default();
    state.register_catalog(&catalog("catalog"));
    state
}

pub fn node(index: usize) -> String {
    format!("node-{index:08x}-0000-0000-0000-000000000000")
}

pub fn wire(index: usize) -> String {
    format!("wire-{index:08x}-0000-0000-0000-000000000000")
}

pub fn endpoint(index: usize, pin: &str) -> Endpoint {
    Endpoint {
        instance_id: node(index),
        pin_id: pin.into(),
    }
}

pub fn add(index: usize) -> CircuitEdit {
    CircuitEdit::AddInstance {
        instance_id: node(index),
        component_id: "test-part".into(),
        component_version: "1.0.0".into(),
    }
}

pub fn connect(index: usize, a: Endpoint, b: Endpoint) -> CircuitEdit {
    CircuitEdit::Connect {
        wire_id: wire(index),
        a,
        b,
        color: WireColor::Green,
    }
}

pub fn populated(count: usize) -> CircuitState {
    let mut state = state();
    for index in 0..count {
        state.edit(add(index)).unwrap();
    }
    state
}

pub fn apply_json(state: &mut CircuitState, value: Value) -> Result<CircuitSnapshot, String> {
    let edit = serde_json::from_value(value).map_err(|error| error.to_string())?;
    state.edit(edit).map_err(|error| error.to_string())
}

pub fn rejected(state: &mut CircuitState, value: Value) {
    let before = state.snapshot();
    assert!(
        apply_json(state, value.clone()).is_err(),
        "accepted: {value}"
    );
    assert_eq!(
        state.snapshot(),
        before,
        "failed edit changed the graph: {value}"
    );
}

pub fn connect_json() -> Value {
    json!({"type":"connect", "wire_id":wire(0), "a":endpoint(0,"A"),
        "b":endpoint(1,"z"), "color":"green"})
}
