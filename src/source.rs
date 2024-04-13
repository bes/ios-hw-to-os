use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub devices: Vec<Device>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub r#type: HardwareType,
    pub version: String,
    pub status: HardwareStatus,
    pub hardware: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HardwareType {
    #[serde(rename = "iPad")]
    IPad,
    #[serde(rename = "iPadMini")]
    IPadMini,
    #[serde(rename = "iPadAir")]
    IPadAir,
    #[serde(rename = "iPadPro")]
    IPadPro,
    #[serde(rename = "iPhone")]
    IPhone,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HardwareStatus {
    Supported,
    Unsupported,
}
