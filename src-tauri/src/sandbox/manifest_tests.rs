use super::*;

pub(crate) const MANIFEST: &str = r#"
schema_version = 1
[compatibility]
engine_api = 1
[component]
id = "basic-led"
name = "LED"
category = "Output"
author = "Nodus"
version = "1.0.0"
description = "A test LED"
[electrical]
operating_voltage_min = 0.0
operating_voltage_max = 5.0
[[pins]]
id = "A"
name = "Anode"
type = "input"
svg_anchor = "pin-a"
[[controls]]
id = "brightness"
label = "Brightness"
type = "slider"
min = 0.0
max = 1.0
step = 0.1
default = 0.5
[[visuals]]
id = "brightness"
type = "number"
target = "dyn-light"
property = "opacity"
min = 0.0
max = 1.0
default = 0.0
"#;

#[test]
fn accepts_valid_manifest() {
    let manifest = parse_manifest(MANIFEST).unwrap();
    assert!(manifest.validate("basic-led", &svg_ids()).is_ok());
}

fn svg_ids() -> HashSet<String> {
    ["pin-a", "dyn-light"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn rejects_versions_ids_metadata_and_invalid_numbers() {
    for (from, to) in [
        ("schema_version = 1", "schema_version = 2"),
        ("engine_api = 1", "engine_api = 0"),
        ("version = \"1.0.0\"", "version = \"1.0\""),
        ("version = \"1.0.0\"", "version = \"01.0.0\""),
        ("version = \"1.0.0\"", "version = \" 1.0.0\""),
        ("id = \"basic-led\"", "id = \"../basic-led\""),
        ("id = \"basic-led\"", "id = \"Basic-led\""),
        ("id = \"basic-led\"", "id = \"basic--led\""),
        ("id = \"basic-led\"", "id = \"-basic-led\""),
        ("name = \"LED\"", "name = \"  \""),
        ("id = \"A\"", "id = \"1A\""),
        ("id = \"A\"", "id = \"Å\""),
        ("type = \"input\"", "type = \"analog\""),
        ("type = \"slider\"", "type = \"button\""),
        ("type = \"number\"", "type = \"boolean\""),
        ("property = \"opacity\"", "property = \"style\""),
        (
            "operating_voltage_min = 0.0",
            "operating_voltage_min = -1.0",
        ),
        ("operating_voltage_min = 0.0", "operating_voltage_min = 6.0"),
        (
            "operating_voltage_max = 5.0",
            "operating_voltage_max = 1000.1",
        ),
        ("operating_voltage_max = 5.0", "operating_voltage_max = inf"),
        ("operating_voltage_min = 0.0", "operating_voltage_min = nan"),
        ("step = 0.1", "step = 0.0"),
        ("step = 0.1", "step = -0.1"),
        ("step = 0.1", "step = nan"),
        ("default = 0.5", "default = 2.0"),
        ("default = 0.5", "default = -inf"),
        ("default = 0.0", "default = nan"),
        ("default = 0.0", "default = -0.1"),
        ("target = \"dyn-light\"", "target = \"pin-a\""),
        ("svg_anchor = \"pin-a\"", "svg_anchor = \"dyn-light\""),
    ] {
        assert!(
            parse_manifest(&MANIFEST.replace(from, to)).is_err(),
            "accepted {to}"
        );
    }
    assert!(parse_manifest(&MANIFEST.replace("max = 1.0", "max = 1.1")).is_err());
    assert!(
        parse_manifest(&MANIFEST.replace("min = 0.0\nmax = 1.0", "min = -0.1\nmax = 1.0")).is_err()
    );
    let version = MANIFEST.replace("version = \"1.0.0\"", "version = \"1.2.3-beta.1+build.7\"");
    assert!(parse_manifest(&version).is_ok());
}

#[test]
fn rejects_unknown_fields_at_every_object_depth_and_duplicate_toml_keys() {
    for needle in [
        "schema_version = 1",
        "engine_api = 1",
        "name = \"LED\"",
        "operating_voltage_max = 5.0",
        "svg_anchor = \"pin-a\"",
        "step = 0.1",
        "property = \"opacity\"",
    ] {
        let source = MANIFEST.replace(needle, &format!("{needle}\nunknown = true"));
        assert!(
            parse_manifest(&source).is_err(),
            "accepted extra field after {needle}"
        );
    }
    assert!(parse_manifest(
        &MANIFEST.replace("name = \"LED\"", "name = \"LED\"\nname = \"Other\"")
    )
    .is_err());
    let json = serde_json::to_value(parse_manifest(MANIFEST).unwrap()).unwrap();
    for field in ["compatibility", "component", "electrical"] {
        let mut value = json.clone();
        value[field]["extra"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Manifest>(value).is_err());
    }
    for field in ["pins", "controls", "visuals"] {
        let mut value = json.clone();
        value[field][0]["extra"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Manifest>(value).is_err());
    }
}

#[test]
fn checks_id_scopes_and_rejects_duplicate_ids_anchors_and_bindings() {
    // Controls and visuals intentionally share an ID in the valid fixture.
    for section in ["pins", "controls", "visuals"] {
        let start = MANIFEST.find(&format!("[[{section}]]")).unwrap();
        let end = MANIFEST[start + 2..]
            .find("[[")
            .map_or(MANIFEST.len(), |i| start + 2 + i);
        let block = &MANIFEST[start..end];
        assert!(parse_manifest(&format!("{MANIFEST}\n{block}")).is_err());
        if section != "controls" {
            let unique = block
                .replace("id = \"A\"", "id = \"B\"")
                .replace("id = \"brightness\"", "id = \"other\"");
            assert!(parse_manifest(&format!("{MANIFEST}\n{unique}")).is_err());
        }
    }
    let second_pin =
        "\n[[pins]]\nid = \"a\"\nname = \"Other\"\ntype = \"nc\"\nsvg_anchor = \"pin-b\"\n";
    assert!(parse_manifest(&format!("{MANIFEST}{second_pin}")).is_ok());
}

#[test]
fn validates_directory_and_svg_references() {
    let manifest = parse_manifest(MANIFEST).unwrap();
    assert!(manifest.validate("other-led", &svg_ids()).is_err());
    for missing in ["pin-a", "dyn-light"] {
        let mut ids = svg_ids();
        ids.remove(missing);
        assert!(manifest
            .validate("basic-led", &ids)
            .unwrap_err()
            .contains("graphic.svg"));
    }
}

#[test]
fn required_fields_defaults_and_size_limits() {
    let minimal = MANIFEST.split("[[controls]]").next().unwrap();
    let parsed = parse_manifest(minimal).unwrap();
    assert!(parsed.controls.is_empty() && parsed.visuals.is_empty());
    let no_pins = minimal.split("[[pins]]").next().unwrap();
    assert!(parse_manifest(no_pins).is_err());
    let mut empty_pins = serde_json::to_value(parse_manifest(MANIFEST).unwrap()).unwrap();
    empty_pins["pins"] = serde_json::json!([]);
    let empty_pins: Manifest = serde_json::from_value(empty_pins).unwrap();
    assert!(empty_pins
        .validate("basic-led", &svg_ids())
        .unwrap_err()
        .contains("pins"));
    for (old, value) in [
        ("basic-led", "a".repeat(65)),
        ("LED", "a".repeat(257)),
        ("A test LED", "a".repeat(2049)),
        ("Anode", "a".repeat(257)),
        ("Brightness", "a".repeat(257)),
    ] {
        assert!(
            parse_manifest(&MANIFEST.replace(&format!("\"{old}\""), &format!("\"{value}\"")))
                .is_err()
        );
    }
    assert!(parse_manifest(&" ".repeat(64 * 1024 + 1)).is_err());
    for (section, count) in [("pins", 128), ("controls", 32), ("visuals", 32)] {
        let mut value = serde_json::to_value(parse_manifest(MANIFEST).unwrap()).unwrap();
        let template = value[section][0].clone();
        let mut entries = Vec::new();
        let mut ids = svg_ids();
        for index in 0..=count {
            let mut entry = template.clone();
            entry["id"] = serde_json::json!(format!("ID{index}"));
            if section == "pins" {
                entry["svg_anchor"] = serde_json::json!(format!("pin-{index}"));
                ids.insert(format!("pin-{index}"));
            } else if section == "visuals" {
                entry["target"] = serde_json::json!(format!("dyn-{index}"));
                ids.insert(format!("dyn-{index}"));
            }
            entries.push(entry);
        }
        value[section] = serde_json::json!(entries);
        let over: Manifest = serde_json::from_value(value.clone()).unwrap();
        assert!(over.validate("basic-led", &ids).is_err());
        value[section].as_array_mut().unwrap().pop();
        let boundary: Manifest = serde_json::from_value(value).unwrap();
        assert!(boundary.validate("basic-led", &ids).is_ok());
    }
}

#[test]
fn json_wire_shape_matches_frontend_contract() {
    let value = serde_json::to_value(parse_manifest(MANIFEST).unwrap()).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "schema_version": 1,
            "compatibility": {"engine_api": 1},
            "component": {"id": "basic-led", "name": "LED", "category": "Output",
                "author": "Nodus", "version": "1.0.0", "description": "A test LED"},
            "electrical": {"operating_voltage_min": 0.0, "operating_voltage_max": 5.0},
            "pins": [{"id": "A", "name": "Anode", "type": "input", "svg_anchor": "pin-a"}],
            "controls": [{"id": "brightness", "label": "Brightness", "type": "slider",
                "min": 0.0, "max": 1.0, "step": 0.1, "default": 0.5}],
            "visuals": [{"id": "brightness", "type": "number", "target": "dyn-light",
                "property": "opacity", "min": 0.0, "max": 1.0, "default": 0.0}]
        })
    );
    for kind in ["input", "output", "bidirectional", "power", "ground", "nc"] {
        let source = MANIFEST.replace("type = \"input\"", &format!("type = \"{kind}\""));
        let json = serde_json::to_value(parse_manifest(&source).unwrap()).unwrap();
        assert_eq!(json["pins"][0]["type"], kind);
    }

    let mcu_toml = format!("{MANIFEST}\n[mcu]\nbackend = \"avr_atmega328p\"\n");
    let mcu_manifest = parse_manifest(&mcu_toml).unwrap();
    assert_eq!(
        mcu_manifest.mcu,
        Some(McuConfig {
            backend: McuBackend::AvrAtmega328p
        })
    );
    let mcu_json = serde_json::to_value(mcu_manifest).unwrap();
    assert_eq!(mcu_json["mcu"]["backend"], "avr_atmega328p");

    let bad_mcu = format!("{MANIFEST}\n[mcu]\nbackend = \"unsupported_arch\"\n");
    assert!(parse_manifest(&bad_mcu).is_err());
}
