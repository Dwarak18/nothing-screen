use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
    pub model: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AdbError {
    #[error("adb binary not found")]
    NotFound,
    #[error("adb command failed: {0}")]
    CommandFailed(String),
    #[error("failed to parse adb output: {0}")]
    Parse(String),
}

/// Platform-specific adb executable name.
const ADB_NAME: &str = if cfg!(windows) { "adb.exe" } else { "adb" };

/// Resolve the adb binary: env override → next to our executable (bundled)
/// → dev fallback (`src-tauri/binaries/`) → bare name from PATH.
pub fn adb_binary() -> PathBuf {
    // First, check if the environment variable is set.
    if let Ok(p) = std::env::var("NOTHINGCAST_ADB") {
        let p = PathBuf::from(p);
        if p.exists() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [dir.join("binaries").join(ADB_NAME), dir.join(ADB_NAME)] {
                if cand.exists() {
                    return cand;
                }
            }
        }
    }
    #[cfg(debug_assertions)]
    {
        let cand =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join(ADB_NAME);
        if cand.exists() {
            return cand;
        }
    }
    PathBuf::from(ADB_NAME)
}

/// Build a Command pointing at the resolved adb binary.
pub fn command(args: &[&str]) -> std::process::Command {
    let mut cmd = std::process::Command::new(adb_binary());
    cmd.args(args);
    cmd
}

fn run(args: &[&str]) -> Result<String, AdbError> {
    let out = command(args)
        .output()
        .map_err(|_| AdbError::NotFound)?;
    if !out.status.success() {
        return Err(AdbError::CommandFailed(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn run_checked(args: &[&str]) -> Result<String, AdbError> {
    run(args)
}

pub fn ensure_server() -> Result<(), AdbError> {
    run(&["start-server"]).map(|_| ())
}

pub fn pair(address: &str, code: &str) -> Result<(), AdbError> {
    run(&["pair", address, code]).map(|_| ())
}

pub fn connect(address: &str) -> Result<(), AdbError> {
    run(&["connect", address])
        .map(|out| {
            if !out.contains("connected") {
                Err(AdbError::Parse(out))
            } else {
                Ok(())
            }
        })?
}

pub fn devices() -> Result<Vec<AdbDevice>, AdbError> {
    let out = run(&["devices", "-l"])?;
    out.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                return Err(AdbError::Parse(line.to_owned()));
            }
            let model = parts
                .iter()
                .skip(2)
                .find_map(|f| f.strip_prefix("model:"))
                .map(str::to_owned);
            Ok(AdbDevice {
                serial: parts[0].to_owned(),
                state: parts[1].to_owned(),
                model,
            })
        })
        .collect()
}
