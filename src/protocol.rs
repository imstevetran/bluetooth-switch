use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentMessage {
    #[serde(rename = "handoff")]
    HandoffRequest {
        device_name: String,
        bt_address: String,
        from_host: String,
    },
    #[serde(rename = "handoff_ack")]
    HandoffAck {
        device_name: String,
        status: HandoffStatus,
        message: String,
    },
    #[serde(rename = "ping")]
    Ping {
        hostname: String,
        version: String,
    },
    #[serde(rename = "pong")]
    Pong {
        hostname: String,
        version: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    Accepted,
    Connected,
    Failed(String),
    Rejected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub bt_address: String,
    pub connected: bool,
    pub paired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub hostname: String,
    pub devices: Vec<DeviceInfo>,
}
