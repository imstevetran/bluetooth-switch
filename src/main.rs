mod agent;
mod backend;
mod cli;
mod config;
mod error;
mod protocol;
mod ui;

use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use error::Result;
use protocol::AgentMessage;
use std::net::TcpStream;
use std::io::{Read, Write};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let cli = Cli::parse();
    let mut config = Config::load();

    match &cli.command {
        Command::List => cmd_list(&config),
        Command::Status => cmd_status(&config),
        Command::Handoff { device, to, port } => cmd_handoff(&config, device, to, *port),
        Command::Agent { daemonize } => cmd_agent(config, *daemonize),
        Command::Discover => cmd_discover(&config),
        Command::Alias { name, address } => cmd_alias(&mut config, name, address),
        Command::Rm { name } => cmd_rm(&mut config, name),
        Command::Gui => { ui::run_gui(); Ok(()) }
    }
}

fn cmd_list(config: &Config) -> Result<()> {
    let backend = backend::create_backend()?;
    let devices = backend.list_devices()?;

    println!("{:<30} {:<20} {}", "Name", "Address", "Status");
    println!("{:-<30} {:-<20} {:-<10}", "", "", "");

    for d in &devices {
        let status = if d.connected { "connected" } else { "paired" };
        let name = resolve_alias(config, d);
        println!("{:<30} {:<20} {}", name, d.bt_address, status);
    }

    Ok(())
}

fn cmd_status(config: &Config) -> Result<()> {
    let backend = backend::create_backend()?;
    let devices = backend.list_devices()?;

    println!("Host: {}", config.identity.hostname);
    println!();

    let connected: Vec<_> = devices.iter().filter(|d| d.connected).collect();
    println!("Connected ({}):", connected.len());
    if connected.is_empty() {
        println!("  (none)");
    } else {
        for d in &connected {
            let name = resolve_alias(config, d);
            println!("  - {} ({})", name, d.bt_address);
        }
    }

    println!();
    let available: Vec<_> = devices.iter().filter(|d| !d.connected).collect();
    println!("Available ({}):", available.len());
    if available.is_empty() {
        println!("  (none)");
    } else {
        for d in &available {
            let name = resolve_alias(config, d);
            println!("  - {} ({})", name, d.bt_address);
        }
    }

    Ok(())
}

fn cmd_handoff(config: &Config, device: &str, to: &str, port: u16) -> Result<()> {
    let backend = backend::create_backend()?;

    let devices = backend.list_devices()?;

    let target = find_device(config, &devices, device)
        .ok_or_else(|| error::BtError::DeviceNotFound(device.into()))?;

    println!("Disconnecting {} ({}) from {}...", target.name, target.bt_address, config.identity.hostname);
    backend.disconnect(&target.bt_address)?;
    println!("Disconnected.");

    let addr = format!("{to}:{port}");
    println!("Connecting to agent at {addr}...");

    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| error::BtError::Backend(format!("cannot connect to {addr}: {e}")))?;

    let msg = AgentMessage::HandoffRequest {
        device_name: target.name.clone(),
        bt_address: target.bt_address.clone(),
        from_host: config.identity.hostname.clone(),
    };

    let mut buf = serde_json::to_string(&msg)?;
    buf.push('\n');

    stream.write_all(buf.as_bytes())
        .map_err(|e| error::BtError::Backend(format!("send handoff: {e}")))?;

    let mut resp = String::new();
    stream.read_to_string(&mut resp)
        .map_err(|e| error::BtError::Backend(format!("read response: {e}")))?;

    if let Ok(ack) = serde_json::from_str::<AgentMessage>(resp.trim()) {
        match ack {
            AgentMessage::HandoffAck { status, message, .. } => {
                match status {
                    protocol::HandoffStatus::Connected => {
                        println!("✅ Handoff complete: {message}");
                    }
                    protocol::HandoffStatus::Accepted => {
                        println!("Handoff accepted, waiting for connection...");
                    }
                    protocol::HandoffStatus::Failed(reason) => {
                        println!("❌ Handoff failed: {reason}");
                    }
                    protocol::HandoffStatus::Rejected(reason) => {
                        println!("⛔ Handoff rejected: {reason}");
                    }
                }
            }
            _ => {
                println!("Unexpected response from agent");
            }
        }
    }

    Ok(())
}

fn cmd_agent(config: Config, _daemonize: bool) -> Result<()> {
    let backend = backend::create_backend()?;
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| error::BtError::Backend(format!("tokio runtime: {e}")))?;

    runtime.block_on(async {
        let agent = agent::Agent::new(backend, config);
        agent.run().await
    })
}

fn cmd_discover(_config: &Config) -> Result<()> {
    println!("Scanning for bluetooth-switch agents on LAN...");
    println!("(use: bluetooth-switch agent to start an agent)");
    Ok(())
}

fn cmd_alias(config: &mut Config, name: &str, address: &str) -> Result<()> {
    config.devices.retain(|d| d.name != name && d.bt_address.as_deref() != Some(address));
    config.devices.push(config::DeviceAlias {
        name: name.into(),
        bt_address: Some(address.into()),
    });
    config.save();
    println!("Alias saved: {name} -> {address}");
    Ok(())
}

fn cmd_rm(config: &mut Config, name: &str) -> Result<()> {
    let before = config.devices.len();
    config.devices.retain(|d| d.name != name && d.bt_address.as_deref() != Some(name));
    config.save();
    let removed = before - config.devices.len();
    if removed > 0 {
        println!("Removed {removed} alias(es)");
    } else {
        println!("No matching alias found");
    }
    Ok(())
}

fn find_device<'a>(
    config: &Config,
    devices: &'a [protocol::DeviceInfo],
    query: &str,
) -> Option<&'a protocol::DeviceInfo> {
    let q = query.to_lowercase();

    if let Some(alias) = config.devices.iter().find(|d| d.name.to_lowercase() == q) {
        if let Some(addr) = &alias.bt_address {
            if let Some(d) = devices.iter().find(|d| d.bt_address == *addr) {
                return Some(d);
            }
        }
    }

    devices
        .iter()
        .find(|d| d.name.to_lowercase() == q || d.bt_address == q)
}

fn resolve_alias(config: &Config, device: &protocol::DeviceInfo) -> String {
    config
        .devices
        .iter()
        .find(|a| a.bt_address.as_deref() == Some(&device.bt_address))
        .map(|a| a.name.clone())
        .unwrap_or_else(|| device.name.clone())
}
