use super::*;
use crate::engine::{Endpoint, WireColor};
use crate::sandbox::pkg_loader::scan_registry;
use std::path::Path;
use std::sync::Arc;

fn catalog(name: &str) -> RegistryCatalog {
    let catalog = scan_registry(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap();
    assert!(catalog.diagnostics.is_empty());
    assert_eq!(catalog.packages.len(), 1);
    catalog
}

fn add(index: usize, version: &str) -> CircuitEdit {
    CircuitEdit::AddInstance {
        instance_id: format!("node-{index:08x}-0000-0000-0000-000000000000"),
        component_id: "test-part".into(),
        component_version: version.into(),
    }
}

#[test]
fn successful_scan_registers_before_catalog_is_returned() {
    let state = ManagedCircuit::default();
    assert!(snapshot(&state).unwrap().instances.is_empty());
    assert!(edit(&state, add(0, "1.0.0")).is_err());
    let result = register_scan(&state, Ok(catalog("catalog"))).unwrap();
    assert_eq!(result.packages[0].manifest.component.version, "1.0.0");
    assert_eq!(edit(&state, add(0, "1.0.0")).unwrap().instances.len(), 1);
}

#[test]
fn failed_scan_preserves_error_latest_catalog_and_existing_topology() {
    let state = ManagedCircuit::default();
    register_scan(&state, Ok(catalog("catalog"))).unwrap();
    edit(&state, add(0, "1.0.0")).unwrap();
    register_scan(&state, Ok(catalog("reloaded"))).unwrap();
    let before = snapshot(&state).unwrap();
    let error = scan_registry(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/nonexistent-registry"),
    )
    .unwrap_err();
    match register_scan(&state, Err(NodusError::PackageDiscovery(error.clone()))) {
        Err(NodusError::PackageDiscovery(message)) => assert_eq!(message, error),
        result => panic!("discovery error changed: {result:?}"),
    }
    assert_eq!(snapshot(&state).unwrap(), before);
    assert!(edit(&state, add(1, "1.0.0")).is_err());
    edit(&state, add(1, "2.0.0")).unwrap();
    let error = register_scan(&state, Err(NodusError::Worker("join failure".into()))).unwrap_err();
    assert!(matches!(error,NodusError::Worker(message) if message == "join failure"));
}

#[test]
fn successful_empty_catalog_replaces_add_definitions_but_keeps_session_graph() {
    let state = ManagedCircuit::default();
    register_scan(&state, Ok(catalog("catalog"))).unwrap();
    edit(&state, add(0, "1.0.0")).unwrap();
    let before = snapshot(&state).unwrap();
    register_scan(
        &state,
        Ok(RegistryCatalog {
            packages: vec![],
            diagnostics: vec![],
        }),
    )
    .unwrap();
    assert_eq!(snapshot(&state).unwrap(), before);
    assert!(edit(&state, add(1, "1.0.0")).is_err());
    edit(
        &state,
        CircuitEdit::Connect {
            wire_id: "wire-00000000-0000-0000-0000-000000000000".into(),
            a: Endpoint {
                instance_id: before.instances[0].instance_id.clone(),
                pin_id: "A".into(),
            },
            b: Endpoint {
                instance_id: before.instances[0].instance_id.clone(),
                pin_id: "z".into(),
            },
            color: WireColor::Amber,
        },
    )
    .unwrap();
}

#[test]
fn poisoned_mutex_returns_circuit_error_and_preserves_prior_scan_errors() {
    let state = ManagedCircuit::default();
    let _ = std::panic::catch_unwind(|| {
        let _guard = state.lock().unwrap();
        panic!("poison fixture");
    });
    for result in [snapshot(&state), edit(&state, add(0, "1.0.0"))] {
        assert!(matches!(result,Err(NodusError::Circuit(message)) if message.contains("poisoned")));
    }
    assert!(
        matches!(register_scan(&state,Ok(catalog("catalog"))),Err(NodusError::Circuit(message)) if message.contains("poisoned"))
    );
    assert!(
        matches!(register_scan(&state,Err(NodusError::Worker("original".into()))),Err(NodusError::Worker(message)) if message == "original")
    );
}

#[test]
fn concurrent_duplicate_adds_are_serialized_without_lost_edits() {
    let state = Arc::new(ManagedCircuit::default());
    register_scan(&state, Ok(catalog("catalog"))).unwrap();
    let handles: Vec<_> = (0..12)
        .map(|_| {
            let state = Arc::clone(&state);
            std::thread::spawn(move || edit(&state, add(0, "1.0.0")).is_ok())
        })
        .collect();
    let successes = handles
        .into_iter()
        .map(|handle| usize::from(handle.join().unwrap()))
        .sum::<usize>();
    assert_eq!(successes, 1);
    let first = snapshot(&state).unwrap();
    assert_eq!(first.instances.len(), 1);
    // A new page reads the same managed session; snapshots are detached values.
    let mut page = snapshot(&state).unwrap();
    page.instances.clear();
    assert_eq!(snapshot(&state).unwrap(), first);
}
