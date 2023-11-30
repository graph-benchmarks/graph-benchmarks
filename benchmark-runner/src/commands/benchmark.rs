use std::{
    collections::{BTreeMap, HashMap},
    env,
    net::IpAddr,
    time::Instant,
};

use anyhow::{bail, Result};
use common::{
    command::{command, finish_progress, progress},
    config::parse_config,
    exit,
    provider::PlatformInfo,
};
use diesel::prelude::*;
use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper, AsyncConnection, AsyncPgConnection,
    RunQueryDsl,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::{
    api::core::v1::{
        ConfigMap, ConfigMapVolumeSource, Container, HostPathVolumeSource, Node, PersistentVolume,
        PersistentVolumeClaim, PersistentVolumeClaimSpec, PersistentVolumeClaimVolumeSource,
        PersistentVolumeSpec, Pod, PodSpec, ResourceRequirements, Volume, VolumeMount,
    },
    apimachinery::pkg::api::resource::Quantity,
};
use kube::{
    api::{DeleteParams, ListParams, PostParams},
    runtime::{
        conditions::is_pod_running,
        wait::{await_condition, Condition},
        watcher, WatchStreamExt,
    },
    Api, Client, ResourceExt,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    args::Cli,
    metrics_utils::{start_recording, stop_recording},
    model::Benchmark,
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
const ALGORITHMS: &[&str] = &["bfs", "pr", "wcc", "cdlp", "lcc", "sssp"];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Run {
    dataset: String,
    algorithm: String,
    run_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlatformConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatasetConfig {
    vertex: String,
    edges: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DriverConfig {
    postgres: PostgresConfig,
    platform: PlatformConfig,
    dataset: HashMap<String, DatasetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostgresConfig {
    host: String,
    port: u32,
    db: String,
    user: String,
    ps: String,
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

    setup_db(connect_args.master_ip.clone())?;
    let mut connection = AsyncPgConnection::establish(&format!(
        "postgres://postgres:graph_benchmarks@{}:30002/postgres",
        connect_args.master_ip
    ))
    .await?;

    env::set_var("KUBECONFIG", "k3s/kube-config");
    setup_pv_and_pvc().await?;

    let mut runs: Vec<Run> = Vec::new();

    config.setup.node_configs.sort_by(|a, b| b.cmp(a));
    for n_nodes in config.setup.node_configs {
        new_cluster_node_count(n_nodes).await?;
        for driver in &config.benchmark.drivers {
            let driver_config = match base_driver::get_driver_config(driver) {
                Some(d) => d,
                None => exit!("", "Could not find driver {}", driver),
            };

            driver_config.set_node_config(n_nodes, 2, 1024).await?;
            setup_graph_platform(&driver, &connect_args, cli.verbose).await?;
            let service_ip = driver_config.get_service_ip().await?;
            let pod_ids = driver_config.metrics_pod_ids().await?;

            let mut cfg = DriverConfig {
                postgres: PostgresConfig {
                    host: "postgres".into(),
                    db: "postgres".into(),
                    user: "postgres".into(),
                    port: 30002,
                    ps: "graph_benchmarks".into(),
                },
                platform: PlatformConfig {
                    host: service_ip.0,
                    port: service_ip.1,
                },
                dataset: HashMap::new(),
            };

            for dataset in &config.benchmark.datasets {
                for algorithm in config.benchmark.algorithms.as_ref().unwrap_or(
                    &ALGORITHMS
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>(),
                ) {
                    cfg.dataset = HashMap::from([(
                        dataset.clone(),
                        DatasetConfig {
                            vertex: "".into(),
                            edges: "".into(),
                        },
                    )]);
                    info!("{cfg:#?}");

                    let run_id = get_run_id(&mut connection, n_nodes).await?;
                    runs.push(Run {
                        run_id,
                        dataset: dataset.clone(),
                        algorithm: algorithm.clone(),
                    });

                    let bench_pod =
                        start_bench(&driver, &connect_args.master_ip, &cfg, run_id).await?;
                    let metrics_ip = format!("{}:30001", connect_args.master_ip);
                    start_recording(metrics_ip.clone(), pod_ids.clone(), run_id).await?;

                    let client = Client::try_default().await?;
                    let api: Api<Pod> = Api::default_namespaced(client);
                    await_condition(
                        api,
                        &bench_pod.metadata.name.unwrap(),
                        is_pod_running().not(),
                    )
                    .await?;

                    stop_recording(metrics_ip, pod_ids.clone(), run_id).await?;
                    return Ok(());
                }
            }

            remove_driver(&driver, &connect_args, cli.verbose).await?;
        }
    }

    Ok(())
}

async fn new_cluster_node_count(n_nodes: usize) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<Node> = Api::all(client);
    let mut nodes = api.list(&ListParams::default()).await?;
    if nodes.items.len() > n_nodes {
        nodes.items.sort_by(|a, b| {
            let a_name = a.metadata.name.as_ref().unwrap();
            let b_name = b.metadata.name.as_ref().unwrap();
            let get_idx = |n: &str| {
                if n.starts_with("worker-") {
                    n.split("worker-").last().unwrap().parse::<i32>().unwrap()
                } else {
                    -1
                }
            };

            get_idx(a_name).cmp(&get_idx(b_name))
        });

        let curr_nodes_len = nodes.items.len();
        let delete_nodes = nodes
            .items
            .drain(..)
            .rev()
            .take(curr_nodes_len - n_nodes)
            .map(|x| x.metadata.name.unwrap())
            .collect::<Vec<String>>();
        for node in delete_nodes {
            api.delete(&node, &DeleteParams::default()).await?;
        }
    }
    Ok(())
}

async fn remove_driver(driver: &str, connect_args: &PlatformInfo, verbose: bool) -> Result<()> {
    let mut env = HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]);
    if verbose {
        env.insert("DEBUG_ANSIBLE", "1");
    }

    command(
        "ansible-playbook",
        &[
            "remove.yaml",
            "--private-key",
            &connect_args.ssh_key,
            "-i",
            "../../k3s/inventory/master-hosts.yaml",
        ],
        verbose,
        [
            &format!("Removing driver {driver}"),
            &format!("Could not remove {driver}"),
            &format!("Removed {driver}"),
        ],
        &format!("drivers/{}", driver),
        env,
    )
    .await
}

async fn setup_graph_platform(
    name: &str,
    connect_args: &PlatformInfo,
    verbose: bool,
) -> Result<()> {
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
        "Platform ready",
        &format!("drivers/{name}"),
        start.elapsed(),
        Some(pb),
    );
    Ok(())
}

async fn start_bench(name: &str, host_ip: &IpAddr, cfg: &DriverConfig, run_id: i32) -> Result<Pod> {
    let client = Client::try_default().await?;

    let default_pp = PostParams {
        dry_run: false,
        field_manager: None,
    };

    let pod_name = format!("{}-bench", name);

    let config_map: Api<ConfigMap> = Api::default_namespaced(client.clone());
    _ = config_map.delete(&pod_name, &DeleteParams::default()).await;

    let mut config_map_spec = ConfigMap::default();
    config_map_spec.metadata.name = Some(pod_name.clone());
    config_map_spec.data = Some(BTreeMap::from([(
        "config.yaml".into(),
        serde_yaml::to_string(&cfg)?,
    )]));
    config_map_spec.immutable = Some(true);
    config_map.create(&default_pp, &config_map_spec).await?;

    let pods: Api<Pod> = Api::default_namespaced(client);
    let mut dp = DeleteParams::default();
    dp.grace_period_seconds = Some(0);
    _ = pods.delete(&pod_name, &dp).await;

    let mut pod_spec = Pod::default();
    pod_spec.metadata.name = Some(pod_name.clone());
    pod_spec.spec = Some(PodSpec {
        containers: vec![Container {
            args: Some(vec!["/cfg/config.yaml".into(), run_id.to_string()]),
            image: Some(format!("{}:30000/benches/{}:latest", host_ip, name)),
            name: format!("{}-bench", name).into(),
            volume_mounts: Some(vec![
                VolumeMount {
                    mount_path: "/attached".into(),
                    name: "benchmark-pvc".into(),
                    ..VolumeMount::default()
                },
                VolumeMount {
                    name: pod_name.clone(),
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
                name: pod_name.clone(),
                config_map: Some(ConfigMapVolumeSource {
                    name: Some(pod_name.clone()),
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

fn setup_db(master_ip: IpAddr) -> Result<()> {
    std::thread::spawn(move || {
        let mut connection = AsyncConnectionWrapper::<AsyncPgConnection>::establish(&format!(
            "postgres://postgres:graph_benchmarks@{}:30002/postgres",
            master_ip
        )).unwrap();
        connection.run_pending_migrations(MIGRATIONS).unwrap();
    }).join().unwrap();
    Ok(())
}

async fn get_run_id(conn: &mut AsyncPgConnection, n_nodes: usize) -> Result<i32> {
    use crate::schema::benchmarks::{self, dsl::*};
    let b: Benchmark = diesel::insert_into(benchmarks::table)
        .values(nodes.eq(n_nodes as i32))
        .returning(Benchmark::as_returning())
        .get_result(conn)
        .await?;
    Ok(b.id)
}
