use crate::error::Result;
use crate::protocol::DeviceInfo;

pub trait BluetoothBackend: Send + Sync {
    fn list_devices(&self) -> Result<Vec<DeviceInfo>>;
    fn disconnect(&self, address: &str) -> Result<()>;
    fn connect(&self, address: &str) -> Result<()>;
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacOsBackend;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::WindowsBackend;

pub fn create_backend() -> Result<Box<dyn BluetoothBackend>> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(MacOsBackend::new()?))
    }
    #[cfg(windows)]
    {
        WindowsBackend::new()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Err(crate::error::BtError::UnsupportedPlatform)
    }
}
