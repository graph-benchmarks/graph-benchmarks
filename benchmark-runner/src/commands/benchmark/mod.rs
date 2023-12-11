use std::{
    collections::{BTreeMap, HashMap},
    env,
    net::IpAddr,
    time::Instant,
};

use anyhow::Result;
use common::{
    command::{command_print, finish_progress, progress, GREEN_TICK},
    config::parse_config,
    exit,
    provider::PlatformInfo,
};
use diesel::{Connection, ExpressionMethods, SelectableHelper};
use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper, AsyncConnection, AsyncPgConnection,
    RunQueryDsl,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::api::{
    batch::v1::{Job, JobSpec},
    core::v1::{
        ConfigMap, ConfigMapVolumeSource, Container, NFSVolumeSource, Node, Pod, PodSpec,
        PodTemplateSpec, Service, Volume, VolumeMount,
    },
};
use kube::{
    api::{DeleteParams, ListParams, PostParams, WatchParams},
    core::{ObjectMeta, WatchEvent},
    Api, Client,
};
use tokio::{fs, net::TcpStream, spawn};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::info;

use crate::{
    args::Cli,
    metrics_utils::{start_recording, stop_recording},
    model::Benchmark,
};

use self::{ansible::*, helpers::*, types::*};

mod ansible;
mod helpers;
mod types;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
const ALGORITHMS: &[&str] = &["bfs", "pr", "wcc", "cdlp", "lcc", "sssp"];
const POSTGRES_CONFIG: PostgresConfig = PostgresConfig {
    host: "postgres",
    db: "postgres",
    user: "postgres",
    port: 5432,
    ps: "graph_benchmarks",
};

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
    config.setup.node_configs.sort_by(|a, b| b.cmp(a));

    start_notifier(connect_args.master_ip.to_string()).await?;
    copy_datasets(
        &config.benchmark.datasets,
        &connect_args,
        config
            .setup
            .host_username
            .as_ref()
            .unwrap_or(&"root".to_owned()),
        cli.verbose,
    )
    .await?;
    clear_dirs(&connect_args, cli.verbose).await?;

    let client = Client::try_default().await?;
    let nodes: Api<Node> = Api::all(client);
    let nodes = nodes.list(&ListParams::default()).await?;
    if nodes.items.len() != config.setup.node_configs[0] {
        join_all_nodes(&connect_args, cli.verbose).await?;
    }
    start_metrics(&connect_args.master_ip.to_string()).await?;

    let svc: Api<Service> = Api::default_namespaced(Client::try_default().await?);
    let nfs_ip = svc
        .get("nfs-service")
        .await?
        .spec
        .unwrap()
        .cluster_ip
        .unwrap();

    let mut runs: Vec<Run> = Vec::new();

    let (mut ws_stream, _) =
        connect_async(format!("ws://{}:30003/ws", connect_args.master_ip)).await?;

    for n_nodes in config.setup.node_configs {
        new_cluster_node_count(n_nodes).await?;
        for driver in &config.benchmark.drivers {
            let driver_config = match base_driver::get_driver_config(driver) {
                Some(d) => d,
                None => exit!("", "Could not find driver {}", driver),
            };
            let runs_start_point = runs.len();

            info!("{:#?}", config.setup.graph_platform_args);
            let extra_vars = driver_config
                .scale_service(
                    n_nodes,
                    config
                        .setup
                        .graph_platform_args
                        .as_ref()
                        .unwrap_or(&HashMap::new())
                        .get(driver)
                        .map(|x| x.to_owned()),
                )
                .await?;
            setup_graph_platform(
                &driver,
                &connect_args,
                n_nodes,
                extra_vars.clone(),
                cli.verbose,
            )
            .await?;
            let platform_config = driver_config.get_platform_config(n_nodes).await?;
            let pod_ids = driver_config.metrics_pod_ids().await?;
            info!("pod ids: {pod_ids:?}");

            let mut cfg = DriverConfig {
                postgres: POSTGRES_CONFIG,
                platform: platform_config,
                dataset: DatasetConfig::default(),
                config: RunConfig {
                    ids: "".into(),
                    algos: "".into(),
                    log_file: "/attached/log".into(),
                    nodes: n_nodes,
                },
            };

            for dataset in &config.benchmark.datasets {
                let algos = config.benchmark.algorithms.clone().unwrap_or(
                    ALGORITHMS
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>(),
                );

                println!("Benchmarking {dataset} {} times", config.benchmark.repeat);
                for _ in 0..config.benchmark.repeat {
                    let run_ids = get_run_ids(&mut connection, n_nodes, algos.len()).await?;
                    run_ids
                        .iter()
                        .zip(algos.clone())
                        .for_each(|(run_id, algo)| {
                            runs.push(Run {
                                run_id: *run_id,
                                dataset: dataset.clone(),
                                algorithm: algo.clone(),
                                nodes: n_nodes,
                            });
                        });

                    let d: DatasetUserConfig = toml::from_str(
                        &fs::read_to_string(format!("datasets/{dataset}/config.toml")).await?,
                    )?;

                    cfg.dataset = DatasetConfig {
                        name: dataset.clone(),
                        vertex: format!("/attached/{dataset}.v"),
                        edges: format!("/attached/{dataset}.e"),
                        weights: d.weights,
                        directed: d.directed,
                    };
                    cfg.config.algos = algos.join(",");
                    cfg.config.ids = run_ids
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(",");
                    info!("{cfg:#?}");

                    start_bench(
                        &driver,
                        &connect_args.master_ip,
                        &cfg,
                        nfs_ip.clone(),
                        config.benchmark.debug.clone().unwrap_or_default().bench_ttl,
                    )
                    .await?;

                    let metrics_ip = format!("http://{}:30001", connect_args.master_ip);
                    for i in 0..run_ids.len() {
                        let pb = progress(&format!("Benchmarking ({} on {dataset})", algos[i]));
                        let start = Instant::now();
                        match ws_stream.try_next().await? {
                            Some(msg) => {
                                let msg: BenchStartEvent = serde_json::from_str(&msg.into_text()?)?;
                                if !msg.status {
                                    exit!(
                                        "",
                                        "Expected bench starting message, got bench ending message"
                                    );
                                }
                            }
                            None => exit!("", "Received incorrect benchmark starting signal"),
                        }

                        start_recording(metrics_ip.clone(), pod_ids.clone(), run_ids[i]).await?;
                        info!("started recording metrics on {metrics_ip}");

                        ws_stream = wait_for_ws_end_message(
                            ws_stream,
                            metrics_ip.clone(),
                            pod_ids.clone(),
                            run_ids[i],
                        )
                        .await?;

                        finish_progress(
                            "Done benchmarking",
                            &format!("{} on {dataset}", algos[i]),
                            start.elapsed(),
                            Some(pb),
                        );
                    }

                    wait_for_bench_delete().await?;
                }
            }
            visualize_dataset_algos(
                &runs[runs_start_point..],
                &driver,
                n_nodes,
                connect_args.master_ip.to_string(),
                nfs_ip.clone(),
            )
            .await?;
            remove_graph_platform(&driver, &connect_args, extra_vars, cli.verbose).await?;
        }
    }

    ws_stream.close(None).await?;

    visualize_algos_workers(
        &runs,
        &config.benchmark.datasets,
        connect_args.master_ip.to_string(),
        nfs_ip.clone(),
    )
    .await?;
    copy_generated_graphs(cli.verbose, &connect_args).await?;

    stop_pod_service("metrics").await?;
    stop_pod_service("notifier").await?;

    Ok(())
}

async fn wait_for_ws_end_message(
    mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    metrics_ip: String,
    pod_ids: Vec<String>,
    run_id: i32,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    match ws_stream.try_next().await? {
        Some(msg) => {
            let msg: BenchStartEvent = serde_json::from_str(&msg.into_text()?)?;
            if msg.status {
                exit!(
                    "",
                    "Expected bench stopping message, got bench starting message"
                );
            }
            stop_recording(metrics_ip, pod_ids, run_id).await?;
            info!("stopped recording metrics");
            Ok(ws_stream)
        }
        None => exit!("", "Received incorrect benchmark stopping signal"),
    }
}

async fn wait_for_bench_delete() -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<Job> = Api::default_namespaced(client);

    let mut job_stream = api
        .watch(&WatchParams::default().labels("app=graph-bench"), "0")
        .await
        .unwrap()
        .boxed();
    while let Ok(Some(status)) = job_stream.try_next().await {
        match status {
            WatchEvent::Deleted(_) => break,
            s => info!("got event: {s:?}"),
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
            info!("Removing node {node}");
            api.delete(&node, &DeleteParams::default()).await?;
            println!("{} Removed node {node}", GREEN_TICK.to_string());
        }
    }
    Ok(())
}

async fn setup_graph_platform(
    name: &str,
    connect_args: &PlatformInfo,
    nodes: usize,
    extra_vars: Vec<String>,
    verbose: bool,
) -> Result<()> {
    let mut args = vec![
        "setup.yaml",
        "--private-key",
        &connect_args.ssh_key,
        "-i",
        "../../k3s/inventory/master-hosts.yaml",
    ];

    let extra_vars_str = extra_vars.join(" ");
    if extra_vars.len() > 0 {
        args.push("--extra-vars");
        args.push(&extra_vars_str);
    }

    command_print(
        "ansible-playbook",
        &args,
        verbose,
        [
            &format!("Installing {name}"),
            &format!("Could not install {name}"),
            &format!("Installed {name}"),
        ],
        &format!("drivers/{}", name),
        HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]),
    )
    .await?;

    let start = Instant::now();
    let pb = progress(&format!("Waiting for {name} to be ready"));

    base_driver::get_driver_config(name)
        .unwrap()
        .wait_for_service_ready(nodes)
        .await?;

    finish_progress(
        "Platform ready",
        &format!("drivers/{name}"),
        start.elapsed(),
        Some(pb),
    );
    Ok(())
}

async fn start_bench(
    name: &str,
    host_ip: &IpAddr,
    cfg: &DriverConfig<'_>,
    nfs_ip: String,
    bench_ttl: Option<i32>,
) -> Result<()> {
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

    let c = client.clone();
    let wait_for_job_close = spawn(async {
        let jobs: Api<Job> = Api::default_namespaced(c);

        if let Ok(jobs) = jobs
            .list(&ListParams::default().labels("app=graph-bench"))
            .await
        {
            if jobs.items.len() == 0 {
                return;
            }
        }

        let mut job_stream = jobs
            .watch(&WatchParams::default().labels("app=graph-bench"), "0")
            .await
            .unwrap()
            .boxed();
        while let Ok(Some(status)) = job_stream.try_next().await {
            match status {
                WatchEvent::Deleted(_) => break,
                s => info!("got event: {s:?}"),
            }
        }
        info!("old jobs deleted!");
    });

    let jobs: Api<Job> = Api::default_namespaced(client.clone());
    _ = jobs
        .delete(&job_name, &DeleteParams::default().grace_period(0))
        .await;

    let pods: Api<Pod> = Api::default_namespaced(client);
    _ = pods
        .delete_collection(
            &DeleteParams::default().grace_period(0),
            &ListParams::default().labels("app=graph-bench"),
        )
        .await;

    wait_for_job_close.await?;

    let mut job_spec = Job::default();
    job_spec.metadata = ObjectMeta {
        name: Some(job_name.clone()),
        labels: Some(BTreeMap::from([("app".into(), "graph-bench".into())])),
        ..Default::default()
    };
    job_spec.spec = Some(JobSpec {
        backoff_limit: Some(0),
        ttl_seconds_after_finished: bench_ttl,
        template: PodTemplateSpec {
            metadata: Some(ObjectMeta {
                labels: Some(BTreeMap::from([("app".into(), "graph-bench".into())])),
                ..Default::default()
            }),
            spec: Some(PodSpec {
                service_account_name: Some("admin-user".into()),
                restart_policy: Some("Never".into()),
                containers: vec![Container {
                    args: Some(vec!["/cfg/config.yaml".into()]),
                    image: Some(format!("{host_ip}:30000/benches/{name}:latest")),
                    image_pull_policy: Some("Always".into()),
                    name: format!("{}-bench", name).into(),
                    volume_mounts: Some(vec![
                        VolumeMount {
                            mount_path: "/attached".into(),
                            name: "bench-storage".into(),
                            ..VolumeMount::default()
                        },
                        VolumeMount {
                            mount_path: "/scratch".into(),
                            name: "scratch".into(),
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
                        name: "bench-storage".into(),
                        nfs: Some(NFSVolumeSource {
                            path: "/bench-storage".to_owned(),
                            server: nfs_ip.clone(),
                            read_only: Some(false),
                        }),
                        ..Volume::default()
                    },
                    Volume {
                        name: "scratch".into(),
                        nfs: Some(NFSVolumeSource {
                            path: "/scratch".to_owned(),
                            server: nfs_ip,
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

    jobs.create(&default_pp, &job_spec).await?;
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

async fn get_run_ids(
    conn: &mut AsyncPgConnection,
    n_nodes: usize,
    count: usize,
) -> Result<Vec<i32>> {
    use crate::schema::benchmarks::{self, dsl::*};
    let b = diesel::insert_into(benchmarks::table)
        .values(
            (0..count)
                .into_iter()
                .map(|_| nodes.eq(n_nodes as i32))
                .collect::<Vec<_>>(),
        )
        .returning(Benchmark::as_returning())
        .get_results(conn)
        .await?;
    Ok(b.into_iter().map(|x| x.id).collect())
}
