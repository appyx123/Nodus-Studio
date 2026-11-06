use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub compatibility: Compatibility,
    pub component: Component,
    pub electrical: Electrical,
    pub pins: Vec<Pin>,
    #[serde(default)]
    pub controls: Vec<SliderControl>,
    #[serde(default)]
    pub visuals: Vec<NumberVisual>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    pub engine_api: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub category: String,
    pub author: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Electrical {
    pub operating_voltage_min: f64,
    pub operating_voltage_max: f64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: PinType,
    pub svg_anchor: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PinType {
    Input,
    Output,
    Bidirectional,
    Power,
    Ground,
    Nc,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SliderControl {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: SliderType,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SliderType {
    Slider,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NumberVisual {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: NumberType,
    pub target: String,
    pub property: VisualProperty,
    pub min: f64,
    pub max: f64,
    pub default: f64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NumberType {
    Number,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualProperty {
    Opacity,
}

pub fn parse_manifest(source: &str) -> Result<Manifest, String> {
    if source.len() > 64 * 1024 {
        return Err("manifest.toml exceeds 64 KiB".into());
    }
    let manifest: Manifest =
        toml::from_str(source).map_err(|error| format!("manifest.toml: {error}"))?;
    manifest.validate_values()?;
    Ok(manifest)
}

impl Manifest {
    pub fn validate(&self, package_id: &str, svg_ids: &HashSet<String>) -> Result<(), String> {
        self.validate_values()?;
        if self.component.id != package_id {
            return Err("component.id must match the .nodpkg directory basename".into());
        }
        for pin in &self.pins {
            if !svg_ids.contains(&pin.svg_anchor) {
                return Err(format!(
                    "pins.{}.svg_anchor is absent from graphic.svg",
                    pin.id
                ));
            }
        }
        for visual in &self.visuals {
            if !svg_ids.contains(&visual.target) {
                return Err(format!(
                    "visuals.{}.target is absent from graphic.svg",
                    visual.id
                ));
            }
        }
        Ok(())
    }

    fn validate_values(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("schema_version must be 1".into());
        }
        if self.compatibility.engine_api != 1 {
            return Err("compatibility.engine_api must be 1".into());
        }
        let component = &self.component;
        if !component_id(&component.id) {
            return Err("component.id must be lowercase ASCII kebab-case, at most 64 bytes".into());
        }
        for (field, value, limit) in [
            ("component.name", &component.name, 256),
            ("component.category", &component.category, 256),
            ("component.author", &component.author, 256),
            ("component.version", &component.version, 256),
            ("component.description", &component.description, 2048),
        ] {
            text(field, value, limit)?;
        }
        if component.version.trim() != component.version
            || semver::Version::parse(&component.version).is_err()
        {
            return Err("component.version must be SemVer without surrounding whitespace".into());
        }
        let electrical = &self.electrical;
        range(
            "electrical.operating_voltage",
            electrical.operating_voltage_min,
            electrical.operating_voltage_min,
            electrical.operating_voltage_max,
        )?;
        if electrical.operating_voltage_min < 0.0 || electrical.operating_voltage_max > 1000.0 {
            return Err("electrical.operating_voltage must be within 0..=1000".into());
        }
        if self.pins.is_empty() || self.pins.len() > 128 {
            return Err("pins must contain 1..=128 entries".into());
        }
        if self.controls.len() > 32 || self.visuals.len() > 32 {
            return Err("controls and visuals must contain at most 32 entries each".into());
        }
        let mut ids = HashSet::new();
        let mut anchors = HashSet::new();
        for pin in &self.pins {
            unique_id("pins.id", &pin.id, &mut ids)?;
            text("pins.name", &pin.name, 256)?;
            prefixed("pins.svg_anchor", &pin.svg_anchor, "pin-")?;
            if !anchors.insert(&pin.svg_anchor) {
                return Err("pins.svg_anchor must not be reused".into());
            }
        }
        ids.clear();
        for control in &self.controls {
            unique_id("controls.id", &control.id, &mut ids)?;
            text("controls.label", &control.label, 256)?;
            range("controls.range", control.min, control.default, control.max)?;
            if !control.step.is_finite() || control.step <= 0.0 {
                return Err("controls.step must be finite and positive".into());
            }
        }
        ids.clear();
        let mut targets = HashSet::new();
        for visual in &self.visuals {
            unique_id("visuals.id", &visual.id, &mut ids)?;
            prefixed("visuals.target", &visual.target, "dyn-")?;
            range("visuals.range", visual.min, visual.default, visual.max)?;
            if visual.min < 0.0 || visual.max > 1.0 {
                return Err("visuals.opacity range must be within 0..=1".into());
            }
            // Opacity is the only v1 property; target uniqueness is binding uniqueness.
            if !targets.insert(&visual.target) {
                return Err("visuals target/property binding must not be repeated".into());
            }
        }
        Ok(())
    }
}

fn component_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn identifier(value: &str) -> bool {
    value.len() <= 64
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn unique_id<'a>(field: &str, value: &'a str, ids: &mut HashSet<&'a str>) -> Result<(), String> {
    if !identifier(value) {
        return Err(format!(
            "{field} must match [A-Za-z][A-Za-z0-9_-]*, at most 64 bytes"
        ));
    }
    if !ids.insert(value) {
        return Err(format!("{field} contains duplicate ID {value}"));
    }
    Ok(())
}

fn text(field: &str, value: &str, limit: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > limit {
        return Err(format!(
            "{field} must be nonempty and at most {limit} bytes"
        ));
    }
    Ok(())
}

fn prefixed(field: &str, value: &str, prefix: &str) -> Result<(), String> {
    text(field, value, 256)?;
    if !value.starts_with(prefix) || value.len() == prefix.len() {
        return Err(format!(
            "{field} must start with {prefix} and have a suffix"
        ));
    }
    Ok(())
}

fn range(field: &str, min: f64, default: f64, max: f64) -> Result<(), String> {
    if ![min, default, max].iter().all(|value| value.is_finite()) || min > default || default > max
    {
        return Err(format!("{field} requires finite min <= default <= max"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
pub(crate) mod tests;
