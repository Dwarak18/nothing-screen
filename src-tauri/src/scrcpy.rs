use crate::adb;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("server binary missing: {0}")]
    ServerMissing(PathBuf),
    #[error(transparent)]
    Adb(#[from] adb::AdbError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct QualityPreset {
    pub name: &'static str,
    pub max_size: u32,
    pub max_fps: u32,
    pub codec: &'static str,
}

pub const SMOOTH: QualityPreset = QualityPreset { name: "smooth", max_size: 720, max_fps: 60, codec: "h264" };
pub const BALANCED: QualityPreset = QualityPreset { name: "balanced", max_size: 1080, max_fps: 60, codec: "h264" };
pub const SHARP: QualityPreset = QualityPreset { name: "sharp", max_size: 0, max_fps: 60, codec: "h265" };

/// Version of the vendored scrcpy-server jar (must match binaries/scrcpy-server).
const SCRCPY_VERSION: &str = "3.1";

/// Local TCP port the decoder client connects to (Windows has no Unix
/// abstract sockets, so the host side of the forward must be TCP).
const FORWARD_PORT: u16 = 27183;

fn server_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/scrcpy-server")
}

/// Live stream child process (the scrcpy server running on the device via adb shell).
static SESSION: Mutex<Option<std::process::Child>> = Mutex::new(None);

pub fn push_server(serial: &str) -> Result<(), StreamError> {
    let path = server_path();
    if !path.exists() {
        return Err(StreamError::ServerMissing(path));
    }
    adb::run_checked(&[
        "-s", serial,
        "push", path.to_str().unwrap(), "/data/local/tmp/scrcpy-server.jar",
    ])?;
    Ok(())
}

/// Launch the scrcpy server on the device.
///
/// `serial` is the adb serial of an already-connected device; for wireless
/// devices that is `ip:port` (the same string passed to `adb connect`).
pub fn launch_stream(serial: &str) -> Result<(), StreamError> {
    stop_stream();
    ensure_connected(serial)?;
    push_server(serial)?;

    // Host-side TCP listener forwarding into the device's scrcpy socket.
    // A decoder client connects to 127.0.0.1:<FORWARD_PORT> and speaks the
    // scrcpy demuxer protocol (handshake → video/audio streams).
    adb::run_checked(&[
        "-s", serial,
        "forward", &format!("tcp:{FORWARD_PORT}"), "localabstract:scrcpy",
    ])?;

    // The whole command MUST be one shell string: modern `adb shell` execs
    // separate args directly (no shell), which drops the CLASSPATH env prefix
    // → ClassNotFoundException on device.
    let preset = BALANCED;
    let max_size_arg = if preset.max_size > 0 {
        format!(" max_size={}", preset.max_size)
    } else {
        String::new()
    };
    let shell_cmd = format!(
        "CLASSPATH=/data/local/tmp/scrcpy-server.jar \
         app_process / com.genymobile.scrcpy.Server {SCRCPY_VERSION} \
         log_level=info{max_size_arg} max_fps={} video_codec={} audio=true stay_awake=false \
         tunnel_forward=true",
        preset.max_fps, preset.codec,
    );

    let child = adb::command(&["-s", serial, "shell", &shell_cmd]).spawn()?;
    *SESSION.lock().unwrap() = Some(child);
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