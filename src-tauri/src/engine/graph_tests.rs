use super::{test_support::*, *};

fn groups(state: &CircuitState) -> Vec<Vec<Endpoint>> {
    state
        .snapshot()
        .nets
        .into_iter()
        .map(|net| net.endpoints)
        .collect()
}

#[test]
fn all_declared_pins_including_nc_start_as_singleton_nets() {
    let state = populated(2);
    assert_eq!(
        groups(&state),
        (0..2)
            .flat_map(|i| ["A", "NC", "z"].map(|p| vec![endpoint(i, p)]))
            .collect::<Vec<_>>()
    );
}

#[test]
fn branching_transitive_nets_split_after_removing_a_bridge() {
    let mut state = populated(4);
    for (id, a, b) in [(0, 0, 1), (1, 1, 2), (2, 1, 3)] {
        state
            .edit(connect(id, endpoint(a, "A"), endpoint(b, "A")))
            .unwrap();
    }
    assert_eq!(
        groups(&state)[0],
        (0..4).map(|i| endpoint(i, "A")).collect::<Vec<_>>()
    );
    assert_eq!(groups(&state).len(), 9);
    state
        .edit(CircuitEdit::RemoveWire { wire_id: wire(0) })
        .unwrap();
    assert_eq!(groups(&state)[0], vec![endpoint(0, "A")]);
    assert!(groups(&state).contains(&vec![endpoint(1, "A"), endpoint(2, "A"), endpoint(3, "A")]));
    assert_eq!(groups(&state).len(), 10);
}

#[test]
fn cycles_are_allowed_and_breaking_one_edge_preserves_connectivity() {
    let mut state = populated(3);
    for (id, a, b) in [(0, 0, 1), (1, 1, 2), (2, 2, 0)] {
        state
            .edit(connect(id, endpoint(a, "A"), endpoint(b, "A")))
            .unwrap();
    }
    let before = groups(&state);
    state
        .edit(CircuitEdit::RemoveWire { wire_id: wire(1) })
        .unwrap();
    assert_eq!(groups(&state), before);
    state
        .edit(CircuitEdit::RemoveWire { wire_id: wire(0) })
        .unwrap();
    assert!(groups(&state).contains(&vec![endpoint(1, "A")]));
}

#[test]
fn same_instance_distinct_pins_connect_but_pins_are_not_implicitly_shorted() {
    let mut state = populated(2);
    state
        .edit(connect(0, endpoint(0, "A"), endpoint(0, "z")))
        .unwrap();
    assert_eq!(groups(&state)[0], vec![endpoint(0, "A"), endpoint(0, "z")]);
    assert!(groups(&state).contains(&vec![endpoint(1, "A")]));
    assert_eq!(groups(&state).len(), 5);
}

#[test]
fn removing_instance_cascades_incident_wires_and_splits_nets_only() {
    let mut state = populated(4);
    for (id, a, b) in [(0, 0, 1), (1, 1, 2), (2, 2, 3)] {
        state
            .edit(connect(id, endpoint(a, "A"), endpoint(b, "A")))
            .unwrap();
    }
    let snapshot = state
        .edit(CircuitEdit::RemoveInstance {
            instance_id: node(1),
        })
        .unwrap();
    assert_eq!(snapshot.instances.len(), 3);
    assert_eq!(
        snapshot
            .wires
            .iter()
            .map(|w| w.wire_id.clone())
            .collect::<Vec<_>>(),
        vec![wire(2)]
    );
    assert!(groups(&state).contains(&vec![endpoint(0, "A")]));
    assert!(groups(&state).contains(&vec![endpoint(2, "A"), endpoint(3, "A")]));
    assert!(snapshot
        .nets
        .iter()
        .flat_map(|n| &n.endpoints)
        .all(|e| e.instance_id != node(1)));
}

#[test]
fn snapshots_are_sorted_and_independent_of_mutation_insertion_order() {
    let mut left = populated(3);
    let mut right = state();
    for index in (0..3).rev() {
        right.edit(add(index)).unwrap();
    }
    let edits = [
        connect(1, endpoint(2, "A"), endpoint(1, "A")),
        connect(0, endpoint(0, "z"), endpoint(2, "A")),
    ];
    for edit in edits.clone() {
        left.edit(edit).unwrap();
    }
    for edit in edits.into_iter().rev() {
        right.edit(edit).unwrap();
    }
    assert_eq!(left.snapshot(), right.snapshot());
    assert_eq!(left.snapshot().wires[0].wire_id, wire(0));
    assert_eq!(left.snapshot().instances[0].instance_id, node(0));
    let before = left.snapshot();
    assert_eq!(left.snapshot(), before);
}

#[test]
fn catalog_reload_preserves_existing_versions_pin_definitions_and_wires() {
    let mut state = populated(2);
    state
        .edit(connect(0, endpoint(0, "A"), endpoint(1, "z")))
        .unwrap();
    let before = state.snapshot();
    state.register_catalog(&catalog("reloaded"));
    assert_eq!(state.snapshot(), before);
    assert!(state.edit(add(2)).is_err());
    state
        .edit(CircuitEdit::AddInstance {
            instance_id: node(2),
            component_id: "test-part".into(),
            component_version: "2.0.0".into(),
        })
        .unwrap();
    assert_eq!(state.snapshot().instances[2].pins, ["A", "B"]);
    assert!(state
        .edit(connect(1, endpoint(2, "A"), endpoint(0, "z")))
        .is_err());
    assert!(state
        .edit(connect(1, endpoint(2, "z"), endpoint(0, "z")))
        .is_err());
    state
        .edit(connect(1, endpoint(0, "z"), endpoint(1, "A")))
        .unwrap();
    state.register_catalog(&crate::sandbox::pkg_loader::RegistryCatalog {
        packages: vec![],
        diagnostics: vec![],
    });
    let before = state.snapshot();
    assert!(state.edit(add(3)).is_err());
    assert_eq!(state.snapshot(), before);
    state
        .edit(connect(2, endpoint(0, "A"), endpoint(0, "z")))
        .unwrap();
    state
        .edit(CircuitEdit::SetWireColor {
            wire_id: wire(0),
            color: WireColor::Violet,
        })
        .unwrap();
}
