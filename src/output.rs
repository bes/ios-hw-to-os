use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Output<'a> {
    #[serde(borrow)]
    pub devices: Vec<Device<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device<'a> {
    pub hardware: &'a str,
    pub version: &'a str,
}
