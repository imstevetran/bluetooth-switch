# bluetooth-switch — Agent Guide

## Build

```powershell
$env:Path = "C:\llvm-mingw\llvm-mingw-20260602-ucrt-x86_64\bin;$env:Path"
cargo build
cargo build --release
```

Before running, copy runtime DLLs next to the binary:
```powershell
Copy-Item "C:\llvm-mingw\llvm-mingw-20260602-ucrt-x86_64\bin\libunwind.dll" target\debug\
Copy-Item "C:\llvm-mingw\llvm-mingw-20260602-ucrt-x86_64\bin\libwinpthread-1.dll" target\debug\
```

## Run

```powershell
cargo run -- list
cargo run -- gui
cargo run -- agent
cargo run -- handoff <device> --to <host>
```

## Architecture

- `src/main.rs` — CLI dispatch
- `src/cli.rs` — clap command definitions
- `src/ui.rs` — egui GUI (bt-swap gui)
- `src/agent.rs` — TCP JSON protocol listener for handoff coordination
- `src/backend/mod.rs` — BluetoothBackend trait
- `src/backend/windows.rs` — PowerShell-based backend (Get/Disable/Enable-PnpDevice)
- `src/backend/macos.rs` — blueutil-based backend
- `src/protocol.rs` — AgentMessage/DeviceInfo types
- `src/config.rs` — JSON config (aliases, peers, port)
- `src/error.rs` — BtError type

## Platform

- Toolchain: `stable-x86_64-pc-windows-gnu` (GNU, not MSVC)
- Linker: LLVM MinGW `x86_64-w64-mingw32-gcc`
- No `windows` crate — all Win32 calls go through PowerShell

## Conventions

- No comments in code unless necessary
- `BtError` for all error types
- Windows backend uses `powershell -NoProfile -Command`
- macOS backend uses `blueutil` CLI
