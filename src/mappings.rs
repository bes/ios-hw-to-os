use crate::source::{parse_version, Source};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Mappings<'a> {
    /// The most modern OS version
    #[serde(rename = "highestVersion")]
    pub highest_version: &'a str,
    #[serde(borrow)]
    pub devices: Vec<Mapping<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mapping<'a> {
    pub hardware: &'a str,
    pub version: &'a str,
}

impl<'a> Mappings<'a> {
    fn new(source: &'a Source) -> Result<Self, Box<dyn Error>> {
        let devices: Vec<Mapping<'a>> = source
            .devices
            .iter()
            .flat_map(|device| {
                device.hardware.iter().map(|hardware| Mapping {
                    hardware,
                    version: &device.version,
                })
            })
            .collect();
        let highest_version = devices
            .iter()
            .map(|mapping| mapping.version)
            .max_by_key(|version| parse_version(version))
            .ok_or("no hardware to map")?;
        Ok(Self {
            highest_version,
            devices,
        })
    }
}

pub async fn write(source: &Source, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut json = serde_json::to_string_pretty(&Mappings::new(source)?)?;
    json.push('\n');
    fs::write(path, json)
        .await
        .map_err(|e| format!("writing {}: {e}", path.display()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{Device, HardwareStatus, HardwareType, Metadata};

    fn source(versions: &[&str]) -> Source {
        Source {
            metadata: Metadata::default(),
            devices: versions
                .iter()
                .enumerate()
                .map(|(index, version)| Device {
                    r#type: HardwareType::IPhone,
                    version: version.to_string(),
                    status: HardwareStatus::Supported,
                    hardware: vec![format!("iPhone{index},1")],
                })
                .collect(),
        }
    }

    #[test]
    fn highest_version_compares_components_not_text() {
        let source = source(&["18.7.10", "27.0", "9.3.6", "26.7"]);
        assert_eq!(Mappings::new(&source).unwrap().highest_version, "27.0");
    }

    #[test]
    fn highest_version_ignores_devices_without_hardware() {
        let mut source = source(&["26.7", "27.0"]);
        source.devices[1].hardware.clear();
        assert_eq!(Mappings::new(&source).unwrap().highest_version, "26.7");
    }

    #[test]
    fn mapping_without_hardware_is_an_error() {
        assert!(Mappings::new(&source(&[])).is_err());
    }
}
