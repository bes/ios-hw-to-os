use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    #[serde(default)]
    pub metadata: Metadata,
    pub devices: Vec<Device>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default)]
    pub last_updated: NaiveDate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub r#type: HardwareType,
    pub version: String,
    pub status: HardwareStatus,
    pub hardware: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
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

impl HardwareType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IPad => "iPad",
            Self::IPadMini => "iPadMini",
            Self::IPadAir => "iPadAir",
            Self::IPadPro => "iPadPro",
            Self::IPhone => "iPhone",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::IPad => "iPad",
            Self::IPadMini => "iPad Mini",
            Self::IPadAir => "iPad Air",
            Self::IPadPro => "iPad Pro",
            Self::IPhone => "iPhone",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HardwareStatus {
    Unreleased,
    Supported,
    Unsupported,
}

impl HardwareStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unreleased => "Unreleased",
            Self::Supported => "Supported",
            Self::Unsupported => "Unsupported",
        }
    }
}

/// Parse version ("18.7.10") into components.
///
/// Returns `None` for anything that is not numeric (this filters out non-release version strings)
pub fn parse_version(version: &str) -> Option<Vec<u32>> {
    version.split('.').map(|part| part.parse().ok()).collect()
}

/// Split "iPad8,12" into ("iPad", 8, 12).
pub fn parse_identifier(identifier: &str) -> Option<(&'static str, u32, u32)> {
    let (family, minor) = identifier.split_once(',')?;
    let digits = family.find(|c: char| c.is_ascii_digit())?;
    let prefix = match &family[..digits] {
        "iPhone" => "iPhone",
        "iPad" => "iPad",
        _ => return None,
    };
    Some((prefix, family[digits..].parse().ok()?, minor.parse().ok()?))
}

pub async fn read(path: &Path) -> Result<Source, Box<dyn Error>> {
    let yaml = fs::read_to_string(path)
        .await
        .map_err(|e| format!("reading {}: {e}", path.display()))?;
    serde_yaml::from_str(&yaml).map_err(|e| format!("parsing {}: {e}", path.display()).into())
}

pub async fn write(source: &Source, path: &Path) -> Result<(), Box<dyn Error>> {
    fs::write(path, source.to_string())
        .await
        .map_err(|e| format!("writing {}: {e}", path.display()).into())
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "metadata:")?;
        writeln!(f, "  last_updated: {}", self.metadata.last_updated)?;
        write!(f, "devices:")?;

        let mut section = None;
        let mut subsection = None;
        for device in &self.devices {
            if section != Some(device.r#type) {
                section = Some(device.r#type);
                subsection = None;
                let title = device.r#type.title();
                write!(f, "\n\n# {title}\n# {title}\n# {title}")?;
            }
            if subsection != Some(device.status) {
                subsection = Some(device.status);
                let (status, title) = (device.status.as_str(), device.r#type.title());
                write!(f, "\n\n## {status} {title} versions")?;
            }
            write!(f, "\n- type: {}", device.r#type.as_str())?;
            write!(f, "\n  version: \"{}\"", device.version)?;
            write!(f, "\n  status: {}", device.status.as_str())?;
            write!(f, "\n  hardware:")?;
            for hardware in &device.hardware {
                write!(f, "\n  - {hardware}")?;
            }
        }
        writeln!(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing_rejects_appledb_non_releases() {
        assert_eq!(parse_version("18.7.10"), Some(vec![18, 7, 10]));
        assert_eq!(parse_version("27.0"), Some(vec![27, 0]));
        for not_a_release in ["27.0 SDK", "27.0 Simulator", "26.3 (a)", "9.3.3 beta", ""] {
            assert_eq!(parse_version(not_a_release), None, "{not_a_release}");
        }
        assert!(parse_version("27.0") > parse_version("26.7"));
        assert!(parse_version("26.7") > parse_version("18.7.10"));
    }

    #[test]
    fn identifier_parsing_rejects_other_hardware() {
        assert_eq!(parse_identifier("iPad8,12"), Some(("iPad", 8, 12)));
        assert_eq!(parse_identifier("iPhone19,2"), Some(("iPhone", 19, 2)));
        for other in ["iPod9,1", "AppleTV14,1", "iOS SDK 27.0", "iPad", "Mac14,2"] {
            assert_eq!(parse_identifier(other), None, "{other}");
        }
    }

    #[test]
    fn yaml_round_trips_through_serde() {
        let source = Source {
            metadata: Metadata {
                last_updated: NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            },
            devices: vec![
                Device {
                    r#type: HardwareType::IPadMini,
                    version: "27.0".to_string(),
                    status: HardwareStatus::Supported,
                    hardware: vec!["iPad16,1".to_string(), "iPad16,2".to_string()],
                },
                Device {
                    r#type: HardwareType::IPadMini,
                    version: "15.8.8".to_string(),
                    status: HardwareStatus::Unsupported,
                    hardware: vec!["iPad5,1".to_string()],
                },
            ],
        };

        let yaml = source.to_string();
        assert_eq!(
            yaml,
            "metadata:\n  \
             last_updated: 2026-09-19\n\
             devices:\n\n\
             # iPad Mini\n# iPad Mini\n# iPad Mini\n\n\
             ## Supported iPad Mini versions\n\
             - type: iPadMini\n  version: \"27.0\"\n  status: Supported\n  hardware:\n  - iPad16,1\n  - iPad16,2\n\n\
             ## Unsupported iPad Mini versions\n\
             - type: iPadMini\n  version: \"15.8.8\"\n  status: Unsupported\n  hardware:\n  - iPad5,1\n"
        );

        let reparsed: Source = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(
            reparsed.metadata.last_updated,
            NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
        );
        assert_eq!(reparsed.to_string(), yaml);
    }
}
