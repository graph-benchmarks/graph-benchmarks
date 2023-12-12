use std::{collections::HashMap, net::IpAddr};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::exit;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub setup: SetupArgs,
    pub kubernetes: Option<KubeSetup>,
    pub benchmark: Benchmark,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformConnectInfo {
    pub private_key_file: String,
    pub master_ip: IpAddr,
    pub worker_ips: Vec<IpAddr>,
    pub host_username: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetupArgs {
    pub host_username: Option<String>,
    pub platform: String,
    pub provider: String,
    pub node_configs: Vec<usize>,
    pub master_platform: Option<HashMap<String, String>>,
    pub worker_platform: Option<HashMap<String, String>>,
    pub platform_args: Option<HashMap<String, String>>,
    pub graph_platform_args: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KubeSetup {
    pub dashboard: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Benchmark {
    pub drivers: Vec<String>,
    pub datasets: Vec<String>,
    pub repeat: usize,
    pub algorithms: Option<Vec<String>>,
    pub debug: Option<DebugOptions>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugOptions {
    pub bench_ttl: Option<i32>,
    pub skip_join_nodes: Option<bool>,
}

impl Default for DebugOptions {
    fn default() -> Self {
        Self {
            bench_ttl: Some(0),
            skip_join_nodes: Some(false),
        }
    }
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
