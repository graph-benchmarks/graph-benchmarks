use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub dataset: String,
    pub algorithm: String,
    pub nodes: usize,
    pub run_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    pub vertex: String,
    pub edges: String,
    pub name: String,
    pub directed: bool,
    pub weights: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub id: i32,
    pub algo: String,
    pub log_file: String,
    pub nodes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverConfig<'a> {
    pub config: RunConfig,
    #[serde(borrow)]
    pub postgres: PostgresConfig<'a>,
    pub platform: PlatformConfig,
    pub dataset: DatasetConfig,
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