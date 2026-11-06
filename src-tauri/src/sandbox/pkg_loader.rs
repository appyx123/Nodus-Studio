use super::manifest::{parse_manifest, Manifest};
use super::svg::validate_svg;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::Path;

const MAX_ENTRIES: usize = 512;
const MAX_PACKAGES: usize = 128;
const MAX_PACKAGE_PAYLOAD: usize = 14 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct ComponentPackage {
    pub manifest: Manifest,
    pub svg: String,
    #[serde(skip_serializing)]
    pub lua_source: String,
}

#[derive(Debug, Serialize)]
pub struct PackageDiagnostic {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RegistryCatalog {
    pub packages: Vec<ComponentPackage>,
    pub diagnostics: Vec<PackageDiagnostic>,
}

pub fn scan_registry(root: &Path) -> Result<RegistryCatalog, String> {
    let metadata = fs::symlink_metadata(root).map_err(|error| format!("registry root: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("registry root must be a real directory, not a symlink".into());
    }
    let root = fs::canonicalize(root).map_err(|error| format!("registry root: {error}"))?;
    let entries = fs::read_dir(&root).map_err(|error| format!("registry root: {error}"))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("registry iteration: {error}"))?;
        if paths.len() == MAX_ENTRIES {
            return Err("registry exceeds 512 direct entries".into());
        }
        paths.push((entry.file_name(), entry.path()));
    }
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    let mut catalog = RegistryCatalog {
        packages: Vec::new(),
        diagnostics: Vec::new(),
    };
    let mut candidates = 0;
    let mut payload = 0;
    for (name, path) in paths {
        if path
            .extension()
            .is_none_or(|extension| extension != "nodpkg")
        {
            continue;
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                diagnostic(&mut catalog, &path, &format!("package metadata: {error}"));
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            diagnostic(&mut catalog, &path, "package symlinks are forbidden");
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }
        candidates += 1;
        if candidates > MAX_PACKAGES {
            diagnostic(
                &mut catalog,
                &path,
                "registry exceeds 128 package directories",
            );
            continue;
        }
        let result = name
            .to_str()
            .and_then(|name| name.strip_suffix(".nodpkg"))
            .ok_or_else(|| "package directory name must be UTF-8 with .nodpkg suffix".to_string())
            .and_then(|id| load_package(&root, &path, id));
        match result {
            Ok(package) => {
                let size = serde_json::to_vec(&package)
                    .map_err(|error| format!("catalog serialization: {error}"))?
                    .len()
                    + 1;
                if payload + size > MAX_PACKAGE_PAYLOAD {
                    diagnostic(
                        &mut catalog,
                        &path,
                        "registry package payload exceeds 14 MiB",
                    );
                } else {
                    payload += size;
                    catalog.packages.push(package);
                }
            }
            Err(error) => diagnostic(&mut catalog, &path, &error),
        }
    }
    Ok(catalog)
}

fn load_package(root: &Path, path: &Path, id: &str) -> Result<ComponentPackage, String> {
    let canonical = fs::canonicalize(path).map_err(|error| format!("package path: {error}"))?;
    if canonical.parent() != Some(root) {
        return Err("package directory escapes registry root".into());
    }
    let source = read_required(&canonical, "manifest.toml", 64 * 1024)?;
    let manifest = parse_manifest(&source)?;
    let svg = read_required(&canonical, "graphic.svg", 256 * 1024)?;
    let info = validate_svg(&svg).map_err(|error| format!("graphic.svg: {error}"))?;
    manifest.validate(id, &info.ids)?;
    let lua = read_required(&canonical, "logic.lua", 128 * 1024)?;
    if lua.trim().is_empty() {
        return Err("logic.lua must contain nonempty text (not executed in Phase 1)".into());
    }
    Ok(ComponentPackage {
        manifest,
        svg: info.svg,
        lua_source: lua,
    })
}

fn read_required(directory: &Path, name: &str, limit: usize) -> Result<String, String> {
    let path = directory.join(name);
    let metadata = fs::symlink_metadata(&path).map_err(|error| format!("{name}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{name} must be a regular file, not a symlink"));
    }
    let canonical = fs::canonicalize(&path).map_err(|error| format!("{name}: {error}"))?;
    if canonical.parent() != Some(directory) {
        return Err(format!("{name} escapes package directory"));
    }
    if metadata.len() > limit as u64 {
        return Err(format!("{name} exceeds {limit} bytes"));
    }
    let file = open_regular(&path, name)?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{name}: {error}"))?;
    if bytes.len() > limit {
        return Err(format!("{name} exceeds {limit} bytes"));
    }
    String::from_utf8(bytes).map_err(|_| format!("{name} must be UTF-8"))
}

fn open_regular(path: &Path, name: &str) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A file replaced after metadata checks must not follow a link or block on a FIFO.
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("{name}: {error}"))?;
    if !file
        .metadata()
        .map_err(|error| format!("{name}: {error}"))?
        .is_file()
    {
        return Err(format!("{name} must be a regular file"));
    }
    Ok(file)
}

fn diagnostic(catalog: &mut RegistryCatalog, path: &Path, message: &str) {
    // 512 diagnostics, each <= 256 UTF-8 bytes per field, leave total JSON below 16 MiB
    // even with worst-case JSON escaping, in addition to the 14 MiB package budget.
    fn bounded(value: &str) -> String {
        let mut end = value.len().min(256);
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value[..end].to_owned()
    }
    if catalog.diagnostics.len() < MAX_ENTRIES {
        catalog.diagnostics.push(PackageDiagnostic {
            path: bounded(&path.to_string_lossy()),
            message: bounded(message),
        });
    }
}

#[cfg(test)]
#[path = "pkg_loader_tests.rs"]
mod tests;
