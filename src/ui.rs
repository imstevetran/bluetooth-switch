use crate::backend::{self, BluetoothBackend};
use crate::config::Config;
use crate::protocol::DeviceInfo;
use eframe::egui;
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(PartialEq)]
enum Tab {
    Devices,
    Agent,
    Aliases,
}

pub struct BtSwapApp {
    backend: Box<dyn BluetoothBackend>,
    config: Config,
    devices: Vec<DeviceInfo>,
    status: String,
    error: String,
    tab: Tab,
    alias_name: String,
    alias_address: String,
    handoff_addr: String,
    handoff_port: String,
    agent_running: bool,
    agent_log: Vec<String>,
    disconnect_addr: Option<String>,
    handoff_trigger: Option<(String, String)>,
}

impl BtSwapApp {
    pub fn new(backend: Box<dyn BluetoothBackend>, config: Config) -> Self {
        let mut app = Self {
            devices: Vec::new(),
            status: String::new(),
            error: String::new(),
            tab: Tab::Devices,
            alias_name: String::new(),
            alias_address: String::new(),
            handoff_addr: String::new(),
            handoff_port: String::from("9400"),
            agent_running: false,
            agent_log: Vec::new(),
            disconnect_addr: None,
            handoff_trigger: None,
            backend,
            config,
        };
        app.refresh_devices();
        app
    }

    fn refresh_devices(&mut self) {
        match self.backend.list_devices() {
            Ok(d) => {
                self.devices = d;
                self.status = format!("{} device(s) found", self.devices.len());
                self.error.clear();
            }
            Err(e) => {
                self.error = format!("Failed to list devices: {e}");
            }
        }
    }

    fn resolve_alias(&self, addr: &str) -> Option<&str> {
        self.config
            .devices
            .iter()
            .find(|a| a.bt_address.as_deref() == Some(addr))
            .map(|a| a.name.as_str())
    }
}

impl eframe::App for BtSwapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("bluetooth-switch");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                } else if !self.status.is_empty() {
                    ui.label(&self.status);
                }
            });
        });

        egui::SidePanel::left("tabs")
            .resizable(false)
            .default_width(120.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    if ui
                        .selectable_label(self.tab == Tab::Devices, "Devices")
                        .clicked()
                    {
                        self.tab = Tab::Devices;
                        self.refresh_devices();
                    }
                    if ui
                        .selectable_label(self.tab == Tab::Agent, "Agent")
                        .clicked()
                    {
                        self.tab = Tab::Agent;
                    }
                    if ui
                        .selectable_label(self.tab == Tab::Aliases, "Aliases")
                        .clicked()
                    {
                        self.tab = Tab::Aliases;
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Devices => self.show_devices(ui),
            Tab::Agent => self.show_agent(ui),
            Tab::Aliases => self.show_aliases(ui),
        });

        if let Some(addr) = self.disconnect_addr.take() {
            match self.backend.disconnect(&addr) {
                Ok(()) => {
                    self.status = format!("Disconnected {addr}");
                    self.refresh_devices();
                }
                Err(e) => self.error = format!("Disconnect failed: {e}"),
            }
        }

        if let Some((name, addr)) = self.handoff_trigger.take() {
            self.handoff_addr = addr;
            self.status = format!("Ready to handoff {name}");
        }
    }
}

impl BtSwapApp {
    fn show_devices(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Paired Devices");
            if ui.button("\u{1F504} Refresh").clicked() {
                self.refresh_devices();
            }
        });
        ui.separator();
        ui.add_space(4.0);

        if self.devices.is_empty() {
            ui.label("No Bluetooth devices found.");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("devices_grid")
                .striped(true)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Address");
                    ui.strong("Status");
                    ui.strong("Action");
                    ui.end_row();

                    for d in &self.devices {
                        let name = self.resolve_alias(&d.bt_address).unwrap_or(&d.name);
                        ui.label(name);
                        ui.monospace(&d.bt_address);
                        if d.connected {
                            ui.colored_label(egui::Color32::GREEN, "Connected");
                            if ui.button("Disconnect").clicked() {
                                self.disconnect_addr = Some(d.bt_address.clone());
                            }
                        } else {
                            ui.colored_label(egui::Color32::GRAY, "Paired");
                            if ui.button("Handoff \u{2192}").clicked() {
                                self.handoff_trigger =
                                    Some((d.name.clone(), d.bt_address.clone()));
                            }
                        }
                        ui.end_row();
                    }
                });
        });

        if !self.handoff_addr.is_empty() {
            ui.separator();
            ui.add_space(4.0);
            ui.heading("Handoff");
            ui.horizontal(|ui| {
                ui.label("Target host:");
                ui.text_edit_singleline(&mut self.handoff_addr);
            });
            ui.horizontal(|ui| {
                ui.label("Port:");
                ui.text_edit_singleline(&mut self.handoff_port);
            });
            if ui.button("Send Handoff").clicked() {
                self.do_handoff();
            }
        }
    }

    fn show_agent(&mut self, ui: &mut egui::Ui) {
        ui.heading("Agent");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Port:");
            ui.add(egui::Slider::new(&mut self.config.agent_port, 1024..=65535));
        });

        ui.horizontal(|ui| {
            let label = if self.agent_running {
                "Stop Agent"
            } else {
                "Start Agent"
            };
            if ui.button(label).clicked() {
                if self.agent_running {
                    self.agent_running = false;
                    self.agent_log.push("Agent stopped".into());
                } else {
                    self.agent_log.push(format!(
                        "Agent starting on port {}...",
                        self.config.agent_port
                    ));
                    self.status =
                        "Agent mode not available in GUI yet (use CLI: bluetooth-switch agent)".into();
                }
            }
        });

        ui.add_space(8.0);
        ui.label("Hostname:");
        ui.label(&self.config.identity.hostname);

        ui.add_space(8.0);
        ui.strong("Log:");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for line in &self.agent_log {
                    ui.label(line);
                }
            });
    }

    fn show_aliases(&mut self, ui: &mut egui::Ui) {
        ui.heading("Device Aliases");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.alias_name);
            ui.label("Address:");
            ui.text_edit_singleline(&mut self.alias_address);
            if ui.button("Add Alias").clicked() {
                let name = self.alias_name.trim().to_string();
                let addr = self.alias_address.trim().to_string();
                if !name.is_empty() && !addr.is_empty() {
                    self.config
                        .devices
                        .retain(|d| d.name != name && d.bt_address.as_deref() != Some(&addr));
                    self.config.devices.push(crate::config::DeviceAlias {
                        name: name.clone(),
                        bt_address: Some(addr),
                    });
                    self.config.save();
                    self.status = format!("Alias '{name}' saved");
                    self.alias_name.clear();
                    self.alias_address.clear();
                    self.refresh_devices();
                }
            }
        });

        ui.add_space(8.0);
        if self.config.devices.is_empty() {
            ui.label("No aliases defined.");
        } else {
            egui::Grid::new("aliases_grid")
                .striped(true)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Address");
                    ui.strong("");
                    ui.end_row();

                    let mut to_remove: Option<String> = None;
                    for a in &self.config.devices {
                        ui.label(&a.name);
                        ui.monospace(a.bt_address.as_deref().unwrap_or(""));
                        if ui.button("Delete").clicked() {
                            to_remove = Some(a.name.clone());
                        }
                        ui.end_row();
                    }

                    if let Some(name) = to_remove {
                        self.config.devices.retain(|d| d.name != name);
                        self.config.save();
                        self.refresh_devices();
                    }
                });
        }
    }

    fn do_handoff(&mut self) {
        let addr = self.handoff_addr.clone();
        let port = self.handoff_port.clone();

        let devices = match self.backend.list_devices() {
            Ok(d) => d,
            Err(e) => {
                self.error = format!("Failed to list devices for handoff: {e}");
                return;
            }
        };

        let target = devices.iter().find(|d| d.bt_address == addr);
        let target = match target {
            Some(d) => d.clone(),
            None => {
                self.error = format!("Device {addr} not found in paired list");
                return;
            }
        };

        match self.backend.disconnect(&target.bt_address) {
            Ok(()) => {
                self.status = format!("Disconnected {} ({})", target.name, target.bt_address);
            }
            Err(e) => {
                self.error = format!("Disconnect failed: {e}");
                return;
            }
        }

        let remote = format!("{}:{}", &self.handoff_addr, port);
        match TcpStream::connect(&remote) {
            Ok(mut stream) => {
                let msg = crate::protocol::AgentMessage::HandoffRequest {
                    device_name: target.name.clone(),
                    bt_address: target.bt_address.clone(),
                    from_host: self.config.identity.hostname.clone(),
                };
                let mut buf = serde_json::to_string(&msg).unwrap_or_default();
                buf.push('\n');
                let _ = stream.write_all(buf.as_bytes());
                let mut resp = String::new();
                let _ = stream.read_to_string(&mut resp);
                if let Ok(ack) =
                    serde_json::from_str::<crate::protocol::AgentMessage>(resp.trim())
                {
                    match ack {
                        crate::protocol::AgentMessage::HandoffAck {
                            status, message, ..
                        } => match status {
                            crate::protocol::HandoffStatus::Connected => {
                                self.status = format!("Handoff complete: {message}");
                                self.handoff_addr.clear();
                            }
                            crate::protocol::HandoffStatus::Accepted => {
                                self.status = "Handoff accepted, waiting for connection".into();
                            }
                            crate::protocol::HandoffStatus::Failed(reason) => {
                                self.error = format!("Handoff failed: {reason}");
                            }
                            crate::protocol::HandoffStatus::Rejected(reason) => {
                                self.error = format!("Handoff rejected: {reason}");
                            }
                        },
                        _ => {
                            self.error = "Unexpected response from agent".into();
                        }
                    }
                }
            }
            Err(e) => {
                self.error = format!("Cannot connect to agent at {remote}: {e}");
            }
        }
    }
}

pub fn run_gui() {
    let backend = backend::create_backend().expect("Failed to create Bluetooth backend");
    let config = Config::load();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("bluetooth-switch"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "bluetooth-switch",
        native_options,
        Box::new(|_cc| Ok(Box::new(BtSwapApp::new(backend, config)))),
    );
}
