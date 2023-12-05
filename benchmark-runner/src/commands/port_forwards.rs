use std::env;

use anyhow::{Context, Result};
use common::exit;
use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::api::{
    authentication::v1::{TokenRequest, TokenRequestSpec},
    core::v1::{Pod, ServiceAccount},
};
use kube::{
    api::{ListParams, PostParams},
    Api, Client,
};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tracing::info;

use crate::args::Cli;

pub async fn dashboard(_: &Cli) -> Result<()> {
    env::set_var("KUBECONFIG", "k3s/kube-config");
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::default_namespaced(client.clone());
    let p = pods.list(&ListParams::default().labels("app.kubernetes.io/name=kubernetes-dashboard,app.kubernetes.io/instance=kubernetes-dashboard")).await?;
    if p.items.is_empty() {
        exit!("", "Could not find dashboard in cluster");
    }

    let token: Api<ServiceAccount> = Api::default_namespaced(client);
    let token = token
        .create_token_request(
            "admin-user",
            &PostParams::default(),
            &TokenRequest {
                spec: TokenRequestSpec {
                    expiration_seconds: Some(3600),
                    ..TokenRequestSpec::default()
                },
                ..TokenRequest::default()
            },
        )
        .await?;
    println!("Access token: {}", token.status.unwrap().token);

    let pod_name = p.items[0].metadata.name.as_ref().unwrap().clone();
    server(pods, pod_name, 8443).await?;
    Ok(())
}

pub async fn postgres(_: &Cli) -> Result<()> {
    env::set_var("KUBECONFIG", "k3s/kube-config");
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::default_namespaced(client.clone());
    let p = pods
        .list(&ListParams::default().labels("app=postgres"))
        .await?;
    if p.items.is_empty() {
        exit!("", "Could not find dashboard in cluster");
    }

    let pod_name = p.items[0].metadata.name.as_ref().unwrap().clone();
    server(pods, pod_name, 5432).await?;
    Ok(())
}

pub async fn server(pods: Api<Pod>, pod_name: String, port: u16) -> Result<()> {
    println!("\nListening on {port}!");
    let server =
        TcpListenerStream::new(TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap())
            .take_until(tokio::signal::ctrl_c())
            .try_for_each(|mut client_conn| async {
                let pods = pods.clone();
                let pn = pod_name.clone();
                tokio::spawn(async move {
                    let mut forwarder = pods.portforward(&pn, &[port]).await.unwrap();
                    let mut upstream_conn = forwarder
                        .take_stream(port)
                        .context("port not found in forwarder")
                        .unwrap();

                    if let Err(_) =
                        tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await
                    {
                        info!("Could not get stream!");
                        return;
                    }
                    drop(upstream_conn);
                    if let Err(_) = forwarder.join().await {
                        info!("Connection dropped!");
                    }
                });
                Ok(())
            });
    if let Err(e) = server.await {
        exit!(e, "Forwarder server error");
    }
    Ok(())
}
