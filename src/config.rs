use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent_port: u16,
    pub identity: Identity,
    pub devices: Vec<DeviceAlias>,
    pub peers: Vec<Peer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAlias {
    pub name: String,
    pub bt_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub hostname: String,
    pub address: String,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent_port: 9400,
            identity: Identity {
                hostname: hostname().unwrap_or_else(|| "unknown".into()),
            },
            devices: Vec::new(),
            peers: Vec::new(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let mut p = dirs();
        p.push("config.json");
        p
    }

    pub fn load() -> Self {
        let p = Self::path();
        if p.exists() {
            std::fs::read_to_string(&p)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        if let Some(parent) = Self::path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), s);
        }
    }
}

fn dirs() -> PathBuf {
    let base = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(".bt-swap")
}

fn hostname() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
}
