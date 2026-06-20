use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bluetooth-switch", version, about = "Bluetooth device handoff tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List all paired Bluetooth devices with connection status
    List,

    /// Show connection status for all paired devices
    Status,

    /// Handoff a device to another machine
    Handoff {
        /// Device name or BT address to handoff
        device: String,

        /// Target machine IP or hostname
        #[arg(short = 't', long = "to")]
        to: String,

        /// Target agent port (default: 9400)
        #[arg(short = 'p', long = "port", default_value = "9400")]
        port: u16,
    },

    /// Run as background agent (daemon)
    Agent {
        /// Daemonize (fork to background)
        #[arg(short = 'd', long = "daemonize")]
        daemonize: bool,
    },

    /// Discover other bluetooth-switch agents on the local network
    Discover,

    /// Add a device alias for easier reference
    Alias {
        /// Alias name
        name: String,
        /// Bluetooth MAC address
        address: String,
    },

    /// Remove a device alias
    Rm {
        /// Alias name or BT address to remove
        name: String,
    },

    /// Launch graphical user interface
    Gui,
}
