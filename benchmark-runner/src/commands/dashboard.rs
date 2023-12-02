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

use crate::args::Cli;

pub async fn access(_: &Cli) -> Result<()> {
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
    println!("\nListening on 8443!");

    let pod_name = p.items[0].metadata.name.as_ref().unwrap().clone();
    let server = TcpListenerStream::new(TcpListener::bind("0.0.0.0:8443").await.unwrap())
        .take_until(tokio::signal::ctrl_c())
        .try_for_each(|mut client_conn| async {
            let pods = pods.clone();
            let pn = pod_name.clone();
            tokio::spawn(async move {
                let mut forwarder = pods.portforward(&pn, &[8443]).await.unwrap();
                let mut upstream_conn = forwarder
                    .take_stream(8443)
                    .context("port not found in forwarder")
                    .unwrap();
                tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn)
                    .await
                    .unwrap();
                drop(upstream_conn);
                forwarder.join().await.unwrap();
            });
            Ok(())
        });
    if let Err(e) = server.await {
        exit!(e, "server error");
    }
    Ok(())
}
