use anyhow::Result;

#[async_trait::async_trait]
pub trait DriverConfig {
    /// The name of the driver
    fn name(self: &Self) -> String;
    /// (Host, port) of the platform's pod to pass onto benchmark pod
    async fn get_service_ip(&self) -> Result<(String, u16)>;
    /// Scale pods & other resources
    async fn set_node_config(&self, nodes: usize, options: Option<serde_yaml::Value>)
        -> Result<()>;
    /// Pod ids to record metrics for
    async fn metrics_pod_ids(&self) -> Result<Vec<String>>;
    /// kubernetes label of the pod to check running status
    fn pod_ready_label(&self) -> &'static str;
}
