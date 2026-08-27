use std::{collections::HashMap, sync::mpsc, thread, time::Duration};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;

use crate::adb;

pub const PAIRING_SERVICE: &str = "_adb-tls-pairing._tcp.local.";
pub const CONNECT_SERVICE: &str = "_adb-tls-connect._tcp.local.";

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredPhone {
    /// Stable identifier: the mDNS instance fullname (survives re-resolve,
    /// and matches what ServiceRemoved reports).
    pub id: String,
    pub name: String,
    pub address: String,
    pub needs_pairing: bool,
}

/// Discovery delta consumed by the event pump in `lib.rs`.
#[derive(Debug, Clone)]
pub enum PhoneEvent {
    Found(DiscoveredPhone),
    Lost(String),
}

pub struct Discovery {
    daemon: ServiceDaemon,
    receiver: mpsc::Receiver<PhoneEvent>,
}

impl Discovery {
    pub fn start() -> Result<Self, String> {
        log::info!("discovery: creating mdns ServiceDaemon");
        let daemon = ServiceDaemon::new().map_err(|e| {
            log::error!("discovery: ServiceDaemon::new failed: {e}");
            e.to_string()
        })?;
        let (tx, rx) = mpsc::channel();

        for service in [PAIRING_SERVICE, CONNECT_SERVICE] {
            let tx = tx.clone();
            let receiver = daemon
                .browse(service)
                .map_err(|e| {
                    log::error!("discovery: browse {service} failed: {e}");
                    e.to_string()
                })?;
            log::info!("discovery: browsing {service}");
            thread::spawn(move || {
                // Blocking listen: mDNS announcements are periodic, so we must
                // keep listening. The loop exits when the browse monitor closes
                // (i.e. Discovery is dropped / browsing stopped).
                while let Ok(event) = receiver.recv() {
                    match event {
                        ServiceEvent::ServiceResolved(info) => {
                            log::info!(
                                "discovery: resolved {} {}",
                                info.get_fullname(),
                                info.get_addresses()
                                    .iter().next().map(|i| i.to_string()).unwrap_or_default()
                            );
                            if tx.send(PhoneEvent::Found(to_phone(&info))).is_err() {
                                break;
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            if tx.send(PhoneEvent::Lost(fullname)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        Ok(Self {
            daemon,
            receiver: rx,
        })
    }

    pub fn poll(&mut self) -> Vec<PhoneEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        let _ = self.daemon.stop_browse(CONNECT_SERVICE);
        let _ = self.daemon.stop_browse(PAIRING_SERVICE);
    }
}

fn to_phone(info: &ServiceInfo) -> DiscoveredPhone {
    let host = info
        .get_addresses()
        .iter()
        .next()
        .map(|ip| ip.to_string())
        .unwrap_or_default();
    DiscoveredPhone {
        id: info.get_fullname().to_owned(),
        name: info
            .get_property_val_str("name")
            .unwrap_or_else(|| info.get_fullname())
            .to_owned(),
        address: format!("{host}:{}", info.get_port()),
        needs_pairing: info.get_fullname().contains("pairing"),
    }
}

pub fn open_walkthrough() {
    let url = "https://developer.android.com/studio/debug/dev-options";
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/c", "start", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

pub fn health_check() -> Result<Vec<String>, String> {
    let mut findings = Vec::new();

    match Discovery::start() {
        Ok(mut d) => {
            // Give mDNS a realistic window to collect responses before concluding.
            thread::sleep(Duration::from_secs(3));
            let found = d
                .poll()
                .into_iter()
                .filter_map(|e| match e {
                    PhoneEvent::Found(p) => Some(p),
                    PhoneEvent::Lost(_) => None,
                })
                .count();
            if found == 0 {
                findings.push(
                    "No phones advertising wireless debugging on this network yet.".into(),
                );
            } else {
                findings.push(format!("Network OK — {found} phone(s) visible via mDNS."));
            }
        }
        Err(e) => findings.push(format!("mDNS unavailable: {e}")),
    }

    match adb::devices() {
        Ok(devices) => {
            if devices.is_empty() {
                findings.push("No adb devices paired or connected yet.".into());
            } else {
                findings.push(format!(
                    "adb OK — {} device(s): {}",
                    devices.len(),
                    devices
                        .iter()
                        .map(|d| d.model.clone().unwrap_or_else(|| d.serial.clone()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        Err(e) => findings.push(format!("adb unavailable: {e}")),
    }

    Ok(findings)
}

// Re-exported for convenience of callers building device maps.
pub type DeviceMap = HashMap<String, DiscoveredPhone>;