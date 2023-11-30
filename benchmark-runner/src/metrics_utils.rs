use anyhow::Result;
use tonic::Request;

use crate::metrics::{
    performance_metrics_service_client::PerformanceMetricsServiceClient, StartRecordingRequest,
    StopRecordingRequest,
};

pub async fn start_recording(ip: String, pod_ids: Vec<String>, run_id: i32) -> Result<()> {
    let mut client = PerformanceMetricsServiceClient::connect(ip).await?;
    let req = Request::new(StartRecordingRequest {
        pod_ids,
        interval: 100.0,
        run_id,
    });
    client.start_recording(req).await?;
    Ok(())
}

pub async fn stop_recording(ip: String, pod_ids: Vec<String>, run_id: i32) -> Result<()> {
    let mut client = PerformanceMetricsServiceClient::connect(ip).await?;
    let req = Request::new(StopRecordingRequest { pod_ids, run_id });
    client.stop_recording(req).await?;
    Ok(())
}
