use anyhow::Result;
use common::driver_config::DriverConfig;
use k8s_openapi::api::core::v1::Service;
use kube::{Api, Client};

pub struct Graphscope;

#[async_trait::async_trait]
impl DriverConfig for Graphscope {
    fn name(&self) -> String {
        "graphscope".to_owned()
    }

    async fn get_service_ip(&self) -> Result<std::string::String> {
        let client = Client::try_default().await?;
        let services: Api<Service> = Api::default_namespaced(client);
        let coordinator = services.get("coordinator-service-graphscope").await?;
        Ok(format!(
            "coordinator-service-graphscope:{}",
            coordinator.spec.unwrap().ports.unwrap()[0].port
        ))
    }

    async fn set_node_config(&self, nodes: usize, cpu: usize, memory: usize) -> Result<()> {
        Ok(())
    }

    async fn metrics_node_ids(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }

    fn pod_ready_label(&self) -> &'static str {
        "graphscope.components=coordinator"
    }
}
