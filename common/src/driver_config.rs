use std::collections::HashMap;

use anyhow::Result;

#[async_trait::async_trait]
pub trait DriverConfig {
    /// The name of the driver
    fn name(self: &Self) -> String;

    /// Get host, port, or any other required platform information
    async fn get_platform_config(&self, nodes: usize) -> Result<HashMap<String, String>>;

    /// Setup config to scale pods & other resources
    /// Returns a vector of arguments to pass with --extra-vars to ansible, eg. vec!["a=3", "b=4"]
    async fn scale_service(
        &self,
        nodes: usize,
        options: Option<serde_yaml::Value>,
    ) -> Result<Vec<String>>;

    /// Pod ids to record metrics for
    async fn metrics_pod_ids(&self) -> Result<Vec<String>>;

    /// kubernetes label of the pod to check running status
    async fn wait_for_service_ready(&self, nodes: usize) -> Result<()>;
}
