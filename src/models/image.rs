use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum::{Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub name: String,
    pub path: PathBuf,
    pub architecture: Architecture
}

#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumString)]
pub enum Architecture {
    #[strum(to_string = "x86")]
    X86,
    #[strum(to_string = "x86_64")]
    X86_64,
    #[strum(to_string = "arm64")]
    ARM64
}