# bluetooth-switch

Cross-platform Bluetooth device handoff tool. Disconnect a BT device from one machine and tell another machine to reconnect — over LAN.

## Usage

```
bluetooth-switch list       List paired devices
bluetooth-switch status     Show connection status
bluetooth-switch handoff <device> --to <host>  Handoff device to another machine
bluetooth-switch agent      Run as background agent (daemon)
bluetooth-switch gui        Launch graphical interface
bluetooth-switch alias <name> <address>  Add device alias
bluetooth-switch rm <name>  Remove alias
```

## Requirements

- **Windows**: PowerShell (built-in), admin rights for device disable/enable
- **macOS**: `brew install blueutil`

## Build

```sh
cargo build --release
```
