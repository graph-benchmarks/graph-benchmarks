use std::net::IpAddr;

use anyhow::Result;

use crate::config::PlatformArgs;

mod terraform;
mod vagrant;

pub const PLATFORMS: &[&'static dyn Platform] = &[&terraform::Terraform, &vagrant::Vagrant];

const SETUP: [&str; 3] = [
    "Spinning up platform resources",
    "Could not setup platform resources",
    "Platform resources up",
];

const DESTROY: [&str; 3] = [
    "Tearing down platform resources",
    "Could not destroy platform resources",
    "Destroyed platform resources",
];

pub struct PlatformInfo {
    pub master_ip: IpAddr,
    pub worker_ips: Vec<IpAddr>,
    pub ssh_key: String,
}

#[async_trait::async_trait]
pub trait Platform {
    async fn pre_setup(self: &Self, platform_args: &PlatformArgs, verbose: bool) -> Result<()>;
    async fn setup(self: &Self, platform_args: &PlatformArgs, verbose: bool) -> Result<()>;
    async fn platform_info(
        self: &Self,
        platform_args: &PlatformArgs,
        verbose: bool,
    ) -> Result<PlatformInfo>;
    async fn destroy(self: &Self, verbose: bool) -> Result<()>;
    fn name(self: &Self) -> String;
}
