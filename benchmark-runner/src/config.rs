use std::{collections::HashMap, net::IpAddr};

use serde::{Deserialize, Serialize};

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
    pub platform_env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Benchmark {
    pub drivers: Vec<String>,
    pub datasets: Option<Vec<String>>,
    pub algorithms: Option<Vec<String>>,
}
