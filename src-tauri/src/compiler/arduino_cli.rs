use crate::NodusError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_FQBN: &str = "arduino:avr:uno";
pub const MAX_SKETCH_BYTES: usize = 128 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const POLL_INTERVAL_MS: u64 = 15;
static WORKSPACE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerDiagnostic {
    pub line: u32,
    pub column: u32,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileRequest {
    pub sketch_source: String,
    pub fqbn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResponse {
    pub success: bool,
    pub status: String,
    pub fqbn: String,
    pub diagnostics: Vec<CompilerDiagnostic>,
    pub stdout: String,
    pub stderr: String,
    pub hex_len: usize,
    pub elf_len: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CompiledArtifact {
    pub fqbn: String,
    pub hex: String,
    pub elf: Vec<u8>,
}

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn create() -> Result<Self, NodusError> {
        let seq = WORKSPACE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("nodus-sketch-{}-{seq}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)
            .map_err(|err| NodusError::Circuit(format!("Temp workspace creation failed: {err}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&root, fs::Permissions::from_mode(0o700));
        }
        Ok(Self(root))
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub struct CompilerController {
    cancel_flag: Arc<AtomicBool>,
    compiling: Arc<AtomicBool>,
    latest_artifact: Arc<Mutex<Option<CompiledArtifact>>>,
    cli_override: Option<PathBuf>,
    timeout: Duration,
}

impl Default for CompilerController {
    fn default() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            compiling: Arc::new(AtomicBool::new(false)),
            latest_artifact: Arc::new(Mutex::new(None)),
            cli_override: None,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
        }
    }
}

impl CompilerController {
    pub fn with_cli_and_timeout(cli_path: PathBuf, timeout: Duration) -> Self {
        Self {
            cli_override: Some(cli_path),
            timeout,
            ..Self::default()
        }
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn latest_artifact(&self) -> Option<CompiledArtifact> {
        self.latest_artifact.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_artifact(&self, artifact: CompiledArtifact) {
        if let Ok(mut guard) = self.latest_artifact.lock() {
            *guard = Some(artifact);
        }
    }

    pub fn compile(&self, request: CompileRequest) -> Result<CompileResponse, NodusError> {
        let src = request.sketch_source.trim();
        if src.is_empty() || request.sketch_source.len() > MAX_SKETCH_BYTES {
            return Err(NodusError::Circuit(
                "Sketch source is empty or exceeds size limit.".into(),
            ));
        }
        let fqbn = request
            .fqbn
            .as_deref()
            .unwrap_or(DEFAULT_FQBN)
            .trim()
            .to_string();
        if !is_valid_fqbn(&fqbn) {
            return Err(NodusError::Circuit(format!("Invalid board FQBN '{fqbn}'.")));
        }
        if self
            .compiling
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(NodusError::Circuit(
                "A compilation is already in progress.".into(),
            ));
        }
        self.cancel_flag.store(false, Ordering::SeqCst);
        let result = self.run_compile(&request.sketch_source, &fqbn);
        self.compiling.store(false, Ordering::SeqCst);
        result
    }

    fn run_compile(&self, source: &str, fqbn: &str) -> Result<CompileResponse, NodusError> {
        let cli = resolve_arduino_cli(self.cli_override.as_deref())?;
        let workspace = TempWorkspace::create()?;
        let (sketch_dir, build_dir) = (workspace.0.join("sketch"), workspace.0.join("build"));
        fs::create_dir_all(&sketch_dir)
            .and_then(|()| fs::create_dir_all(&build_dir))
            .and_then(|()| fs::write(sketch_dir.join("sketch.ino"), source))
            .map_err(|err| NodusError::Circuit(format!("Failed preparing sketch: {err}")))?;

        let mut child = Command::new(&cli)
            .args(["compile", "--no-color", "--fqbn", fqbn, "--output-dir"])
            .arg(&build_dir)
            .arg(&sketch_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| NodusError::Circuit(format!("Could not run '{}': {e}", cli.display())))?;

        let (out_h, err_h) = (
            child.stdout.take().map(spawn_reader),
            child.stderr.take().map(spawn_reader),
        );
        let start = Instant::now();
        let status = loop {
            if self.cancel_flag.load(Ordering::SeqCst) {
                let _ = (child.kill(), child.wait());
                return Err(NodusError::Circuit("Compilation cancelled by user.".into()));
            }
            if start.elapsed() >= self.timeout {
                let _ = (child.kill(), child.wait());
                return Err(NodusError::Circuit("Compilation timed out.".into()));
            }
            match child.try_wait() {
                Ok(Some(exit)) => break exit,
                Ok(None) => thread::sleep(Duration::from_millis(POLL_INTERVAL_MS)),
                Err(e) => {
                    let _ = child.kill();
                    return Err(NodusError::Circuit(format!("Wait failed: {e}")));
                }
            }
        };
        let (stdout, stderr) = (join_reader(out_h), join_reader(err_h));
        let combined = format!("{stdout}\n{stderr}");
        if !status.success()
            && (combined.contains("Platform 'arduino:avr' not found")
                || combined.contains("platform not installed"))
        {
            return Err(NodusError::Circuit(
                "Arduino AVR core is missing. Run: arduino-cli core install arduino:avr".into(),
            ));
        }

        let diagnostics = parse_compiler_diagnostics(&combined);
        let success = status.success();
        let (hex_len, elf_len) = if success {
            let hex = fs::read_to_string(build_dir.join("sketch.ino.hex")).unwrap_or_default();
            let elf = fs::read(build_dir.join("sketch.ino.elf")).unwrap_or_default();
            let lens = (hex.len(), elf.len());
            if !hex.is_empty() || !elf.is_empty() {
                self.set_artifact(CompiledArtifact {
                    fqbn: fqbn.to_string(),
                    hex,
                    elf,
                });
            }
            lens
        } else {
            (0, 0)
        };
        Ok(CompileResponse {
            success,
            status: if success {
                "Compilation succeeded.".into()
            } else {
                format!("Compilation failed ({} diagnostic(s)).", diagnostics.len())
            },
            fqbn: fqbn.to_string(),
            diagnostics,
            stdout,
            stderr,
            hex_len,
            elf_len,
        })
    }
}

fn spawn_reader<R: Read + Send + 'static>(mut reader: R) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = reader.by_ref().take(256 * 1024).read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    })
}

fn join_reader(handle: Option<thread::JoinHandle<String>>) -> String {
    handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn is_valid_fqbn(fqbn: &str) -> bool {
    let parts: Vec<&str> = fqbn.split(':').collect();
    parts.len() >= 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-=".contains(c))
        })
}

fn resolve_arduino_cli(explicit: Option<&Path>) -> Result<PathBuf, NodusError> {
    if let Some(path) = explicit {
        return path.is_file().then(|| path.to_path_buf()).ok_or_else(|| {
            NodusError::Circuit(format!("arduino-cli not found at '{}'.", path.display()))
        });
    }
    if let Ok(env_path) = std::env::var("NODUS_ARDUINO_CLI") {
        let candidate = PathBuf::from(env_path);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    if let Ok(path_var) = std::env::var("PATH") {
        if let Some(found) = std::env::split_paths(&path_var)
            .map(|d| d.join("arduino-cli"))
            .find(|p| p.is_file())
        {
            return Ok(found);
        }
    }
    if let Some(local) = std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".local/bin/arduino-cli"))
        .filter(|p| p.is_file())
    {
        return Ok(local);
    }
    Err(NodusError::Circuit(
        "arduino-cli not found. Install arduino-cli and arduino:avr core.".into(),
    ))
}

pub fn parse_compiler_diagnostics(output: &str) -> Vec<CompilerDiagnostic> {
    let mut diagnostics = Vec::new();
    for raw_line in output.lines() {
        let line = strip_ansi(raw_line);
        let Some(ino_idx) = line.find(".ino:") else {
            continue;
        };
        let mut segments = line[ino_idx + 5..].splitn(4, ':');
        let (Some(Ok(line_num)), Some(second)) = (
            segments.next().map(|s| s.trim().parse::<u32>()),
            segments.next(),
        ) else {
            continue;
        };
        let (col_num, sev_raw, msg_raw) = if let Ok(col) = second.trim().parse::<u32>() {
            let (Some(sev), Some(msg)) = (segments.next(), segments.next()) else {
                continue;
            };
            (col, sev.trim(), msg.trim())
        } else if let Some(msg) = segments.next() {
            (1, second.trim(), msg.trim())
        } else {
            continue;
        };
        let severity = match sev_raw {
            "error" | "fatal error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            "note" => DiagnosticSeverity::Note,
            _ => continue,
        };
        diagnostics.push(CompilerDiagnostic {
            line: line_num,
            column: col_num,
            severity,
            message: msg_raw.to_string(),
        });
    }
    diagnostics
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gcc_diagnostics_and_handles_missing_or_timed_out_cli() {
        let sample = "/tmp/sketch.ino:4:3: error: expected ';'\n/tmp/sketch.ino:7:2: warning: w";
        let parsed = parse_compiler_diagnostics(sample);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].line, 4);
        assert_eq!(parsed[0].severity, DiagnosticSeverity::Error);
        let missing = CompilerController::with_cli_and_timeout(
            PathBuf::from("/nonexistent"),
            Duration::from_millis(50),
        );
        let req = CompileRequest {
            sketch_source: "void setup(){} void loop(){}".into(),
            fqbn: None,
        };
        assert!(missing.compile(req.clone()).is_err());
        let zero = CompilerController::with_cli_and_timeout(
            resolve_arduino_cli(None).unwrap(),
            Duration::ZERO,
        );
        assert!(zero.compile(req).is_err());
        let controller = CompilerController::default();
        let src = "void setup(){pinMode(13,OUTPUT);} void loop(){digitalWrite(13,HIGH);}";
        let valid = controller.compile(CompileRequest {
            sketch_source: src.into(),
            fqbn: None,
        });
        assert!(valid.is_ok_and(|r| r.success && r.hex_len > 100));
        let bad = "void setup() {\n  pinMode(13, OUTPUT)\n}\nvoid loop() {}\n";
        let invalid = controller.compile(CompileRequest {
            sketch_source: bad.into(),
            fqbn: None,
        });
        assert!(invalid.is_ok_and(|r| !r.success && r.diagnostics[0].line == 3));
    }
}
