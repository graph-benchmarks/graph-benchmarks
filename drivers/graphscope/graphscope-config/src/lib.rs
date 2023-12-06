use std::collections::HashMap;

use anyhow::Result;
use common::driver_config::DriverConfig;
use k8s_openapi::api::core::v1::{Pod, Service};
use kube::{api::ListParams, Api, Client};
use serde_yaml::Mapping;
use tokio::fs;

pub struct Graphscope;

fn set_third_key<'a>(
    from: &'a mut serde_yaml::Value,
    second_key: &str,
    third_key: &str,
) -> &'a mut serde_yaml::Value {
    from.get_mut("resources")
        .unwrap()
        .get_mut(second_key)
        .unwrap()
        .get_mut(third_key)
        .unwrap()
}

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
            "coordinator-service-graphscope".into(),
            coordinator.spec.unwrap().ports.unwrap()[0].port as u16,
        ))
    }

    async fn set_node_config(
        &self,
        nodes: usize,
        options: Option<serde_yaml::Value>,
    ) -> Result<()> {
        let values_file = format!("drivers/{}/values.yaml", self.name());
        let f = fs::read_to_string(&values_file).await?;
        let mut data: HashMap<&str, serde_yaml::Value> = serde_yaml::from_str(&f)?;

        let options = match options {
            Some(s) => s.as_mapping().unwrap().to_owned(),
            None => Mapping::new(),
        };

        let engines = data.get_mut("engines").unwrap();
        *engines.get_mut("num_workers").unwrap() = nodes.into();

        if options.contains_key("cpu") {
            *set_third_key(engines.get_mut("gae").unwrap(), "requests", "cpu") =
                options.get("cpu").unwrap().clone();
            *set_third_key(engines.get_mut("gae").unwrap(), "limits", "cpu") =
                options.get("cpu").unwrap().clone();
            *set_third_key(engines.get_mut("gie").unwrap(), "requests", "cpu") =
                options.get("cpu").unwrap().clone();
            *set_third_key(engines.get_mut("gie").unwrap(), "limits", "cpu") =
                options.get("cpu").unwrap().clone();
        }

        if options.contains_key("memory") {
            *set_third_key(engines.get_mut("gae").unwrap(), "requests", "memory") =
                options.get("memory").unwrap().clone();
            *set_third_key(engines.get_mut("gae").unwrap(), "limits", "memory") =
                options.get("memory").unwrap().clone();
            *set_third_key(engines.get_mut("gie").unwrap(), "requests", "memory") =
                options.get("memory").unwrap().clone();
            *set_third_key(engines.get_mut("gie").unwrap(), "limits", "memory") =
                options.get("memory").unwrap().clone();
        }

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
            .map(|x| x.metadata.name.unwrap())
            .collect())
    }

    fn pod_ready_label(&self) -> &'static str {
        "graphscope.components=coordinator"
    }
}
