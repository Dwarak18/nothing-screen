use crate::adb;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("scrcpy client binary missing: {0}")]
    ClientMissing(PathBuf),
    #[error(transparent)]
    Adb(#[from] adb::AdbError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy)]
pub struct QualityPreset {
    pub name: &'static str,
    pub max_size: u32,
    pub max_fps: u32,
    pub codec: &'static str,
}

pub const SMOOTH: QualityPreset = QualityPreset { name: "smooth", max_size: 720, max_fps: 60, codec: "h264" };
pub const BALANCED: QualityPreset = QualityPreset { name: "balanced", max_size: 1080, max_fps: 60, codec: "h264" };
pub const SHARP: QualityPreset = QualityPreset { name: "sharp", max_size: 0, max_fps: 60, codec: "h265" };

/// Platform-specific scrcpy client executable name.
const SCRCPY_NAME: &str = if cfg!(windows) { "scrcpy.exe" } else { "scrcpy" };

/// Resolve the scrcpy client binary: env override → next to our executable
/// (bundled) → dev fallback (`src-tauri/binaries/`) → bare name from PATH.
///
/// The client (scrcpy.exe) must sit next to its own dependencies (SDL2,
/// FFmpeg DLLs) and its `adb`/`scrcpy-server`. Our bundled copy in
/// `src-tauri/binaries/` satisfies all of these.
pub fn scrcpy_binary() -> PathBuf {
    if let Ok(p) = std::env::var("NOTHINGCAST_SCRCPY") {
        let p = PathBuf::from(p);
        if p.exists() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [dir.join("binaries").join(SCRCPY_NAME), dir.join(SCRCPY_NAME)] {
                if cand.exists() {
                    return cand;
                }
            }
        }
    }
    #[cfg(debug_assertions)]
    {
        let cand = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join(SCRCPY_NAME);
        if cand.exists() {
            return cand;
        }
    }
    PathBuf::from(SCRCPY_NAME)
}

/// The running scrcpy client process (owned by us so Disconnect can end it).
static SESSION: Mutex<Option<std::process::Child>> = Mutex::new(None);

/// Launch the bundled scrcpy client against an already-connected device.
///
/// The client manages everything itself: it pushes its own `scrcpy-server`
/// to the device, opens the adb forward, decodes the H.264 stream and shows a
/// real mirror window with touch/keyboard/audio. `serial` is the adb serial
/// (`ip:port` for wireless devices).
pub fn launch_client(serial: &str, preset: QualityPreset) -> Result<(), StreamError> {
    stop_stream();
    ensure_connected(serial)?;

    let bin = scrcpy_binary();
    if !bin.exists() {
        return Err(StreamError::ClientMissing(bin.clone()));
    }

    let mut cmd = std::process::Command::new(&bin);
    cmd.arg("--serial")
        .arg(serial)
        .arg(format!("--max-fps={}", preset.max_fps))
        .arg(format!("--video-codec={}", preset.codec))
        .arg("--audio")
        .arg("--stay-awake=false")
        .arg(format!("--window-title=NothingCast ({})", preset.name));
    if preset.max_size > 0 {
        cmd.arg(format!("--max-size={}", preset.max_size));
    }

    let child = cmd.spawn().map_err(|e| {
        log::error!("scrcpy client failed to spawn: {e}");
        StreamError::Io(e)
    })?;

    *SESSION.lock().unwrap() = Some(child);
    log::info!("scrcpy client launched for {serial} ({})", preset.name);
    Ok(())
}

/// Kill any running stream and clean up host-side state.
pub fn stop_stream() {
    if let Ok(mut guard) = SESSION.lock() {
        if let Some(child) = guard.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        *guard = None;
    }
}

/// Verify the serial shows up as an active adb device before launching.
fn ensure_connected(serial: &str) -> Result<(), StreamError> {
    let devices = adb::devices()?;
    if devices.iter().any(|d| d.serial == serial && d.state == "device") {
        Ok(())
    } else {
        Err(StreamError::Adb(adb::AdbError::Parse(format!(
            "device `{serial}` is not connected"
        ))))
    }
}