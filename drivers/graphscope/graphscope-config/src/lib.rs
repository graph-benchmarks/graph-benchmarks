use common::driver_config::DriverConfig;
use kube::{Client, Api};
use anyhow::Result;

pub struct Graphscope;

#[async_trait::async_trait]
impl DriverConfig for Graphscope {
    fn name(&self) -> String {
        "graphscope".to_owned()
    }

    async fn get_service_ip(&self) -> Result<std::string::String> {
        let client = Client::try_default().await?;
        let services: Api<k8s_openapi::api::core::v1::Service> = Api::default_namespaced(client);
        let coordinator = services.get("coordinator-service-graphscope").await?;
        Ok(format!("coordinator-service-graphscope:{}", coordinator.spec.unwrap().ports.unwrap()[0].port))
    }

    fn pod_ready_label(&self) -> &'static str {
        "graphscope.components=coordinator"
    }
}