use std::collections::HashMap;

use anyhow::Result;
use common::driver_config::DriverConfig;
use common::traverse_yaml_mut;
use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{Pod, Service};
use kube::{
    api::ListParams,
    runtime::{watcher, WatchStreamExt},
    Api, Client,
};
use serde_yaml::Mapping;
use tokio::fs;
use tracing::info;

pub struct Graphscope;

#[async_trait::async_trait]
impl DriverConfig for Graphscope {
    fn name(&self) -> String {
        "graphscope".to_owned()
    }

    async fn get_platform_config(&self, _: usize) -> Result<HashMap<String, String>> {
        let client = Client::try_default().await?;
        let services: Api<Service> = Api::default_namespaced(client);
        let coordinator = services.get("coordinator-service-graphscope").await?;
        Ok(HashMap::from([
            ("host".into(), "coordinator-service-graphscope".into()),
            (
                "port".into(),
                coordinator.spec.unwrap().ports.unwrap()[0].port.to_string(),
            ),
        ]))
    }

    async fn scale_service(
        &self,
        nodes: usize,
        options: Option<serde_yaml::Value>,
    ) -> Result<Vec<String>> {
        let values_file = format!("drivers/{}/values.yaml", self.name());
        let f = fs::read_to_string(&values_file).await?;
        let mut data: serde_yaml::Value = serde_yaml::from_str(&f)?;

        let options = match options {
            Some(s) => s.as_mapping().unwrap().to_owned(),
            None => Mapping::new(),
        };

        let engines = data.get_mut("engines").unwrap();
        *engines.get_mut("num_workers").unwrap() = nodes.into();

        if options.contains_key("cpu") {
            let cpu = options.get("cpu").unwrap().clone();
            *traverse_yaml_mut(&mut data, "engines.gae.resources.requests.cpu")?.unwrap() =
                cpu.clone().into();
            *traverse_yaml_mut(&mut data, "engines.gae.resources.limits.cpu")?.unwrap() =
                cpu.clone().into();
            *traverse_yaml_mut(&mut data, "engines.gie.resources.requests.cpu")?.unwrap() =
                cpu.clone().into();
            *traverse_yaml_mut(&mut data, "engines.gie.resources.limits.cpu")?.unwrap() =
                cpu.clone().into();
        }

        if options.contains_key("memory") {
            let memory = options.get("memory").unwrap().clone();
            *traverse_yaml_mut(&mut data, "engines.gae.resources.requests.memory")?.unwrap() =
                memory.clone().into();
            *traverse_yaml_mut(&mut data, "engines.gae.resources.limits.memory")?.unwrap() =
                memory.clone().into();
            *traverse_yaml_mut(&mut data, "engines.gie.resources.requests.memory")?.unwrap() =
                memory.clone().into();
            *traverse_yaml_mut(&mut data, "engines.gie.resources.limits.memory")?.unwrap() =
                memory.clone().into();
        }

        fs::write(values_file, serde_yaml::to_string(&data)?).await?;
        Ok(vec![])
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

    async fn wait_for_service_ready(&self, _: usize) -> Result<()> {
        let client = Client::try_default().await?;
        let ss: Api<Pod> = Api::default_namespaced(client);
        let wc = watcher::Config::default().labels("graphscope.components=coordinator");

        let status_check = |pod: Pod| {
            if let Some(status) = pod.status {
                if let Some(cs) = status.container_statuses {
                    if cs.len() > 0 && cs[0].ready {
                        return true;
                    }
                }
            }
            false
        };

        let mut res = watcher(ss, wc).applied_objects().default_backoff().boxed();
        while let Ok(Some(s)) = res.try_next().await {
            info!("got status update {}", s.metadata.name.as_ref().unwrap());
            if status_check(s) {
                break;
            }
        }
        Ok(())
    }
}
