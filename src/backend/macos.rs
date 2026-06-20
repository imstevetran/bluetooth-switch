use crate::error::{BtError, Result};
use crate::protocol::DeviceInfo;
use std::process::Command;

pub struct MacOsBackend;

impl MacOsBackend {
    pub fn new() -> Result<Self> {
        let ok = Command::new("which")
            .arg("blueutil")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            return Err(BtError::Backend(
                "blueutil not found. Install with: brew install blueutil".into(),
            ));
        }
        Ok(Self)
    }

    fn blueutil(&self, args: &[&str]) -> Result<String> {
        let out = Command::new("blueutil")
            .args(args)
            .output()
            .map_err(|e| BtError::Backend(format!("failed to run blueutil: {e}")))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(BtError::Backend(format!("blueutil error: {stderr}")));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

impl super::BluetoothBackend for MacOsBackend {
    fn list_devices(&self) -> Result<Vec<DeviceInfo>> {
        let raw = self.blueutil(&["--paired", "--format", "json"])?;
        let devices: Vec<serde_json::Value> = serde_json::from_str(&raw)
            .map_err(|e| BtError::Backend(format!("parse blueutil output: {e}")))?;

        let connected_raw = self.blueutil(&["--connected", "--format", "json"]).ok();
        let connected_addrs: std::collections::HashSet<String> = connected_raw
            .and_then(|r| serde_json::from_str::<Vec<serde_json::Value>>(&r).ok())
            .map(|list| {
                list.into_iter()
                    .filter_map(|d| d["address"].as_str().map(|s| s.to_lowercase()))
                    .collect()
            })
            .unwrap_or_default();

        let mut out = Vec::new();
        for d in devices {
            let addr = d["address"].as_str().unwrap_or("").to_lowercase();
            let name = d["name"]
                .as_str()
                .filter(|n| !n.is_empty() && *n != "-")
                .unwrap_or(&addr)
                .to_string();
            out.push(DeviceInfo {
                name,
                bt_address: addr.clone(),
                connected: connected_addrs.contains(&addr),
                paired: true,
            });
        }
        Ok(out)
    }

    fn disconnect(&self, address: &str) -> Result<()> {
        self.blueutil(&["--disconnect", address])?;
        Ok(())
    }

    fn connect(&self, address: &str) -> Result<()> {
        self.blueutil(&["--connect", address])?;
        Ok(())
    }
}
