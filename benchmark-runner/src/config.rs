use std::{collections::HashMap, net::IpAddr};

use log::info;
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::exit;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub setup: Setup,
    pub benchmark: Benchmark,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Setup {
    PreConfiguredPlatform(PlatformConnectInfo),
    Platform(PlatformArgs),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformConnectInfo {
    pub private_key_file: String,
    pub ips: Vec<IpAddr>,
    pub host_username: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformArgs {
    pub host_username: Option<String>,
    pub platform: String,
    pub node_configs: Vec<usize>,
    pub master_platform_env: Option<HashMap<String, String>>,
    pub worker_platform_env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Benchmark {
    pub drivers: Vec<String>,
    pub datasets: Option<Vec<String>>,
    pub algorithms: Option<Vec<String>>,
}

pub fn parse_config(file: &str) -> Result<Config> {
    let config = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(err) => exit!(err, "Could not read config file {}", file),
    };

    let config: Result<Config, toml::de::Error> = toml::from_str(config.as_str());
    let config = match config {
        Ok(c) => c,
        Err(err) => exit!(err, "Could not parse config file {}", file),
    };

    info!("config file parsed");
    Ok(config)
}