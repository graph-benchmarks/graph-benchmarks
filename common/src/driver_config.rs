use anyhow::Result;

#[async_trait::async_trait]
pub trait DriverConfig {
    fn name(self: &Self) -> String;
    async fn get_service_ip(&self) -> Result<String>;
    async fn set_node_config(&self, nodes: usize, cpu: usize, memory: usize) -> Result<()>;
    async fn metrics_node_ids(&self) -> Result<Vec<String>>;
    fn pod_ready_label(&self) -> &'static str;
}
