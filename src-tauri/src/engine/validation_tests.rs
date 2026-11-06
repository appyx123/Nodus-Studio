use super::{test_support::*, *};
use serde_json::json;

#[test]
fn add_requires_catalog_identity_version_and_unique_instance_id() {
    let mut empty = CircuitState::default();
    assert!(empty.edit(add(0)).is_err());
    let mut state = populated(1);
    rejected(&mut state, serde_json::to_value(add(0)).unwrap());
    for (field, value) in [
        ("component_id", "unknown"),
        ("component_version", "2.0.0"),
        ("component_version", "1.0"),
        ("component_version", ""),
    ] {
        let mut edit = serde_json::to_value(add(1)).unwrap();
        edit[field] = json!(value);
        rejected(&mut state, edit);
    }
}

#[test]
fn canonical_ascii_uuid_ids_are_required_without_enforcing_v4() {
    let mut state = populated(2);
    for invalid in [
        "",
        "node-1",
        "wire-00000000-0000-0000-0000-000000000000",
        "node-00000000000000000000000000000000",
        "node-0000000g-0000-0000-0000-000000000000",
        "node-00000000_0000-0000-0000-000000000000",
        "node-00000000-000-00000-0000-000000000000",
        "node-００００００００-0000-0000-0000-000000000000",
        "node-00000000-0000-0000-0000-000000000000 ",
        "node-AAAAAAAA-0000-0000-0000-000000000000",
    ] {
        let mut edit = serde_json::to_value(add(3)).unwrap();
        edit["instance_id"] = json!(invalid);
        rejected(&mut state, edit);
    }
    for invalid in [
        "wire-1",
        "wire-gggggggg-0000-0000-0000-000000000000",
        "wire-00000000000000000000000000000000",
        "node-00000000-0000-0000-0000-000000000000",
    ] {
        let mut edit = connect_json();
        edit["wire_id"] = json!(invalid);
        rejected(&mut state, edit);
    }
    state.edit(add(0xabcdef)).unwrap();
    state
        .edit(connect(0xabcdef, endpoint(0, "A"), endpoint(1, "A")))
        .unwrap();
}

#[test]
fn connect_rejects_unknown_nc_identical_and_duplicate_endpoints_atomically() {
    let mut state = populated(2);
    for endpoint in [
        endpoint(99, "A"),
        endpoint(0, "missing"),
        endpoint(0, "NC"),
        endpoint(0, "First terminal"),
    ] {
        for field in ["a", "b"] {
            let mut edit = connect_json();
            edit[field] = json!(endpoint);
            rejected(&mut state, edit);
        }
    }
    let mut identical = connect_json();
    identical["b"] = identical["a"].clone();
    rejected(&mut state, identical);
    state
        .edit(connect(0, endpoint(0, "A"), endpoint(1, "z")))
        .unwrap();
    rejected(
        &mut state,
        serde_json::to_value(connect(0, endpoint(0, "z"), endpoint(1, "A"))).unwrap(),
    );
    for (a, b) in [
        (endpoint(0, "A"), endpoint(1, "z")),
        (endpoint(1, "z"), endpoint(0, "A")),
    ] {
        rejected(&mut state, serde_json::to_value(connect(1, a, b)).unwrap());
    }
    state
        .edit(connect(1, endpoint(0, "z"), endpoint(1, "A")))
        .unwrap();
}

#[test]
fn unknown_removal_and_recolor_fail_without_changing_state() {
    let mut state = populated(2);
    for edit in [
        CircuitEdit::RemoveInstance {
            instance_id: node(99),
        },
        CircuitEdit::RemoveWire { wire_id: wire(99) },
        CircuitEdit::SetWireColor {
            wire_id: wire(99),
            color: WireColor::Red,
        },
    ] {
        rejected(&mut state, serde_json::to_value(edit).unwrap());
    }
}

#[test]
fn instance_limit_accepts_256_rejects_257_and_allows_reuse_after_delete() {
    let mut state = populated(256);
    assert_eq!(state.snapshot().instances.len(), 256);
    rejected(&mut state, serde_json::to_value(add(256)).unwrap());
    state
        .edit(CircuitEdit::RemoveInstance {
            instance_id: node(0),
        })
        .unwrap();
    state.edit(add(256)).unwrap();
    assert_eq!(state.snapshot().instances.len(), 256);
}

#[test]
fn wire_limit_accepts_1024_rejects_1025_and_allows_reuse_after_delete() {
    let mut state = populated(48);
    let mut count = 0;
    'outer: for a in 0..48 {
        for b in a + 1..48 {
            if count == 1024 {
                break 'outer;
            }
            state
                .edit(connect(count, endpoint(a, "A"), endpoint(b, "A")))
                .unwrap();
            count += 1;
        }
    }
    assert_eq!(state.snapshot().wires.len(), 1024);
    rejected(
        &mut state,
        serde_json::to_value(connect(1024, endpoint(0, "z"), endpoint(1, "z"))).unwrap(),
    );
    state
        .edit(CircuitEdit::SetWireColor {
            wire_id: wire(0),
            color: WireColor::Blue,
        })
        .unwrap();
    state
        .edit(CircuitEdit::RemoveWire { wire_id: wire(0) })
        .unwrap();
    state
        .edit(connect(1024, endpoint(0, "z"), endpoint(1, "z")))
        .unwrap();
    assert_eq!(state.snapshot().wires.len(), 1024);
}
