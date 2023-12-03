use std::net::IpAddr;

use anyhow::Result;

use crate::config::SetupArgs;

pub const SETUP: [&str; 3] = [
    "Spinning up platform resources",
    "Could not setup platform resources",
    "Platform resources up",
];

pub const DESTROY: [&str; 3] = [
    "Tearing down platform resources",
    "Could not destroy platform resources",
    "Destroyed platform resources",
];

#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub master_ip: IpAddr,
    pub worker_ips: Vec<IpAddr>,
    pub ssh_key: String,
}

#[async_trait::async_trait]
pub trait Platform {
    /// Perform any activities before creating the resources, such as generating configurations
    /// getting dependencies ready, etc.
    async fn pre_setup(&self, setup_args: &SetupArgs, verbose: bool) -> Result<()>;
    /// Create the platform resources
    async fn setup(&self, setup_args: &SetupArgs, verbose: bool) -> Result<()>;
    /// Get the information necessary to connect to the resources, such as IPs and the ssh key
    async fn platform_info(&self, setup_args: &SetupArgs, verbose: bool) -> Result<PlatformInfo>;
    /// Destroy the created resources
    async fn destroy(&self, setup_args: &SetupArgs, verbose: bool) -> Result<()>;
    /// Name of the platform provider
    fn name(&self) -> String;
}
