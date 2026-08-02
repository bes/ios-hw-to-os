use crate::source::{self, Device, HardwareStatus, HardwareType, Metadata, Source};
use chrono::NaiveDate;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::Write;

const FIRMWARE_URL: &str = "https://api.appledb.dev/ios/iOS/main.json";
const DEVICE_URL: &str = "https://api.appledb.dev/device/main.json";

/// Only track iOS 15+
const MIN_MAJOR_VERSION: u32 = 15;

#[derive(Debug, Deserialize)]
struct Firmware {
    #[serde(rename = "osStr")]
    os_str: String,
    version: String,
    #[serde(default, deserialize_with = "release_date")]
    released: Option<NaiveDate>,
    #[serde(default)]
    beta: bool,
    #[serde(default)]
    rc: bool,
    #[serde(default)]
    internal: bool,
    #[serde(default)]
    rsr: bool,
    #[serde(default, rename = "hideFromLatestVersions")]
    hide_from_latest_versions: bool,
    #[serde(default, rename = "deviceMap")]
    device_map: Vec<String>,
}

impl Firmware {
    fn shipped_version(&self, today: NaiveDate) -> Option<Vec<u32>> {
        if self.os_str != "iOS" && self.os_str != "iPadOS" {
            return None;
        }
        if self.beta || self.rc || self.internal || self.rsr || self.hide_from_latest_versions {
            return None;
        }
        if self.released.is_none_or(|released| released > today) {
            return None;
        }
        source::parse_version(&self.version)
    }
}

#[derive(Debug, Deserialize)]
struct CatalogDevice {
    #[serde(rename = "type")]
    device_type: String,
    #[serde(default)]
    identifier: Vec<String>,
    #[serde(default, deserialize_with = "first_release_date")]
    released: Option<NaiveDate>,
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()
}

fn release_date<'de, D: Deserializer<'de>>(de: D) -> Result<Option<NaiveDate>, D::Error> {
    let raw = Option::<String>::deserialize(de)?;
    Ok(raw.as_deref().and_then(parse_date))
}

fn first_release_date<'de, D: Deserializer<'de>>(de: D) -> Result<Option<NaiveDate>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Once(String),
        Repeatedly(Vec<String>),
        Never,
    }

    Ok(match Raw::deserialize(de)? {
        Raw::Once(date) => parse_date(&date),
        Raw::Repeatedly(dates) => dates.iter().filter_map(|d| parse_date(d)).min(),
        Raw::Never => None,
    })
}

fn hardware_type(device_type: &str) -> Option<HardwareType> {
    Some(match device_type {
        "iPhone" => HardwareType::IPhone,
        "iPad" => HardwareType::IPad,
        "iPad Air" => HardwareType::IPadAir,
        "iPad mini" => HardwareType::IPadMini,
        "iPad Pro" => HardwareType::IPadPro,
        _ => return None,
    })
}

/// The AppleDB `osStr`
fn os_str(hardware_type: HardwareType) -> &'static str {
    match hardware_type {
        HardwareType::IPhone => "iOS",
        _ => "iPadOS",
    }
}

/// The newest OS a hardware can run
struct Latest<'a> {
    version: Vec<u32>,
    text: &'a str,
    os_str: &'a str,
}

/// Groups in the source.yaml order
type GroupKey = (HardwareType, HardwareStatus, Reverse<Vec<u32>>);

/// The version text from AppleDB and the hardware
type Group<'a> = (&'a str, Vec<Entry<'a>>);

/// One row of a `hardware:` list
struct Entry<'a> {
    identifier: &'a str,
    family: u32,
    minor: u32,
}

fn detail(error: impl Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let _ = write!(message, ": {cause}");
        source = cause.source();
    }
    message
}

async fn fetch<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T, Box<dyn Error>> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {}", detail(e)))?
        .error_for_status()
        .map_err(|e| format!("GET {url}: {}", detail(e)))?;
    let data = response
        .json::<T>()
        .await
        .map_err(|e| format!("GET {url}: {}", detail(e)))?;
    Ok(data)
}

pub async fn fetch_source() -> Result<Source, Box<dyn Error>> {
    let today = chrono::Utc::now().date_naive();
    let client = Client::builder()
        .build()
        .map_err(|e| format!("building HTTP client: {e}"))?;

    let (firmwares, catalog): (Vec<Firmware>, Vec<CatalogDevice>) =
        tokio::try_join!(fetch(&client, FIRMWARE_URL), fetch(&client, DEVICE_URL))?;

    eprintln!(
        "AppleDB: {} firmware builds, {} devices",
        firmwares.len(),
        catalog.len()
    );
    Ok(build(&firmwares, &catalog, today))
}

fn build(firmwares: &[Firmware], catalog: &[CatalogDevice], today: NaiveDate) -> Source {
    let mut ceiling: HashMap<&str, (Vec<u32>, &str)> = HashMap::new();
    let mut latest: HashMap<&str, Latest> = HashMap::new();

    for firmware in firmwares {
        let Some(version) = firmware.shipped_version(today) else {
            continue;
        };
        let os_ceiling = ceiling
            .entry(&firmware.os_str)
            .or_insert_with(|| (Vec::new(), ""));
        if version > os_ceiling.0 {
            *os_ceiling = (version.clone(), &firmware.version);
        }
        for identifier in &firmware.device_map {
            if source::parse_identifier(identifier).is_none() {
                continue;
            }
            if latest
                .get(identifier.as_str())
                .is_none_or(|best| version > best.version)
            {
                latest.insert(
                    identifier,
                    Latest {
                        version: version.clone(),
                        text: &firmware.version,
                        os_str: &firmware.os_str,
                    },
                );
            }
        }
    }

    let mut known: HashMap<&str, (HardwareType, NaiveDate)> = HashMap::new();
    for device in catalog {
        let Some(hardware_type) = hardware_type(&device.device_type) else {
            continue;
        };
        let Some(released) = device.released else {
            continue;
        };
        for identifier in &device.identifier {
            known.insert(identifier, (hardware_type, released));
        }
    }

    let mut groups: BTreeMap<GroupKey, Group> = BTreeMap::new();
    for (identifier, &(hardware_type, released)) in &known {
        let Some((_, family, minor)) = source::parse_identifier(identifier) else {
            continue;
        };
        let (version, text, status) = match latest.get(*identifier) {
            Some(best) => {
                let status = if ceiling.get(best.os_str).map(|c| &c.0) == Some(&best.version) {
                    HardwareStatus::Supported
                } else {
                    HardwareStatus::Unsupported
                };
                (best.version.clone(), best.text, status)
            }
            None if released > today => {
                let Some((version, text)) = ceiling.get(os_str(hardware_type)) else {
                    continue;
                };
                (version.clone(), *text, HardwareStatus::Unreleased)
            }
            None => continue,
        };
        if version[0] < MIN_MAJOR_VERSION {
            continue;
        }
        groups
            .entry((hardware_type, status, Reverse(version)))
            .or_insert_with(|| (text, Vec::new()))
            .1
            .push(Entry {
                identifier,
                family,
                minor,
            });
    }

    let devices = groups
        .into_iter()
        .map(|((hardware_type, status, _), (version, mut entries))| {
            entries.sort_by(|a, b| b.family.cmp(&a.family).then(a.minor.cmp(&b.minor)));
            Device {
                r#type: hardware_type,
                version: version.to_string(),
                status,
                hardware: entries.iter().map(|e| e.identifier.to_string()).collect(),
            }
        })
        .collect();

    Source {
        metadata: Metadata {
            last_updated: today,
        },
        devices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    const FIRMWARES: &str = r#"[
      {"osStr":"iOS","version":"27.0","released":"2026-09-14","beta":false,"rc":false,
       "deviceMap":["iPhone19,2","iPhone12,1","iPod9,1","iPhone99,11","iPhone98,1","iPhone96,1"]},
      {"osStr":"iOS","version":"27.0","released":"","beta":false,"rc":false,"internal":true,
       "deviceMap":["iPhone19,4","iPhone20,1"]},
      {"osStr":"iOS","version":"27.2 beta","released":"2026-09-16","beta":true,"rc":false,
       "deviceMap":["iPhone12,1"]},
      {"osStr":"iOS","version":"27.0 SDK","released":"2026-09-14","beta":false,"rc":false,
       "deviceMap":["iOS SDK 27.0"]},
      {"osStr":"iOS","version":"28.0","released":"2027-09-14","beta":false,"rc":false,
       "deviceMap":["iPhone12,1"]},
      {"osStr":"iOS","version":"27.1","released":"","beta":false,"rc":false,
       "deviceMap":["iPhone12,1"]},
      {"osStr":"iOS","version":"18.7.10","released":"2026-08-17","beta":false,"rc":false,
       "deviceMap":["iPhone11,2"]},
      {"osStr":"iPadOS","version":"27.0","released":"2026-09-14","beta":false,"rc":false,
       "deviceMap":["iPad8,9"]},
      {"osStr":"iPadOS","version":"26.7","released":"2026-09-14","beta":false,"rc":false,
       "deviceMap":["iPad8,1","iPad8,9"]},
      {"osStr":"iPadOS","version":"12.5.8","released":"2023-01-23","beta":false,"rc":false,
       "deviceMap":["iPad4,1"]},
      {"osStr":"tvOS","version":"27.0","released":"2026-09-14","beta":false,"rc":false,
       "deviceMap":["AppleTV14,1"]}
    ]"#;

    const CATALOG: &str = r#"[
      {"type":"iPhone","identifier":["iPhone19,2"],"released":"2026-09-18"},
      {"type":"iPhone","identifier":["iPhone19,4"],"released":"2026-10-23"},
      {"type":"iPhone","identifier":["iPhone12,1"],"released":"2019-09-20"},
      {"type":"iPhone","identifier":["iPhone11,2"],"released":["2018-10-26","2019-09-10"]},
      {"type":"iPhone","identifier":["iPhone99,11"]},
      {"type":"iPhone","identifier":["iPhone98,1"],"released":"not-a-date"},
      {"type":"iPhone","identifier":["iPhone96,1"],"released":null},
      {"type":"iPad Pro","identifier":["iPad8,9"],"released":"2020-03-25"},
      {"type":"iPad Pro","identifier":["iPad8,1"],"released":"2018-11-07"},
      {"type":"iPad","identifier":["iPad4,1"],"released":"2012-11-02"},
      {"type":"iPod touch","identifier":["iPod9,1"],"released":"2019-05-28"}
    ]"#;

    fn built() -> Source {
        let firmwares: Vec<Firmware> = serde_json::from_str(FIRMWARES).unwrap();
        let catalog: Vec<CatalogDevice> = serde_json::from_str(CATALOG).unwrap();
        build(&firmwares, &catalog, today())
    }

    fn rows(source: &Source) -> Vec<(&str, &str, &str, Vec<&str>)> {
        source
            .devices
            .iter()
            .map(|d| {
                (
                    d.r#type.as_str(),
                    d.version.as_str(),
                    d.status.as_str(),
                    d.hardware.iter().map(String::as_str).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn derives_support_status_and_layout() {
        assert_eq!(
            rows(&built()),
            vec![
                ("iPadPro", "27.0", "Supported", vec!["iPad8,9"]),
                ("iPadPro", "26.7", "Unsupported", vec!["iPad8,1"]),
                ("iPhone", "27.0", "Unreleased", vec!["iPhone19,4"]),
                (
                    "iPhone",
                    "27.0",
                    "Supported",
                    vec!["iPhone19,2", "iPhone12,1"]
                ),
                ("iPhone", "18.7.10", "Unsupported", vec!["iPhone11,2"]),
            ]
        );
    }

    #[test]
    fn excludes_hardware_that_must_never_ship() {
        let source = built();
        let shipped: Vec<&str> = source
            .devices
            .iter()
            .flat_map(|d| d.hardware.iter().map(String::as_str))
            .collect();

        assert!(!shipped.contains(&"iPhone20,1"));
        assert!(!shipped.contains(&"iPhone99,11"));
        assert!(!shipped.contains(&"iPad4,1"));
        assert!(!shipped.contains(&"iPod9,1"));
    }

    #[test]
    fn ignores_unshipped_and_non_release_builds() {
        let source = built();
        let supported = source
            .devices
            .iter()
            .find(|d| d.r#type == HardwareType::IPhone && d.status == HardwareStatus::Supported)
            .unwrap();
        assert_eq!(supported.version, "27.0");
        assert!(supported.hardware.contains(&"iPhone12,1".to_string()));
    }

    #[test]
    fn includes_announced_hardware_before_it_ships() {
        let source = built();
        let upcoming = source
            .devices
            .iter()
            .find(|d| d.status == HardwareStatus::Unreleased)
            .expect("announced hardware should be carried before it ships");

        assert_eq!(upcoming.hardware, vec!["iPhone19,4".to_string()]);
        assert_eq!(upcoming.version, "27.0");
        assert_eq!(upcoming.r#type, HardwareType::IPhone);
    }

    #[test]
    fn treats_unusable_release_dates_as_never_shipped() {
        let source = built();
        let shipped: Vec<&str> = source
            .devices
            .iter()
            .flat_map(|d| d.hardware.iter().map(String::as_str))
            .collect();

        assert!(!shipped.contains(&"iPhone98,1"));
        assert!(!shipped.contains(&"iPhone96,1"));
        assert!(!source.devices.iter().any(|d| d.version == "27.1"));
    }

    #[test]
    fn records_the_date_the_data_was_pulled() {
        assert_eq!(built().metadata.last_updated, today());
    }
}
