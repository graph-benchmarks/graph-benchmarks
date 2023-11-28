use anyhow::Result;

#[async_trait::async_trait]
pub trait DriverConfig {
    fn name(self: &Self) -> String;
    async fn get_service_ip(self: &Self) -> Result<String>;
    fn pod_ready_label(&self) -> &'static str;
}
