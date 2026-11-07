use super::persistence::*;
use super::types::*;
use crate::engine::circuit_graph::CircuitState;
use crate::engine::WireColor;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn sample_project() -> NodusProjectFile {
    let mut controls = BTreeMap::new();
    controls.insert("slider".to_string(), 0.75);

    NodusProjectFile {
        format: NODUS_PROJECT_FORMAT.into(),
        version: CURRENT_SCHEMA_VERSION.into(),
        app_id: "org.nodus.Studio".into(),
        metadata: ProjectMetadata {
            name: "Test Circuit".into(),
            created_at: 1000,
            modified_at: 2000,
            description: Some("Test project description".into()),
        },
        viewport: ViewportSection {
            pan: Point { x: 50.0, y: -20.0 },
            zoom: 1.25,
        },
        circuit: CircuitSection {
            instances: vec![
                InstanceRecord {
                    instance_id: "node-1".into(),
                    component_id: "basic-led".into(),
                    component_version: "1.0.0".into(),
                    position: Point { x: 100.0, y: 200.0 },
                    control_values: controls,
                },
                InstanceRecord {
                    instance_id: "node-2".into(),
                    component_id: "basic-power-source".into(),
                    component_version: "1.0.0".into(),
                    position: Point { x: 300.0, y: 200.0 },
                    control_values: BTreeMap::new(),
                },
            ],
            wires: vec![WireRecord {
                wire_id: "wire-1".into(),
                a: EndpointRecord {
                    instance_id: "node-1".into(),
                    pin_id: "anode".into(),
                },
                b: EndpointRecord {
                    instance_id: "node-2".into(),
                    pin_id: "vcc".into(),
                },
                color: WireColor::Green,
            }],
        },
        firmware: FirmwareSection {
            sketch_source: "void setup() { pinMode(13, OUTPUT); } void loop() {}".into(),
            fqbn: "arduino:avr:uno".into(),
        },
        mqtt_bridge: Some(MqttBridgeSection {
            enabled: true,
            host: "broker.emqx.io".into(),
            port: 1883,
            client_id: "nodus-test-client".into(),
            sub_topics: vec!["nodus/in".into()],
            pub_topics: vec!["nodus/out".into()],
        }),
    }
}

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nodus_proj_test_{}_{}", name, std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn project_save_and_load_round_trip() {
    let dir = test_dir("round_trip");
    let file_path = dir.join("test_circuit.nodus");
    let original = sample_project();

    let save_res = save_project_atomic(&file_path, &original);
    assert!(save_res.is_ok());

    let mut circuit = CircuitState::default();
    let load_res = load_project_from_path(&file_path, &mut circuit);
    assert!(load_res.is_ok());

    let response = load_res.unwrap();
    assert_eq!(response.project.metadata.name, "Test Circuit");
    assert_eq!(response.project.viewport.zoom, 1.25);
    assert_eq!(response.project.circuit.instances.len(), 2);
    assert_eq!(response.project.circuit.wires.len(), 1);
    assert_eq!(
        response.project.firmware.sketch_source,
        original.firmware.sketch_source
    );
    assert_eq!(response.project.firmware.fqbn, "arduino:avr:uno");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn corrupted_json_fails_gracefully_without_altering_circuit() {
    let dir = test_dir("corrupted");
    let file_path = dir.join("broken.nodus");
    let _ = fs::write(&file_path, b"{ broken json content !!! ");

    let mut circuit = CircuitState::default();
    let res = load_project_from_path(&file_path, &mut circuit);
    assert!(res.is_err());
    assert!(circuit.snapshot().instances.is_empty());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rejects_unsupported_major_schema_version() {
    let mut proj = sample_project();
    proj.version = "99.0.0".into();
    let bytes = serde_json::to_vec(&proj).unwrap();

    let res = parse_project_bytes(&bytes);
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("Unsupported project schema version"));
}

#[test]
fn rejects_oversized_project_file() {
    let data = vec![b'a'; MAX_PROJECT_FILE_SIZE + 1];
    let res = parse_project_bytes(&data);
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("exceeds limit"));
}

#[test]
fn missing_package_is_loaded_as_placeholder_with_warning() {
    let proj = sample_project();
    let mut circuit = CircuitState::default();

    let warnings = apply_project_to_circuit(&proj, &mut circuit);
    assert!(!warnings.is_empty());
    assert!(warnings
        .iter()
        .any(|w| w.message.contains("missing from registry")));

    let snap = circuit.snapshot();
    assert_eq!(snap.instances.len(), 2);
    assert_eq!(snap.wires.len(), 1);
}

#[test]
fn example_blink_project_parses_successfully() {
    let example_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/blink-led.nodus");
    let bytes = fs::read(&example_path).unwrap();
    let project = parse_project_bytes(&bytes).unwrap();
    assert_eq!(project.metadata.name, "Arduino Uno Blink");
    assert_eq!(project.circuit.instances.len(), 2);
    assert_eq!(project.circuit.wires.len(), 2);
    assert!(project.firmware.sketch_source.contains("void setup()"));
}
