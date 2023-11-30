use std::collections::HashMap;

use anyhow::Result;
use common::driver_config::DriverConfig;
use k8s_openapi::api::core::v1::{Pod, Service};
use kube::{api::ListParams, Api, Client};
use tokio::fs;

pub struct Graphscope;

#[async_trait::async_trait]
impl DriverConfig for Graphscope {
    fn name(&self) -> String {
        "graphscope".to_owned()
    }

    async fn get_service_ip(&self) -> Result<(String, u16)> {
        let client = Client::try_default().await?;
        let services: Api<Service> = Api::default_namespaced(client);
        let coordinator = services.get("coordinator-service-graphscope").await?;
        Ok((
            "coordinator-service-graphscope:{}".into(),
            coordinator.spec.unwrap().ports.unwrap()[0].port as u16,
        ))
    }

    async fn set_node_config(&self, nodes: usize, cpu: usize, memory: usize) -> Result<()> {
        let values_file = format!("drivers/{}/values.yaml", self.name());
        let f = fs::read_to_string(&values_file).await?;
        let mut data: HashMap<&str, serde_yaml::Value> = serde_yaml::from_str(&f)?;

        let mut e = serde_yaml::Mapping::new();
        e.insert("gae.resources.requests.cpu".into(), cpu.into());
        e.insert(
            "gae.resources.requests.memory".into(),
            format!("{}Gi", memory).into(),
        );
        e.insert("gie.resources.requests.cpu".into(), cpu.into());
        e.insert(
            "gie.resources.requests.memory".into(),
            format!("{}Gi", memory).into(),
        );
        e.insert("num_workers".into(), nodes.into());
        data.insert("engines", serde_yaml::Value::Mapping(e));

        fs::write(values_file, serde_yaml::to_string(&data)?).await?;
        Ok(())
    }

    async fn metrics_pod_ids(&self) -> Result<Vec<String>> {
        let client = Client::try_default().await?;
        let pods: Api<Pod> = Api::default_namespaced(client);
        let pods = pods
            .list(&ListParams::default().labels("app.kubernetes.io/name=graphscope"))
            .await?;
        Ok(pods
            .items
            .into_iter()
            .map(|x| x.metadata.uid.unwrap())
            .collect())
    }

    fn pod_ready_label(&self) -> &'static str {
        "graphscope.components=coordinator"
    }
}
