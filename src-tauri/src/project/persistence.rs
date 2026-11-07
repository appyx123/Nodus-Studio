use super::types::{
    EndpointRecord, LoadProjectResponse, NodusProjectFile, ProjectWarning, WarningLevel,
    CURRENT_SCHEMA_VERSION, MAX_PROJECT_FILE_SIZE, NODUS_PROJECT_FORMAT,
};
use crate::engine::circuit_graph::{CircuitState, Instance};
use crate::engine::pin::PinRuntime;
use crate::engine::{Endpoint, Wire};
use crate::sandbox::manifest::{Pin, PinType};
use crate::NodusError;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn project_error(msg: impl Into<String>) -> NodusError {
    NodusError::Project(msg.into())
}

pub fn sanitize_project(mut project: NodusProjectFile) -> NodusProjectFile {
    project.format = NODUS_PROJECT_FORMAT.into();
    project.version = CURRENT_SCHEMA_VERSION.into();
    project.app_id = "org.nodus.Studio".into();
    project
}

pub fn parse_project_bytes(data: &[u8]) -> Result<NodusProjectFile, NodusError> {
    if data.len() > MAX_PROJECT_FILE_SIZE {
        return Err(project_error(format!(
            "Project file size exceeds limit ({} > {} bytes)",
            data.len(),
            MAX_PROJECT_FILE_SIZE
        )));
    }

    let project: NodusProjectFile = serde_json::from_slice(data)
        .map_err(|err| project_error(format!("Malformed project JSON: {err}")))?;

    if project.format != NODUS_PROJECT_FORMAT {
        return Err(project_error(format!(
            "Invalid project format identifier '{}', expected '{}'",
            project.format, NODUS_PROJECT_FORMAT
        )));
    }

    let proj_semver = semver::Version::parse(&project.version)
        .map_err(|err| project_error(format!("Invalid schema version string: {err}")))?;
    let curr_semver = semver::Version::parse(CURRENT_SCHEMA_VERSION)
        .map_err(|err| project_error(format!("Internal schema version error: {err}")))?;

    if proj_semver.major != curr_semver.major {
        return Err(project_error(format!(
            "Unsupported project schema version '{}'. Current supported major version is '{}'",
            project.version, curr_semver.major
        )));
    }

    Ok(project)
}

pub fn save_project_atomic(path: &Path, project: &NodusProjectFile) -> Result<(), NodusError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() && !parent.exists() {
        return Err(project_error(format!(
            "Destination directory does not exist: {}",
            parent.display()
        )));
    }

    let clean = sanitize_project(project.clone());
    let json_bytes = serde_json::to_vec_pretty(&clean)
        .map_err(|err| project_error(format!("Failed to serialize project: {err}")))?;

    if json_bytes.len() > MAX_PROJECT_FILE_SIZE {
        return Err(project_error("Serialized project exceeds 10 MiB limit"));
    }

    let tmp_path = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project"),
        std::process::id()
    ));

    let mut file = File::create(&tmp_path)
        .map_err(|err| project_error(format!("Failed to create temporary file: {err}")))?;

    file.write_all(&json_bytes)
        .map_err(|err| project_error(format!("Failed to write project data: {err}")))?;

    file.sync_all()
        .map_err(|err| project_error(format!("Failed to flush project file: {err}")))?;

    drop(file);

    fs::rename(&tmp_path, path).map_err(|err| {
        let _ = fs::remove_file(&tmp_path);
        project_error(format!("Failed to atomically replace project file: {err}"))
    })?;

    Ok(())
}

fn endpoint_from_record(record: &EndpointRecord) -> Endpoint {
    Endpoint {
        instance_id: record.instance_id.clone(),
        pin_id: record.pin_id.clone(),
    }
}

pub fn apply_project_to_circuit(
    project: &NodusProjectFile,
    circuit: &mut CircuitState,
) -> Vec<ProjectWarning> {
    let mut warnings = Vec::new();
    let mut new_instances = BTreeMap::new();

    for inst_rec in &project.circuit.instances {
        if let Some(def) = circuit.catalog.get(&inst_rec.component_id) {
            if def.version != inst_rec.component_version {
                warnings.push(ProjectWarning {
                    level: WarningLevel::Warning,
                    message: format!(
                        "Component '{}' version mismatch (project requested {}, registry provides {})",
                        inst_rec.component_id, inst_rec.component_version, def.version
                    ),
                });
            }
            let runtime = def
                .pins
                .keys()
                .map(|pin_id| (pin_id.clone(), PinRuntime::default()))
                .collect();
            new_instances.insert(
                inst_rec.instance_id.clone(),
                Instance {
                    component_id: inst_rec.component_id.clone(),
                    component_version: inst_rec.component_version.clone(),
                    pins: def.pins.clone(),
                    runtime,
                },
            );
        } else {
            warnings.push(ProjectWarning {
                level: WarningLevel::Warning,
                message: format!(
                    "Package '{}' (@{}) is missing from registry; loaded as placeholder",
                    inst_rec.component_id, inst_rec.component_version
                ),
            });
            let mut placeholder_pins = BTreeMap::new();
            for wire in &project.circuit.wires {
                if wire.a.instance_id == inst_rec.instance_id {
                    placeholder_pins.insert(
                        wire.a.pin_id.clone(),
                        Pin {
                            id: wire.a.pin_id.clone(),
                            name: wire.a.pin_id.clone(),
                            kind: PinType::Bidirectional,
                            svg_anchor: wire.a.pin_id.clone(),
                        },
                    );
                }
                if wire.b.instance_id == inst_rec.instance_id {
                    placeholder_pins.insert(
                        wire.b.pin_id.clone(),
                        Pin {
                            id: wire.b.pin_id.clone(),
                            name: wire.b.pin_id.clone(),
                            kind: PinType::Bidirectional,
                            svg_anchor: wire.b.pin_id.clone(),
                        },
                    );
                }
            }
            let runtime = placeholder_pins
                .keys()
                .map(|pin_id| (pin_id.clone(), PinRuntime::default()))
                .collect();
            new_instances.insert(
                inst_rec.instance_id.clone(),
                Instance {
                    component_id: inst_rec.component_id.clone(),
                    component_version: inst_rec.component_version.clone(),
                    pins: placeholder_pins,
                    runtime,
                },
            );
        }
    }

    let mut new_wires = BTreeMap::new();
    for wire_rec in &project.circuit.wires {
        new_wires.insert(
            wire_rec.wire_id.clone(),
            Wire {
                wire_id: wire_rec.wire_id.clone(),
                a: endpoint_from_record(&wire_rec.a),
                b: endpoint_from_record(&wire_rec.b),
                color: wire_rec.color,
            },
        );
    }

    circuit.instances = new_instances;
    circuit.wires = new_wires;

    warnings
}

pub fn load_project_from_path(
    path: &Path,
    circuit: &mut CircuitState,
) -> Result<LoadProjectResponse, NodusError> {
    let metadata = fs::metadata(path)
        .map_err(|err| project_error(format!("Cannot read project file metadata: {err}")))?;

    if metadata.len() as usize > MAX_PROJECT_FILE_SIZE {
        return Err(project_error(format!(
            "Project file exceeds maximum allowed size ({} bytes)",
            metadata.len()
        )));
    }

    let bytes = fs::read(path)
        .map_err(|err| project_error(format!("Failed to read project file: {err}")))?;

    let project = parse_project_bytes(&bytes)?;
    let warnings = apply_project_to_circuit(&project, circuit);

    Ok(LoadProjectResponse { project, warnings })
}
