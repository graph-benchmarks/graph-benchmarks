use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub dataset: String,
    pub algorithm: String,
    pub nodes: usize,
    pub run_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetUserConfig {
    pub weights: bool,
    pub directed: bool,
    pub start_vertex: usize,
    pub skip_algos: Option<Vec<String>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    pub vertex: String,
    pub edges: String,
    pub name: String,
    pub directed: bool,
    pub weights: bool,
    pub start_vertex: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub ids: String,
    pub algos: String,
    pub log_file: String,
    pub nodes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverConfig<'a> {
    pub config: RunConfig,
    #[serde(borrow)]
    pub postgres: PostgresConfig<'a>,
    pub platform: HashMap<String, String>,
    pub dataset: DatasetConfig,
    pub load_data: bool,
    pub drop_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig<'a> {
    pub host: &'a str,
    pub port: u32,
    pub db: &'a str,
    pub user: &'a str,
    pub ps: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchStartEvent {
    pub status: bool,
}
