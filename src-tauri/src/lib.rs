mod adb;
mod mdns;
mod scrcpy;
mod state;

use mdns::{Discovery, PhoneEvent};
use state::AppState;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// Spawn the background discovery pump: polls mDNS events and forwards them
/// to the frontend as `phone-found` / `phone-lost` events.
fn spawn_discovery_pump(app: AppHandle, devices: state::SharedDevices) -> Result<(), String> {
    let mut discovery = Discovery::start()?;
    std::thread::spawn(move || {
        loop {
            for event in discovery.poll() {
                match event {
                    PhoneEvent::Found(phone) => {
                        if let Ok(mut map) = devices.lock() {
                            map.insert(phone.id.clone(), phone.clone());
                        }
                        let _ = app.emit("phone-found", &phone);
                    }
                    PhoneEvent::Lost(id) => {
                        if let Ok(mut map) = devices.lock() {
                            map.remove(&id);
                        }
                        let _ = app.emit("phone-lost", &id);
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
    });
    Ok(())
}

#[tauri::command]
fn start_discovery(
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    {
        let mut st = state.lock().map_err(|e| e.to_string())?;
        if st.discovery_running {
            return Ok(());
        }
        st.set_discovery_running(true);
    }

    let devices = state.lock().map_err(|e| e.to_string())?.devices.clone();

    if let Err(e) = spawn_discovery_pump(app, devices) {
        // Roll back the flag so a later retry can succeed.
        if let Ok(mut st) = state.lock() {
            st.set_discovery_running(false);
        }
        return Err(e);
    }
    Ok(())
}

#[tauri::command]
fn pair_device(address: String, code: String) -> Result<(), String> {
    adb::ensure_server().map_err(|e| e.to_string())?;
    adb::pair(&address, &code).map_err(|e| e.to_string())
}

#[tauri::command]
fn connect_device(
    state: tauri::State<'_, Mutex<AppState>>,
    device_id: String,
) -> Result<(), String> {
    let addr = {
        let st = state.lock().map_err(|e| e.to_string())?;
        st.device_address(&device_id)
            .ok_or_else(|| "unknown device".to_string())?
    };
    adb::ensure_server().map_err(|e| e.to_string())?;
    adb::connect(&addr).map_err(|e| e.to_string())?;

    // For wireless devices the `ip:port` address *is* the adb serial.
    scrcpy::launch_stream(&addr).map_err(|e| format!("stream launch failed: {e}"))
}

#[tauri::command]
fn disconnect_stream() {
    scrcpy::stop_stream();
}

#[tauri::command]
fn open_walkthrough() {
    mdns::open_walkthrough();
}

#[tauri::command]
fn run_health_check() -> Result<Vec<String>, String> {
    mdns::health_check()
}

pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .manage(Mutex::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![
            start_discovery,
            pair_device,
            connect_device,
            disconnect_stream,
            open_walkthrough,
            run_health_check
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}