use anyhow::Result;
use tonic::Request;
use tracing::info;

use crate::rpc::{metrics_collector_client::MetricsCollectorClient, Start, Stop};

pub async fn start_recording(ip: String, pod_ids: Vec<String>, run_id: i32) -> Result<()> {
    let mut client = MetricsCollectorClient::connect(ip).await?;
    let req = Request::new(Start {
        pod_ids,
        interval: 1000.0,
        run_id: run_id.into(),
    });
    info!("{:#?}", client.start_recording(req).await?);
    Ok(())
}

pub async fn stop_recording(ip: String, pod_ids: Vec<String>, run_id: i32) -> Result<()> {
    let mut client = MetricsCollectorClient::connect(ip).await?;
    let req = Request::new(Stop {
        pod_ids,
        run_id: run_id.into(),
    });
    info!("{:#?}", client.stop_recording(req).await?);
    Ok(())
}
