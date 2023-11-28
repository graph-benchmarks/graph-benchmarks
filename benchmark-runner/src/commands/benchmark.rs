use std::{
    collections::{BTreeMap, HashMap},
    env,
    time::Instant,
};

use anyhow::{bail, Result};
use common::{
    command::{command, finish_progress, progress},
    config::parse_config,
    exit,
    provider::PlatformInfo,
};
use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::{
    api::core::v1::{
        ConfigMap, ConfigMapVolumeSource, Container, HostPathVolumeSource, PersistentVolume,
        PersistentVolumeClaim, PersistentVolumeClaimSpec, PersistentVolumeClaimVolumeSource,
        PersistentVolumeSpec, Pod, PodSpec, ResourceRequirements, Volume, VolumeMount,
    },
    apimachinery::pkg::api::resource::Quantity,
};
use kube::{
    api::{DeleteParams, ListParams, PostParams},
    runtime::{watcher, WatchStreamExt},
    Api, Client, ResourceExt,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::args::Cli;

const ALGORITHMS: &[&str] = &["bfs", "pr", "wcc", "cdlp", "lcc", "sssp"];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DriverConfig {
    postgres: PostgresConfig,
    ip: String,
    dataset: String,
    output_path: String,
    algo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostgresConfig {
    ip: String,
    db: String,
    user: String,
    password: String,
}

pub async fn run_benchmark(cli: &Cli) -> Result<()> {
    let mut config = parse_config(&cli.file)?;

    let connect_args = 'p: {
        for p in base_provider::PROVIDERS {
            if p.name() == config.setup.provider {
                break 'p p.platform_info(&config.setup, cli.verbose).await?;
            }
        }
        exit!("", "Unknown platform {}", config.setup.platform)
    };

    setup_pv_and_pvc().await?;

    config.setup.node_configs.sort_by(|a, b| b.cmp(a));
    for n_nodes in config.setup.node_configs {
        for driver in &config.benchmark.drivers {
            let driver_config = match base_driver::get_driver_config(driver) {
                Some(d) => d,
                None => exit!("", "Could not find driver {}", driver),
            };

            setup_driver(&driver, &connect_args, cli.verbose).await?;
            let service_ip = driver_config.get_service_ip().await?;
            let mut cfg = DriverConfig {
                postgres: PostgresConfig {
                    ip: "postgres".to_owned(),
                    db: "postgres".to_owned(),
                    user: "postgres".to_owned(),
                    password: "graph_benchmarks".to_owned(),
                },
                ip: service_ip,
                dataset: "".into(),
                output_path: "/attached".into(),
                algo: "".into(),
            };

            for dataset in &config.benchmark.datasets {
                for algorithm in config.benchmark.algorithms.as_ref().unwrap_or(
                    &ALGORITHMS
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>(),
                ) {
                    cfg.dataset = dataset.clone();
                    cfg.algo = algorithm.clone();
                    info!("{cfg:#?}");

                    let bench_pod = start_bench(&cfg).await?;
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

async fn setup_driver(name: &str, connect_args: &PlatformInfo, verbose: bool) -> Result<()> {
    let mut env = HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]);
    if verbose {
        env.insert("DEBUG_ANSIBLE", "1");
    }

    command(
        "ansible-playbook",
        &[
            "setup.yaml",
            "--private-key",
            &connect_args.ssh_key,
            "-i",
            "../../k3s/inventory/master-hosts.yaml",
        ],
        verbose,
        [
            &format!("Installing {name}"),
            &format!("Could not install {name}"),
            &format!("Installed {name}"),
        ],
        &format!("drivers/{}", name),
        env,
    )
    .await?;

    let start = Instant::now();
    let pb = progress(&format!("Waiting for {name} to be ready"));
    let client = Client::try_default().await?;

    let mut ready = false;
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

    let pod_label = base_driver::get_driver_config(name)
        .unwrap()
        .pod_ready_label();

    let pods: Api<Pod> = Api::default_namespaced(client.clone());
    if let Ok(pod_list) = pods.list(&ListParams::default().labels(pod_label)).await {
        for pod in pod_list.items {
            ready = status_check(pod);
            if ready {
                break;
            }
        }
    }

    if !ready {
        let api = Api::<Pod>::default_namespaced(client);
        let wc = watcher::Config::default().labels(pod_label);

        let mut res = watcher(api, wc).applied_objects().default_backoff().boxed();

        while let Ok(Some(p)) = res.try_next().await {
            info!("got status update {}", p.name_any());
            if status_check(p) {
                break;
            }
        }
    }

    finish_progress(
        "Driver ready",
        &format!("drivers/{name}"),
        start.elapsed(),
        Some(pb),
    );
    Ok(())
}

async fn start_bench(cfg: &DriverConfig) -> Result<Pod> {
    let client = Client::try_default().await?;

    let default_pp = PostParams {
        dry_run: false,
        field_manager: None,
    };

    let config_map: Api<ConfigMap> = Api::default_namespaced(client.clone());
    _ = config_map
        .delete("benchmark-cfg", &DeleteParams::default())
        .await;

    let mut config_map_spec = ConfigMap::default();
    config_map_spec.metadata.name = Some("benchmark-cfg".into());
    config_map_spec.data = Some(BTreeMap::from([(
        "config.yaml".into(),
        serde_yaml::to_string(&cfg)?,
    )]));
    config_map_spec.immutable = Some(true);
    config_map.create(&default_pp, &config_map_spec).await?;

    let pods: Api<Pod> = Api::default_namespaced(client);
    let mut dp = DeleteParams::default();
    dp.grace_period_seconds = Some(0);
    _ = pods.delete("graphscope-bench", &dp).await;

    let mut pod_spec = Pod::default();
    pod_spec.metadata.name = Some("graphscope-bench".into());
    pod_spec.spec = Some(PodSpec {
        containers: vec![Container {
            args: Some(vec!["/cfg/config.yaml".into()]),
            image: Some("registry.pub.348575.xyz:5000/graphscope/bench".into()),
            // image_pull_policy: Some("Never".into()),
            name: "graphscope-bench".into(),
            volume_mounts: Some(vec![
                VolumeMount {
                    mount_path: "/attached".into(),
                    name: "benchmark-pvc".into(),
                    ..VolumeMount::default()
                },
                VolumeMount {
                    name: "benchmark-cfg".into(),
                    mount_path: "/cfg".into(),
                    read_only: Some(true),
                    ..VolumeMount::default()
                },
            ]),
            ..Container::default()
        }],
        volumes: Some(vec![
            Volume {
                name: "benchmark-pvc".into(),
                persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                    claim_name: "benchmark-pvc".into(),
                    read_only: Some(false),
                }),
                ..Volume::default()
            },
            Volume {
                name: "benchmark-cfg".into(),
                config_map: Some(ConfigMapVolumeSource {
                    name: Some("benchmark-cfg".into()),
                    ..ConfigMapVolumeSource::default()
                }),
                ..Volume::default()
            },
        ]),
        ..PodSpec::default()
    });

    Ok(pods.create(&default_pp, &pod_spec).await?)
}

async fn setup_pv_and_pvc() -> Result<()> {
    env::set_var("KUBECONFIG", "k3s/kube-config");

    let default_pp = PostParams {
        dry_run: false,
        field_manager: None,
    };

    let client = Client::try_default().await?;
    let pv: Api<PersistentVolume> = Api::all(client.clone());
    let mut pv_spec = PersistentVolume::default();
    pv_spec.metadata.name = Some("benchmark-pv".into());
    pv_spec.metadata.labels = Some(BTreeMap::from([("type".into(), "local".into())]));
    pv_spec.spec = Some(PersistentVolumeSpec {
        access_modes: Some(vec!["ReadWriteOnce".into()]),
        capacity: Some(BTreeMap::from([(
            "storage".into(),
            Quantity("10Gi".into()),
        )])),
        host_path: Some(HostPathVolumeSource {
            path: "/benchmark-pv".into(),
            type_: None,
        }),
        storage_class_name: Some("manual".into()),
        ..PersistentVolumeSpec::default()
    });

    if let Err(err) = pv.create(&default_pp, &pv_spec).await {
        match err {
            kube::Error::Api(ref api_error) => {
                if api_error.code != 409 {
                    bail!(err);
                } else {
                    info!("PV already exists, not creating");
                }
            }
            _ => bail!(err),
        }
    }

    let pvc: Api<PersistentVolumeClaim> = Api::default_namespaced(client);
    let mut pvc_spec = PersistentVolumeClaim::default();
    pvc_spec.metadata.name = Some("benchmark-pvc".into());
    pvc_spec.spec = Some(PersistentVolumeClaimSpec {
        access_modes: Some(vec!["ReadWriteOnce".into()]),
        resources: Some(ResourceRequirements {
            requests: Some(BTreeMap::from([(
                "storage".into(),
                Quantity("10Gi".into()),
            )])),
            ..Default::default()
        }),
        storage_class_name: Some("manual".into()),
        ..PersistentVolumeClaimSpec::default()
    });

    if let Err(err) = pvc.create(&default_pp, &pvc_spec).await {
        match err {
            kube::Error::Api(ref api_error) => {
                if api_error.code != 409 {
                    bail!(err);
                } else {
                    info!("PVC already exists, not creating");
                }
            }
            _ => bail!(err),
        }
    }
    Ok(())
}
