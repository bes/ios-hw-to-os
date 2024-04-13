use crate::output::{Device, Output};
use crate::source::Source;
use std::fs;

mod output;
mod source;

fn main() {
    let yaml = fs::read_to_string("source.yaml").unwrap();
    let source: Source = serde_yaml::from_str(&yaml).unwrap();

    let mut output = Output {
        devices: Vec::new(),
    };

    for device in source.devices.iter() {
        for hardware in device.hardware.iter() {
            output.devices.push(Device {
                version: &device.version,
                hardware,
            })
        }
    }

    let output = serde_json::to_string_pretty(&output).unwrap();
    println!("{}", output);
}
