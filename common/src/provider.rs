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
    async fn pre_setup(self: &Self, setup_args: &SetupArgs, verbose: bool) -> Result<()>;
    async fn setup(self: &Self, setup_args: &SetupArgs, verbose: bool) -> Result<()>;
    async fn platform_info(
        self: &Self,
        setup_args: &SetupArgs,
        verbose: bool,
    ) -> Result<PlatformInfo>;
    async fn destroy(self: &Self, setup_args: &SetupArgs, verbose: bool) -> Result<()>;
    fn name(self: &Self) -> String;
}
