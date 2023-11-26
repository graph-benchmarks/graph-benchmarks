use macros::include_driver_config;
use anyhow::Result;

#[async_trait::async_trait]
pub trait DriverConfig {
    fn name(self: &Self) -> String;
    async fn get_service_ip(self: &Self) -> Result<String>;
    fn pod_ready_label(&self) -> &'static str;
}

pub fn get_driver_config(name: &str) -> Option<&'static dyn DriverConfig> {
    DRIVER_CONFIGS.iter().find(|x| x.name() == name).copied()
}

include_driver_config!();