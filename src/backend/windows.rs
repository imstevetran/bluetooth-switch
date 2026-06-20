use crate::error::{BtError, Result};
use crate::protocol::DeviceInfo;
use std::process::Command;

pub struct WindowsBackend;

impl WindowsBackend {
    pub fn new() -> Result<Box<dyn super::BluetoothBackend>> {
        Ok(Box::new(Self))
    }

    fn ps(&self, script: &str) -> Result<String> {
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| BtError::Backend(format!("failed to run powershell: {e}")))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(BtError::Backend(format!("powershell error: {stderr}")));
        }
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Ok(stdout)
    }
}

impl super::BluetoothBackend for WindowsBackend {
    fn list_devices(&self) -> Result<Vec<DeviceInfo>> {
        let script = r#"
$devices = @()
$bt = Get-PnpDevice -Class Bluetooth -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -ne $null -and $_.FriendlyName -ne "" -and $_.Class -eq "Bluetooth" }
foreach ($d in $bt) {
    $connected = $d.Status -eq "OK"
    $devices += @{
        name = $d.FriendlyName
        bt_address = $d.InstanceId -replace '.*#(..)(..)(..)(..)(..)(..)#.*','$1:$2:$3:$4:$5:$6'
        connected = $connected
        paired = $true
    }
}
$devices | ConvertTo-Json
"#;
        let raw = self.ps(script)?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&raw)
            .unwrap_or_else(|_| vec![serde_json::from_str(&raw).unwrap_or_default()]);

        let mut out = Vec::new();
        for d in parsed {
            let addr = d["bt_address"].as_str().unwrap_or("").to_lowercase();
            let name = d["name"].as_str().unwrap_or("").to_string();
            if addr.is_empty() && name.is_empty() {
                continue;
            }
            out.push(DeviceInfo {
                name: if name.is_empty() { addr.clone() } else { name },
                bt_address: addr,
                connected: d["connected"].as_bool().unwrap_or(false),
                paired: d["paired"].as_bool().unwrap_or(true),
            });
        }
        Ok(out)
    }

    fn disconnect(&self, address: &str) -> Result<()> {
        let addr_upper = address.to_uppercase().replace(':', "");
        let script = format!(
            r#"$dev = Get-PnpDevice -Class Bluetooth -ErrorAction SilentlyContinue | Where-Object {{ $_.InstanceId -like '*{addr_upper}*' }}; if ($dev) {{ Disable-PnpDevice -InstanceId $dev.InstanceId -Confirm:$false -ErrorAction SilentlyContinue }}"#,
        );
        self.ps(&script)?;
        std::thread::sleep(std::time::Duration::from_millis(1500));
        Ok(())
    }

    fn connect(&self, address: &str) -> Result<()> {
        let addr_upper = address.to_uppercase().replace(':', "");
        let script = format!(
            r#"$dev = Get-PnpDevice -Class Bluetooth -ErrorAction SilentlyContinue | Where-Object {{ $_.InstanceId -like '*{addr_upper}*' }}; if ($dev) {{ Enable-PnpDevice -InstanceId $dev.InstanceId -Confirm:$false -ErrorAction SilentlyContinue }}"#,
        );
        self.ps(&script)?;
        std::thread::sleep(std::time::Duration::from_millis(2000));
        Ok(())
    }
}
