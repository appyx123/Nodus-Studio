use super::{test_support::*, *};
use serde_json::json;

#[test]
fn initial_snapshot_is_empty_and_has_exact_top_level_keys() {
    assert_eq!(
        serde_json::to_value(CircuitState::default().snapshot()).unwrap(),
        json!({"instances":[],"wires":[],"nets":[]})
    );
}

#[test]
fn snapshot_matches_the_exact_frontend_contract_and_catalog_pin_ids() {
    let mut state = populated(2);
    let snapshot = state
        .edit(connect(0, endpoint(0, "A"), endpoint(1, "z")))
        .unwrap();
    let value = serde_json::to_value(snapshot).unwrap();
    assert_eq!(
        value["instances"][0],
        json!({"instance_id":node(0),
        "component_id":"test-part","component_version":"1.0.0","pins":["A","NC","z"]})
    );
    assert_eq!(
        value["wires"],
        json!([{"wire_id":wire(0),"a":endpoint(0,"A"),
        "b":endpoint(1,"z"),"color":"green"}])
    );
    assert_eq!(
        value["nets"][0],
        json!({"endpoints":[endpoint(0,"A"),endpoint(1,"z")]})
    );
    assert_eq!(value.as_object().unwrap().len(), 3);
}

#[test]
fn every_color_round_trips_and_recolor_changes_only_color() {
    let mut state = populated(2);
    state
        .edit(connect(0, endpoint(0, "A"), endpoint(1, "z")))
        .unwrap();
    for color in ["green", "blue", "amber", "red", "violet"] {
        let before = state.snapshot();
        let after = apply_json(
            &mut state,
            json!({"type":"set_wire_color","wire_id":wire(0),"color":color}),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&after.wires[0]).unwrap()["color"],
            color
        );
        assert_eq!(before.nets, after.nets);
        assert_eq!(before.instances, after.instances);
        assert_eq!(before.wires[0].a, after.wires[0].a);
        assert_eq!(before.wires[0].b, after.wires[0].b);
    }
}

#[test]
fn strict_requests_reject_unknown_fields_aliases_missing_values_and_colors() {
    let mut state = populated(2);
    let edits = [
        serde_json::to_value(add(2)).unwrap(),
        json!({"type":"remove_instance","instance_id":node(0)}),
        connect_json(),
        json!({"type":"remove_wire","wire_id":wire(0)}),
        json!({"type":"set_wire_color","wire_id":wire(0),"color":"blue"}),
    ];
    for edit in edits {
        for extra in ["position", "pins", "componentVersion", "extra"] {
            let mut invalid = edit.clone();
            invalid[extra] = json!(0);
            assert!(serde_json::from_value::<CircuitEdit>(invalid.clone()).is_err());
            rejected(&mut state, invalid);
        }
        for key in edit.as_object().unwrap().keys() {
            let mut invalid = edit.clone();
            invalid.as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<CircuitEdit>(invalid).is_err());
        }
    }
    for color in ["pink", "GREEN", "", "#00ff00"] {
        let mut invalid = connect_json();
        invalid["color"] = json!(color);
        rejected(&mut state, invalid);
        rejected(
            &mut state,
            json!({"type":"set_wire_color","wire_id":wire(0),"color":color}),
        );
    }
    for field in ["a", "b"] {
        let mut invalid = connect_json();
        invalid[field]["position"] = json!(0);
        rejected(&mut state, invalid);
        let mut invalid = connect_json();
        invalid[field] = json!({"instanceId":node(0),"pin_id":"A"});
        rejected(&mut state, invalid);
    }
    rejected(
        &mut state,
        json!({"type":"addInstance","instance_id":node(2),"component_id":"test-part","component_version":"1.0.0"}),
    );
}
