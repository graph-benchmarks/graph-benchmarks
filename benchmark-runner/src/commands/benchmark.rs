use std::{
    collections::{BTreeMap, HashMap},
    env,
    net::IpAddr,
    time::Instant,
};

use anyhow::{bail, Result};
use common::{
    command::{command_print, finish_progress, progress},
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
use futures_util::{future::join_all, StreamExt, TryStreamExt};
use k8s_openapi::{
    api::{
        batch::v1::{Job, JobSpec},
        core::v1::{
            ConfigMap, ConfigMapVolumeSource, Container, EnvVar, HostPathVolumeSource, Node,
            PersistentVolume, PersistentVolumeClaim, PersistentVolumeClaimSpec,
            PersistentVolumeClaimVolumeSource, PersistentVolumeSpec, Pod, PodSpec, PodTemplateSpec,
            ResourceRequirements, Volume, VolumeMount,
        },
    },
    apimachinery::pkg::api::resource::Quantity,
};
use kube::{
    api::{DeleteParams, ListParams, PostParams},
    runtime::{
        conditions::{is_job_completed, is_pod_running},
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
const POSTGRES_CONFIG: PostgresConfig = PostgresConfig {
    host: "postgres",
    db: "postgres",
    user: "postgres",
    port: 5432,
    ps: "graph_benchmarks",
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Run {
    dataset: String,
    algorithm: String,
    nodes: usize,
    run_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlatformConfig {
    host: String,
    port: u16,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct DatasetConfig {
    vertex: String,
    edges: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunConfig {
    id: i32,
    algo: String,
    log_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DriverConfig<'a> {
    config: RunConfig,
    #[serde(borrow)]
    postgres: PostgresConfig<'a>,
    platform: PlatformConfig,
    dataset: DatasetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostgresConfig<'a> {
    host: &'a str,
    port: u32,
    db: &'a str,
    user: &'a str,
    ps: &'a str,
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
        "postgres://{}:{}@{}:30002/{}",
        POSTGRES_CONFIG.user, POSTGRES_CONFIG.ps, connect_args.master_ip, POSTGRES_CONFIG.db
    ))
    .await?;

    env::set_var("KUBECONFIG", "k3s/kube-config");
    setup_pv_and_pvc("benchmark", "/benchmark-pv", "10Gi", "10Gi").await?;
    setup_pv_and_pvc("visualize", "/visualize-pv", "10Gi", "50Mi").await?;

    let mut runs: Vec<Run> = Vec::new();

    config.setup.node_configs.sort_by(|a, b| b.cmp(a));
    for n_nodes in config.setup.node_configs {
        new_cluster_node_count(n_nodes).await?;
        for driver in &config.benchmark.drivers {
            let driver_config = match base_driver::get_driver_config(driver) {
                Some(d) => d,
                None => exit!("", "Could not find driver {}", driver),
            };
            let runs_start_point = runs.len();

            driver_config.set_node_config(n_nodes, 2, 1024).await?;
            setup_graph_platform(&driver, &connect_args, cli.verbose).await?;
            let service_ip = driver_config.get_service_ip().await?;
            let pod_ids = driver_config.metrics_pod_ids().await?;

            let mut cfg = DriverConfig {
                postgres: POSTGRES_CONFIG,
                platform: PlatformConfig {
                    host: service_ip.0,
                    port: service_ip.1,
                },
                dataset: DatasetConfig::default(),
                config: RunConfig {
                    id: 0,
                    algo: "".into(),
                    log_file: "/attached/log".into(),
                },
            };

            for dataset in &config.benchmark.datasets {
                copy_dataset(dataset, &connect_args.master_ip.to_string(), cli.verbose).await?;

                for algorithm in config.benchmark.algorithms.as_ref().unwrap_or(
                    &ALGORITHMS
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>(),
                ) {
                    let pb = progress(&format!("Benchmarking ({algorithm} on {dataset})"));
                    let run_id = get_run_id(&mut connection, n_nodes).await?;
                    runs.push(Run {
                        run_id,
                        dataset: dataset.clone(),
                        algorithm: algorithm.clone(),
                        nodes: n_nodes,
                    });

                    cfg.dataset = DatasetConfig {
                        name: dataset.clone(),
                        vertex: format!("/attached/{dataset}.v"),
                        edges: format!("/attached/{dataset}.e"),
                    };
                    cfg.config.algo = algorithm.clone();
                    cfg.config.id = run_id;
                    info!("{cfg:#?}");

                    let start = Instant::now();
                    let bench_job = start_bench(&driver, &connect_args.master_ip, &cfg).await?;
                    let metrics_ip = format!("{}:30001", connect_args.master_ip);
                    start_recording(metrics_ip.clone(), pod_ids.clone(), run_id).await?;

                    let client = Client::try_default().await?;
                    let api: Api<Job> = Api::default_namespaced(client);
                    await_condition(
                        api.clone(),
                        bench_job.metadata.name.as_ref().unwrap(),
                        is_job_completed(),
                    )
                    .await?;

                    stop_recording(metrics_ip, pod_ids.clone(), run_id).await?;
                    finish_progress(
                        "Done benchmarking",
                        &format!("{algorithm} on {dataset}"),
                        start.elapsed(),
                        Some(pb),
                    );
                }
            }
            visualize_dataset_algos(
                &runs[runs_start_point..],
                &driver,
                n_nodes,
                connect_args.master_ip.to_string(),
            )
            .await?;
            remove_driver(&driver, &connect_args, cli.verbose).await?;
        }
    }

    visualize_algos_workers(
        &runs,
        &config.benchmark.datasets,
        connect_args.master_ip.to_string(),
    )
    .await?;
    copy_generated_graphs(cli.verbose, &connect_args).await?;

    Ok(())
}

async fn copy_generated_graphs(verbose: bool, connect_args: &PlatformInfo) -> Result<()> {
    let mut env = HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]);
    if verbose {
        env.insert("DEBUG_ANSIBLE", "1");
    }

    command_print(
        "ansible-playbook",
        &[
            "copy-graphs.yaml",
            "--private-key",
            &connect_args.ssh_key,
            "-i",
            "inventory/master-hosts.yaml",
            "-i",
            "inventory/worker-hosts.yaml",
        ],
        verbose,
        [
            &format!("Copying generated graphs"),
            &format!("Could not copy generated graphs"),
            &format!("Copied generated graphs"),
        ],
        "k3s",
        env,
    )
    .await
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

    command_print(
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

    command_print(
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

async fn start_bench(name: &str, host_ip: &IpAddr, cfg: &DriverConfig<'_>) -> Result<Job> {
    let client = Client::try_default().await?;

    let default_pp = PostParams {
        dry_run: false,
        field_manager: None,
    };

    let job_name = format!("{}-bench", name);

    let config_map: Api<ConfigMap> = Api::default_namespaced(client.clone());
    _ = config_map
        .delete(&job_name, &DeleteParams::default().grace_period(0))
        .await;

    let mut config_map_spec = ConfigMap::default();
    config_map_spec.metadata.name = Some(job_name.clone());
    config_map_spec.data = Some(BTreeMap::from([(
        "config.yaml".into(),
        serde_yaml::to_string(&cfg)?,
    )]));
    config_map_spec.immutable = Some(true);
    config_map.create(&default_pp, &config_map_spec).await?;

    let jobs: Api<Job> = Api::default_namespaced(client);
    _ = jobs
        .delete(&job_name, &DeleteParams::default().grace_period(0))
        .await;

    let mut job_spec = Job::default();
    job_spec.metadata.name = Some(job_name.clone());
    job_spec.spec = Some(JobSpec {
        backoff_limit: Some(0),
        ttl_seconds_after_finished: Some(0),
        template: PodTemplateSpec {
            spec: Some(PodSpec {
                restart_policy: Some("Never".into()),
                containers: vec![Container {
                    args: Some(vec!["/cfg/config.yaml".into()]),
                    image: Some(format!("{host_ip}:30000/benches/{name}:latest")),
                    image_pull_policy: Some("Always".into()),
                    name: format!("{}-bench", name).into(),
                    volume_mounts: Some(vec![
                        VolumeMount {
                            mount_path: "/attached".into(),
                            name: "benchmark-pvc".into(),
                            ..VolumeMount::default()
                        },
                        VolumeMount {
                            name: job_name.clone(),
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
                        name: job_name.clone(),
                        config_map: Some(ConfigMapVolumeSource {
                            name: Some(job_name.clone()),
                            ..ConfigMapVolumeSource::default()
                        }),
                        ..Volume::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    });

    Ok(jobs.create(&default_pp, &job_spec).await?)
}

async fn setup_pv_and_pvc(
    name: &str,
    mount_path: &str,
    size: &str,
    request_size: &str,
) -> Result<()> {
    let default_pp = PostParams {
        dry_run: false,
        field_manager: None,
    };

    let client = Client::try_default().await?;
    let pv: Api<PersistentVolume> = Api::all(client.clone());
    let mut pv_spec = PersistentVolume::default();
    pv_spec.metadata.name = Some(format!("{name}-pv"));
    pv_spec.metadata.labels = Some(BTreeMap::from([("type".into(), "local".into())]));
    pv_spec.spec = Some(PersistentVolumeSpec {
        access_modes: Some(vec!["ReadWriteOnce".into()]),
        capacity: Some(BTreeMap::from([("storage".into(), Quantity(size.into()))])),
        persistent_volume_reclaim_policy: Some("Retain".into()),
        host_path: Some(HostPathVolumeSource {
            path: mount_path.into(),
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
    pvc_spec.metadata.name = Some(format!("{name}-pvc"));
    pvc_spec.spec = Some(PersistentVolumeClaimSpec {
        access_modes: Some(vec!["ReadWriteOnce".into()]),
        resources: Some(ResourceRequirements {
            requests: Some(BTreeMap::from([(
                "storage".into(),
                Quantity(request_size.into()),
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
            "postgres://{}:{}@{}:30002/{}",
            POSTGRES_CONFIG.user, POSTGRES_CONFIG.ps, master_ip, POSTGRES_CONFIG.db
        ))
        .unwrap();
        connection.run_pending_migrations(MIGRATIONS).unwrap();
    })
    .join()
    .unwrap();
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

async fn copy_dataset(dataset: &str, host_ip: &str, verbose: bool) -> Result<()> {
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::default_namespaced(client);
    let mut pod_spec = Pod::default();
    pod_spec.metadata.name = Some("dataset-copy".into());
    pod_spec.spec = Some(PodSpec {
        containers: vec![Container {
            name: "dataset-copy".into(),
            image: Some(format!("{host_ip}:30000/system/rsync:latest")),
            volume_mounts: Some(vec![VolumeMount {
                mount_path: "/attached".into(),
                name: "benchmark-pvc".into(),
                ..VolumeMount::default()
            }]),
            ..Container::default()
        }],
        volumes: Some(vec![Volume {
            name: "benchmark-pvc".into(),
            persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                claim_name: "benchmark-pvc".into(),
                read_only: Some(false),
            }),
            ..Volume::default()
        }]),
        ..PodSpec::default()
    });

    _ = pods
        .delete("dataset-copy", &DeleteParams::default().grace_period(0))
        .await;

    await_condition(pods.clone(), "dataset-copy", is_pod_running().not()).await?;

    pods.create(&PostParams::default(), &pod_spec).await?;

    await_condition(pods.clone(), "dataset-copy", is_pod_running()).await?;
    command_print(
        "./krsync.sh",
        &[
            "-av",
            "--progress",
            "--stats",
            &format!("../datasets/{dataset}/"),
            "dataset-copy:/attached",
        ],
        verbose,
        [
            "Copying dataset",
            "Could not copy dataset",
            "Copied dataset",
        ],
        "k3s",
        HashMap::<&str, &str>::from([("KUBECONFIG", "kube-config")]),
    )
    .await?;

    _ = pods
        .delete("dataset-copy", &DeleteParams::default().grace_period(0))
        .await;

    await_condition(pods.clone(), "dataset-copy", is_pod_running().not()).await?;

    Ok(())
}

async fn visualize_dataset_algos(
    runs: &[Run],
    driver: &str,
    n_nodes: usize,
    host_ip: String,
) -> Result<()> {
    visualize(
        format!("{driver}_{n_nodes}"),
        host_ip,
        runs.iter().map(|x| x.run_id).collect::<Vec<i32>>(),
    )
    .await
}

async fn visualize_algos_workers(runs: &[Run], datasets: &[String], host_ip: String) -> Result<()> {
    let mut jobs = Vec::new();
    for dataset in datasets {
        jobs.push(visualize(
            format!("{dataset}"),
            host_ip.clone(),
            runs.iter().map(|x| x.run_id).collect::<Vec<i32>>(),
        ));
    }
    let j: Result<Vec<()>, _> = join_all(jobs).await.into_iter().collect();
    j?;
    Ok(())
}

fn env_var(name: &str, value: &str) -> EnvVar {
    EnvVar {
        name: name.into(),
        value: Some(value.into()),
        value_from: None,
    }
}

async fn visualize(job_name: String, host_ip: String, run_ids: Vec<i32>) -> Result<()> {
    let mut job_spec = Job::default();
    job_spec.metadata.name = Some(job_name.clone());
    job_spec.spec = Some(JobSpec {
        backoff_limit: Some(0),
        ttl_seconds_after_finished: Some(0),
        template: PodTemplateSpec {
            spec: Some(PodSpec {
                restart_policy: Some("Never".into()),
                containers: vec![Container {
                    args: Some(vec!["/cfg/config.yaml".into()]),
                    image: Some(format!("{host_ip}:30000/benches/graphs:latest")),
                    image_pull_policy: Some("Always".into()),
                    name: "graphs-vis".into(),
                    volume_mounts: Some(vec![VolumeMount {
                        mount_path: "/attached".into(),
                        name: "visualize-pvc".into(),
                        ..VolumeMount::default()
                    }]),
                    env: Some(vec![
                        env_var("POSTGRES_HOST", POSTGRES_CONFIG.host),
                        env_var("POSTGRES_PORT", &POSTGRES_CONFIG.port.to_string()),
                        env_var("POSTGRES_USER", POSTGRES_CONFIG.user),
                        env_var("POSTGRES_PASSWORD", POSTGRES_CONFIG.ps),
                        env_var("POSTGRES_DB", POSTGRES_CONFIG.db),
                        env_var("OUTPUT_DIR", &format!("/attached/{job_name}")),
                        env_var(
                            "SELECT_LOG_ID",
                            &run_ids
                                .iter()
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>()
                                .join(","),
                        ),
                        env_var("GENERATE_GRAPHS", "bars"),
                    ]),
                    ..Container::default()
                }],
                volumes: Some(vec![Volume {
                    name: "visualize-pvc".into(),
                    persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                        claim_name: "visualize-pvc".into(),
                        read_only: Some(false),
                    }),
                    ..Volume::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    });

    let jobs: Api<Job> = Api::default_namespaced(Client::try_default().await?);
    jobs.create(&PostParams::default(), &job_spec).await?;

    Ok(())
}
