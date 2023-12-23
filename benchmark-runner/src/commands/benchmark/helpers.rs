use std::{collections::BTreeMap, time::Instant};

use anyhow::Result;
use common::command::{finish_progress, progress};
use futures_util::{future::join_all, StreamExt, TryStreamExt};
use k8s_openapi::api::{
    batch::v1::{Job, JobSpec},
    core::v1::{
        Container, ContainerPort, EnvVar, NFSVolumeSource, Pod, PodSpec, PodTemplateSpec, Service,
        ServicePort, ServiceSpec, Volume, VolumeMount,
    },
};
use kube::{
    api::{DeleteParams, ListParams, PostParams, WatchParams},
    core::WatchEvent,
    Api, Client,
};
use regex::Regex;
use tokio::spawn;
use tracing::info;

use super::{types::Run, POSTGRES_CONFIG};

fn env_var(name: &str, value: &str) -> EnvVar {
    EnvVar {
        name: name.into(),
        value: Some(value.into()),
        value_from: None,
    }
}

async fn visualize(
    job_name: String,
    host_ip: String,
    run_ids: Vec<i32>,
    nfs_ip: String,
    graph_type: &str,
) -> Result<()> {
    let mut job_spec = Job::default();
    job_spec.metadata.name = Some(job_name.clone());
    job_spec.metadata.labels = Some(BTreeMap::from([("app".into(), "visualization".into())]));
    job_spec.spec = Some(JobSpec {
        backoff_limit: Some(0),
        ttl_seconds_after_finished: Some(0),
        template: PodTemplateSpec {
            spec: Some(PodSpec {
                restart_policy: Some("Never".into()),
                init_containers: Some(vec![Container {
                    name: "create-folder".into(),
                    image: Some("busybox".into()),
                    command: Some(
                        vec!["sh", "-c", &format!("mkdir -p /attached/{job_name}")]
                            .into_iter()
                            .map(|x| x.to_owned())
                            .collect(),
                    ),
                    volume_mounts: Some(vec![VolumeMount {
                        mount_path: "/attached".into(),
                        name: "visualize-storage".into(),
                        ..VolumeMount::default()
                    }]),
                    ..Default::default()
                }]),
                containers: vec![Container {
                    image: Some(format!("{host_ip}:30000/system/graphs:latest")),
                    image_pull_policy: Some("Always".into()),
                    name: "graphs-vis".into(),
                    volume_mounts: Some(vec![VolumeMount {
                        mount_path: "/attached".into(),
                        name: "visualize-storage".into(),
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
                            "SELECT_LOG_IDS",
                            &run_ids
                                .iter()
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>()
                                .join(","),
                        ),
                        env_var("GENERATE_GRAPHS", graph_type),
                    ]),
                    ..Container::default()
                }],
                volumes: Some(vec![Volume {
                    name: "visualize-storage".into(),
                    nfs: Some(NFSVolumeSource {
                        path: "/visualize-storage".to_owned(),
                        server: nfs_ip,
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

    let wait_for_job_close = spawn(async move {
        if let Ok(jobs) = jobs
            .list(&ListParams::default().labels("app=visualization"))
            .await
        {
            if jobs.items.len() == 0 {
                return;
            }
        }

        let mut job_stream = jobs
            .watch(&WatchParams::default().labels("app=visualization"), "0")
            .await
            .unwrap()
            .boxed();
        while let Ok(Some(status)) = job_stream.try_next().await {
            match status {
                WatchEvent::Deleted(_) => break,
                s => info!("got event: {s:?}"),
            }
        }
    });
    wait_for_job_close.await?;

    Ok(())
}

pub async fn start_metrics(host_ip: &str) -> Result<()> {
    _ = stop_pod_service("graph-metrics").await;

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::default_namespaced(client.clone());
    let mut pod_spec = Pod::default();
    pod_spec.metadata.name = Some("graph-metrics".into());
    pod_spec.metadata.labels = Some(BTreeMap::from([("app".into(), "graph-metrics".into())]));
    pod_spec.spec = Some(PodSpec {
        node_selector: Some(BTreeMap::from([(
            "node-role.kubernetes.io/master".into(),
            "true".into(),
        )])),
        containers: vec![Container {
            name: "graph-metrics".into(),
            args: Some(
                vec![
                    "-psql-host",
                    POSTGRES_CONFIG.host,
                    "-psql-port",
                    &POSTGRES_CONFIG.port.to_string(),
                    "-psql-username",
                    POSTGRES_CONFIG.user,
                    "-psql-password",
                    POSTGRES_CONFIG.ps,
                    "-psql-db",
                    POSTGRES_CONFIG.db,
                ]
                .into_iter()
                .map(|x| x.to_owned())
                .collect(),
            ),
            ports: Some(vec![ContainerPort {
                container_port: 9090,
                ..Default::default()
            }]),
            image: Some(format!("{host_ip}:30000/system/metrics:latest")),
            ..Container::default()
        }],
        service_account_name: Some("admin-user".into()),
        ..PodSpec::default()
    });
    pods.create(&PostParams::default(), &pod_spec).await?;

    let service: Api<Service> = Api::default_namespaced(client);
    let mut service_spec = Service::default();
    service_spec.metadata.name = Some("graph-metrics".into());
    service_spec.metadata.namespace = Some("default".into());
    service_spec.spec = Some(ServiceSpec {
        selector: Some(BTreeMap::from([("app".into(), "graph-metrics".into())])),
        type_: Some("NodePort".into()),
        ports: Some(vec![ServicePort {
            port: 9090,
            node_port: Some(30001),
            ..Default::default()
        }]),
        ..Default::default()
    });
    service
        .create(&PostParams::default(), &service_spec)
        .await?;
    Ok(())
}

pub async fn stop_pod_service(name: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::default_namespaced(client.clone());
    let service: Api<Service> = Api::default_namespaced(client);
    _ = pods
        .delete(name, &DeleteParams::default().grace_period(0))
        .await;
    _ = service
        .delete(name, &DeleteParams::default().grace_period(0))
        .await;
    Ok(())
}

pub async fn start_notifier(host_ip: String) -> Result<()> {
    _ = stop_pod_service("notifier").await;

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::default_namespaced(client.clone());
    let mut pod_spec = Pod::default();
    pod_spec.metadata.name = Some("notifier".into());
    pod_spec.metadata.labels = Some(BTreeMap::from([("app".into(), "notifier".into())]));
    pod_spec.spec = Some(PodSpec {
        node_selector: Some(BTreeMap::from([(
            "node-role.kubernetes.io/master".into(),
            "true".into(),
        )])),
        containers: vec![Container {
            name: "notifier".into(),
            ports: Some(vec![ContainerPort {
                container_port: 8080,
                ..Default::default()
            }]),
            image: Some(format!("{host_ip}:30000/system/notifier:latest")),
            ..Container::default()
        }],
        ..PodSpec::default()
    });
    pods.create(&PostParams::default(), &pod_spec).await?;

    let service: Api<Service> = Api::default_namespaced(client);
    let mut service_spec = Service::default();
    service_spec.metadata.name = Some("notifier".into());
    service_spec.metadata.namespace = Some("default".into());
    service_spec.spec = Some(ServiceSpec {
        selector: Some(BTreeMap::from([("app".into(), "notifier".into())])),
        type_: Some("NodePort".into()),
        ports: Some(vec![ServicePort {
            port: 8080,
            node_port: Some(30003),
            ..Default::default()
        }]),
        ..Default::default()
    });
    service
        .create(&PostParams::default(), &service_spec)
        .await?;
    Ok(())
}

pub async fn visualize_dataset_algos(
    runs: &[Run],
    driver: &str,
    n_nodes: usize,
    host_ip: String,
    nfs_ip: String,
) -> Result<()> {
    let pb = progress(&format!("Generating visualization for ({driver})"));
    let start = Instant::now();
    visualize(
        format!("{driver}-{n_nodes}"),
        host_ip,
        runs.iter().map(|x| x.run_id).collect::<Vec<i32>>(),
        nfs_ip.clone(),
        "bars",
    )
    .await?;
    finish_progress(
        "Generated visualization for",
        driver,
        start.elapsed(),
        Some(pb),
    );
    Ok(())
}

pub async fn visualize_algos_workers(
    runs: &[Run],
    datasets: &[String],
    host_ip: String,
    nfs_ip: String,
) -> Result<()> {
    let pb = progress(&format!("Generating overall visualizations"));
    let start = Instant::now();
    let mut jobs = Vec::new();
    let r = Regex::new(r#"[^a-z0-9-]"#)?;
    for dataset in datasets {
        jobs.push(visualize(
            format!("{}", r.replace(dataset, "")),
            host_ip.clone(),
            runs.iter()
                .filter(|x| x.dataset.eq(dataset))
                .map(|x| x.run_id)
                .collect::<Vec<i32>>(),
            nfs_ip.clone(),
            "lines",
        ));
    }
    let j: Result<Vec<()>, _> = join_all(jobs).await.into_iter().collect();
    j?;
    finish_progress(
        "Generated overall visualizations ",
        "",
        start.elapsed(),
        Some(pb),
    );
    Ok(())
}
