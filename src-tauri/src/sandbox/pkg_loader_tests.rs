use super::*;
use crate::sandbox::manifest::tests::MANIFEST;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle id="pin-a" cx="10" cy="10" r="2"/><circle id="dyn-light" cx="50" cy="50" r="20" opacity="0"/></svg>"#;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct TempRegistry(PathBuf);
impl TempRegistry {
    fn new() -> Self {
        for _ in 0..100 {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "nodus-loader-{}-{stamp}-{serial}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create private test directory: {error}"),
            }
        }
        panic!("cannot allocate private test directory");
    }

    fn package(&self, id: &str) -> PathBuf {
        let path = self.0.join(format!("{id}.nodpkg"));
        fs::create_dir(&path).unwrap();
        fs::write(
            path.join("manifest.toml"),
            MANIFEST.replace("basic-led", id),
        )
        .unwrap();
        fs::write(path.join("graphic.svg"), SVG).unwrap();
        fs::write(
            path.join("logic.lua"),
            "error('must never execute during registry scan')",
        )
        .unwrap();
        path
    }

    fn scan(&self) -> RegistryCatalog {
        scan_registry(&self.0).unwrap()
    }
}
impl Drop for TempRegistry {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn rejected(registry: &TempRegistry, field: &str) {
    let catalog = registry.scan();
    assert!(catalog.packages.is_empty());
    assert_eq!(catalog.diagnostics.len(), 1);
    assert!(
        catalog.diagnostics[0].message.contains(field),
        "{:?}",
        catalog.diagnostics
    );
    assert!(catalog.diagnostics[0].path.contains(".nodpkg"));
}

#[test]
fn valid_package_returns_safe_svg_and_never_executes_or_serializes_lua() {
    let registry = TempRegistry::new();
    let path = registry.package("basic-led");
    let source = format!("<!-- removed during normalization -->{SVG}");
    fs::write(path.join("graphic.svg"), &source).unwrap();
    let catalog = registry.scan();
    assert!(catalog.diagnostics.is_empty());
    assert_eq!(catalog.packages.len(), 1);
    let package = &catalog.packages[0];
    assert_eq!(package.manifest.component.id, "basic-led");
    assert_eq!(package.svg, validate_svg(&source).unwrap().svg);
    assert!(!package.svg.contains("removed during normalization"));
    let json = serde_json::to_value(&catalog).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 2);
    assert_eq!(json["packages"][0].as_object().unwrap().len(), 2);
    assert!(json["packages"][0].get("manifest").is_some());
    let serialized = serde_json::to_string(&catalog).unwrap();
    assert!(!serialized.contains("logic.lua"));
    assert!(!serialized.contains("must never execute"));
}

#[test]
fn rejects_directory_id_mismatch_and_unsafe_package_names() {
    for name in ["other-led", "Bad-name", "bad..name", "bad--name"] {
        let registry = TempRegistry::new();
        let path = registry.package(name);
        fs::write(path.join("manifest.toml"), MANIFEST).unwrap();
        rejected(&registry, "component.id");
    }
}

#[test]
fn rejects_each_missing_file_empty_lua_non_utf8_and_nonregular_required_files() {
    for name in ["manifest.toml", "graphic.svg", "logic.lua"] {
        let registry = TempRegistry::new();
        let package = registry.package("basic-led");
        fs::remove_file(package.join(name)).unwrap();
        rejected(&registry, name);
        fs::create_dir(package.join(name)).unwrap();
        rejected(&registry, name);
        fs::remove_dir(package.join(name)).unwrap();
        fs::write(package.join(name), [0xff, 0xfe]).unwrap();
        rejected(&registry, "UTF-8");
    }
    let registry = TempRegistry::new();
    let package = registry.package("basic-led");
    fs::write(package.join("logic.lua"), " \n\t").unwrap();
    rejected(&registry, "logic.lua");
}

#[test]
fn isolates_invalid_svg_manifest_and_anchor_errors_and_sorts_valid_packages() {
    let registry = TempRegistry::new();
    registry.package("z-led");
    let malicious = registry.package("b-led");
    fs::write(
        malicious.join("graphic.svg"),
        SVG.replace("</svg>", "<script>bad()</script></svg>"),
    )
    .unwrap();
    let malformed = registry.package("c-led");
    fs::write(
        malformed.join("manifest.toml"),
        MANIFEST
            .replace("basic-led", "c-led")
            .replace("schema_version = 1", "schema_version = 7"),
    )
    .unwrap();
    let missing = registry.package("d-led");
    fs::write(
        missing.join("graphic.svg"),
        SVG.replace("pin-a", "pin-other"),
    )
    .unwrap();
    registry.package("a-led");
    fs::write(registry.0.join("ignored.nodpkg"), "not a directory").unwrap();
    fs::create_dir(registry.0.join("ignored-folder")).unwrap();
    fs::write(registry.0.join("notes.txt"), "ignored").unwrap();
    let catalog = registry.scan();
    let ids: Vec<_> = catalog
        .packages
        .iter()
        .map(|p| p.manifest.component.id.as_str())
        .collect();
    assert_eq!(ids, ["a-led", "z-led"]);
    assert_eq!(catalog.diagnostics.len(), 3);
    for (diagnostic, name) in catalog.diagnostics.iter().zip(["b-led", "c-led", "d-led"]) {
        assert!(diagnostic.path.ends_with(&format!("{name}.nodpkg")));
    }
    let value = serde_json::to_value(&catalog.diagnostics[0]).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 2);
    assert!(value["path"].is_string() && value["message"].is_string());
}

#[test]
fn rejects_oversize_required_files_and_accepts_lua_at_exact_limit() {
    for (name, limit) in [
        ("manifest.toml", 64 * 1024),
        ("graphic.svg", 256 * 1024),
        ("logic.lua", 128 * 1024),
    ] {
        let registry = TempRegistry::new();
        let package = registry.package("basic-led");
        fs::write(package.join(name), "a".repeat(limit + 1)).unwrap();
        rejected(&registry, "exceeds");
    }
    let registry = TempRegistry::new();
    let package = registry.package("basic-led");
    fs::write(package.join("logic.lua"), "a".repeat(128 * 1024)).unwrap();
    assert_eq!(registry.scan().packages.len(), 1);
}

#[test]
fn root_errors_are_fatal_but_empty_registry_is_valid() {
    let registry = TempRegistry::new();
    assert!(registry.scan().packages.is_empty());
    assert!(scan_registry(&registry.0.join("missing")).is_err());
    let file = registry.0.join("file");
    fs::write(&file, "not a directory").unwrap();
    assert!(scan_registry(&file).is_err());
}

#[test]
fn bounds_direct_entries_and_package_count() {
    let registry = TempRegistry::new();
    for index in 0..512 {
        fs::write(registry.0.join(format!("ignored-{index}")), "").unwrap();
    }
    assert!(registry.scan().packages.is_empty());
    fs::write(registry.0.join("overflow"), "").unwrap();
    assert!(scan_registry(&registry.0).unwrap_err().contains("512"));
    let registry = TempRegistry::new();
    for index in 0..129 {
        registry.package(&format!("led-{index:03}"));
    }
    let catalog = registry.scan();
    assert_eq!(catalog.packages.len(), 128);
    assert_eq!(catalog.diagnostics.len(), 1);
    assert!(catalog.diagnostics[0].message.contains("128"));
}

#[test]
fn bounds_returned_payload_and_diagnostics() {
    let registry = TempRegistry::new();
    let large_svg = SVG.replace(
        "</svg>",
        &format!("<desc>{}</desc></svg>", "x".repeat(240 * 1024)),
    );
    for index in 0..128 {
        let path = registry.package(&format!("led-{index:03}"));
        fs::write(path.join("graphic.svg"), &large_svg).unwrap();
    }
    let catalog = registry.scan();
    assert!(!catalog.packages.is_empty());
    assert!(catalog.packages.len() < 128);
    assert!(catalog
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("payload")));
    assert!(serde_json::to_vec(&catalog).unwrap().len() <= 16 * 1024 * 1024);
    let registry = TempRegistry::new();
    for index in 0..512 {
        fs::create_dir(registry.0.join(format!("invalid-{index}.nodpkg"))).unwrap();
    }
    let catalog = registry.scan();
    assert_eq!(catalog.diagnostics.len(), 512);
    assert!(catalog
        .diagnostics
        .iter()
        .all(|d| d.path.len() <= 256 && d.message.len() <= 256));
}

#[cfg(unix)]
#[test]
fn rejects_package_root_and_required_file_symlinks_inside_and_outside_root() {
    use std::os::unix::fs::symlink;
    let registry = TempRegistry::new();
    let outside = TempRegistry::new();
    let target = outside.package("basic-led");
    symlink(&target, registry.0.join("basic-led.nodpkg")).unwrap();
    rejected(&registry, "symlink");
    let alias = outside.0.join("registry-alias");
    symlink(&registry.0, &alias).unwrap();
    assert!(scan_registry(&alias).is_err());
    for name in ["manifest.toml", "graphic.svg", "logic.lua"] {
        for external in [false, true] {
            let registry = TempRegistry::new();
            let package = registry.package("basic-led");
            let link = package.join(name);
            let destination = if external {
                outside.0.join("linked-file")
            } else {
                package.join("linked-file")
            };
            fs::copy(&link, &destination).unwrap();
            fs::remove_file(&link).unwrap();
            symlink(&destination, &link).unwrap();
            rejected(&registry, name);
        }
    }
    let registry = TempRegistry::new();
    let target = registry.package("a-led");
    symlink(target, registry.0.join("b-led.nodpkg")).unwrap();
    let catalog = registry.scan();
    assert_eq!(catalog.packages.len(), 1);
    assert_eq!(catalog.diagnostics.len(), 1);
}

#[cfg(unix)]
#[test]
fn final_open_rejects_symlink_replacement_after_path_checks() {
    use std::os::unix::fs::symlink;
    let registry = TempRegistry::new();
    let checked = registry.0.join("checked");
    let target = registry.0.join("outside");
    fs::write(&checked, "original").unwrap();
    assert!(fs::symlink_metadata(&checked).unwrap().is_file());
    fs::write(&target, "replacement").unwrap();
    fs::remove_file(&checked).unwrap();
    symlink(&target, &checked).unwrap();
    assert!(open_regular(&checked, "checked").is_err());
}

#[cfg(unix)]
#[test]
fn final_open_rejects_fifo_replacement_without_blocking() {
    let registry = TempRegistry::new();
    let checked = registry.0.join("checked");
    fs::write(&checked, "original").unwrap();
    assert!(fs::symlink_metadata(&checked).unwrap().is_file());
    fs::remove_file(&checked).unwrap();
    assert!(std::process::Command::new("mkfifo")
        .arg(&checked)
        .status()
        .unwrap()
        .success());
    assert!(open_regular(&checked, "checked").is_err());
}

#[test]
fn builtin_led_matches_the_shared_manifest_contract() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let catalog = scan_registry(&workspace.join("components-registry")).unwrap();
    assert!(catalog.diagnostics.is_empty(), "{:?}", catalog.diagnostics);
    assert_eq!(catalog.packages.len(), 1);
    let actual = serde_json::to_value(&catalog.packages[0].manifest).unwrap();
    let expected: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(workspace.join("docs/nodpkg-v1-led.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(actual, expected);
}

#[cfg(unix)]
#[test]
fn rejects_fifo_required_files_without_opening_them() {
    for name in ["manifest.toml", "graphic.svg", "logic.lua"] {
        let registry = TempRegistry::new();
        let path = registry.package("basic-led").join(name);
        fs::remove_file(&path).unwrap();
        let status = std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        rejected(&registry, "regular file");
    }
}
