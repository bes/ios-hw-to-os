use crate::source::Source;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Mappings<'a> {
    #[serde(borrow)]
    pub devices: Vec<Mapping<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mapping<'a> {
    pub hardware: &'a str,
    pub version: &'a str,
}

pub async fn write(source: &Source, path: &Path) -> Result<(), Box<dyn Error>> {
    let mappings = Mappings {
        devices: source
            .devices
            .iter()
            .flat_map(|device| {
                device.hardware.iter().map(|hardware| Mapping {
                    hardware,
                    version: &device.version,
                })
            })
            .collect(),
    };
    let mut json = serde_json::to_string_pretty(&mappings)?;
    json.push('\n');
    fs::write(path, json)
        .await
        .map_err(|e| format!("writing {}: {e}", path.display()).into())
}
