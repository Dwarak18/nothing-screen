use crate::mdns::DeviceMap;
use std::sync::{Arc, Mutex};

/// Device map shared between the discovery thread and Tauri commands.
pub type SharedDevices = Arc<Mutex<DeviceMap>>;

pub struct AppState {
    pub devices: SharedDevices,
    pub discovery_running: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(DeviceMap::new())),
            discovery_running: false,
        }
    }

    pub fn device_address(&self, id: &str) -> Option<String> {
        self.devices
            .lock()
            .ok()?
            .get(id)
            .map(|d| d.address.clone())
    }

    pub fn set_discovery_running(&mut self, running: bool) {
        self.discovery_running = running;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}