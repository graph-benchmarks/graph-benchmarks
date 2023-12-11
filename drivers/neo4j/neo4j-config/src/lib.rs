use std::collections::HashMap;

use anyhow::Result;
use common::{driver_config::DriverConfig, traverse_yaml_mut};
use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{Pod, Service},
};
use kube::{
    api::ListParams,
    runtime::{watcher, WatchStreamExt},
    Api, Client,
};
use serde_yaml::Mapping;
use tokio::fs;
use tracing::info;

pub struct Neo4j;

#[async_trait::async_trait]
impl DriverConfig for Neo4j {
    fn name(&self) -> String {
        "neo4j".to_owned()
    }

    async fn get_platform_config(&self, mut nodes: usize) -> Result<HashMap<String, String>> {
        if nodes == 2 {
            nodes = 3;
        }

        Ok(HashMap::from([
            ("host".into(), "neo-lb-neo4j".into()),
            ("port".into(), 7687.to_string()),
            ("user".into(), "neo4j".into()),
            ("password".into(), "graph_benchmarks".into()),
            ("neo_instances".into(), nodes.to_string()),
        ]))
    }

    async fn scale_service(
        &self,
        mut nodes: usize,
        options: Option<serde_yaml::Value>,
    ) -> Result<Vec<String>> {
        let f = fs::read_to_string(&format!("drivers/{}/values.yaml", self.name())).await?;
        let mut values: serde_yaml::Value = serde_yaml::from_str(&f)?;

        if nodes == 2 {
            nodes = 3;
        }

        let client = Client::try_default().await?;
        let service: Api<Service> = Api::default_namespaced(client);
        let nfs_ip = service
            .get("nfs-service")
            .await?
            .spec
            .unwrap()
            .cluster_ip
            .unwrap();

        let options = match options {
            Some(s) => s.as_mapping().unwrap().to_owned(),
            None => Mapping::new(),
        };

        if options.contains_key("cpu") {
            *traverse_yaml_mut(&mut values, "neo4j.resources.cpu")?.unwrap() =
                options.get("cpu").unwrap().clone();
        }

        if options.contains_key("memory") {
            *traverse_yaml_mut(&mut values, "neo4j.resources.memory")?.unwrap() =
                options.get("memory").unwrap().clone();
        }

        if options.contains_key("storage") {
            *traverse_yaml_mut(&mut values, "volumes.data.dynamic.requests.storage")?.unwrap() =
                options.get("storage").unwrap().clone();
        }

        *traverse_yaml_mut(&mut values, "volumes.import.volumeClaimTemplate.nfs.server")?
            .unwrap() = nfs_ip.clone().into();
        *traverse_yaml_mut(&mut values, "neo4j.minimumClusterSize")?.unwrap() = nodes.into();

        let mut values_other = values.clone();
        traverse_yaml_mut(&mut values_other, "volumes")?
            .unwrap()
            .as_mapping_mut()
            .unwrap()
            .remove("import");

        let values_other_str = serde_yaml::to_string(&values_other)?.replace(
            "acceptLicenseAgreement: yes",
            "acceptLicenseAgreement: 'yes'",
        );
        fs::write(
            format!("drivers/{}/tmp/values-1.yaml", self.name()),
            serde_yaml::to_string(&values)?.replace(
                "acceptLicenseAgreement: yes",
                "acceptLicenseAgreement: 'yes'",
            ),
        )
        .await?;
        for i in 2..=nodes {
            fs::write(
                format!("drivers/{}/tmp/values-{}.yaml", self.name(), i),
                values_other_str.clone(),
            )
            .await?;
        }

        let mut import_pv: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(format!("drivers/{}/import-pv.yaml", self.name())).await?,
        )?;
        *traverse_yaml_mut(&mut import_pv, "spec.nfs.server")?.unwrap() = nfs_ip.into();
        fs::write(
            format!("drivers/{}/tmp/import-pv.yaml", self.name()),
            serde_yaml::to_string(&import_pv)?,
        )
        .await?;

        Ok(vec![format!("num_nodes={nodes}")])
    }

    async fn metrics_pod_ids(&self) -> Result<Vec<String>> {
        let client = Client::try_default().await?;
        let pods: Api<Pod> = Api::default_namespaced(client);
        let pods = pods
            .list(&ListParams::default().labels("helm.neo4j.com/pod_category=neo4j-instance"))
            .await?;
        Ok(pods
            .items
            .into_iter()
            .map(|x| x.metadata.name.unwrap())
            .collect())
    }

    async fn wait_for_service_ready(&self, mut nodes: usize) -> Result<()> {
        if nodes == 2 {
            nodes = 3;
        }

        let client = Client::try_default().await?;
        let ss: Api<StatefulSet> = Api::default_namespaced(client);
        let wc = watcher::Config::default().labels("helm.neo4j.com/neo4j.name=neo");

        let status_check = |ss: StatefulSet| {
            if let Some(status) = ss.status {
                return status.available_replicas.unwrap() == status.replicas;
            }
            false
        };

        let mut successful = 0;
        let mut res = watcher(ss, wc).applied_objects().default_backoff().boxed();
        while let Ok(Some(s)) = res.try_next().await {
            info!("got status update {}", s.metadata.name.as_ref().unwrap());
            if status_check(s) {
                successful += 1;
                if successful == nodes {
                    break;
                }
            }
        }
        Ok(())
    }
}
