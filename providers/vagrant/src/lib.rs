use std::net::{IpAddr, Ipv4Addr};

use anyhow::Result;
use common::{
    command::command_platform,
    config::{self, SetupArgs},
    exit,
    provider::*,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Root {
    pub network: Network,
    pub nodes: Nodes,
    pub software: Software,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Network {
    pub dns_servers: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Nodes {
    pub disk_size: usize,
    pub control: Control,
    pub workers: Workers,
    pub storage_pool_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Control {
    pub cpu: usize,
    pub memory: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Workers {
    pub count: usize,
    pub cpu: usize,
    pub memory: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Software {
    #[serde(rename = "box")]
    pub box_field: String,
}

pub struct Vagrant;

#[async_trait::async_trait]
impl Platform for Vagrant {
    async fn pre_setup(&self, setup_args: &SetupArgs, verbose: bool) -> Result<()> {
        let mut settings: Root = serde_yaml::from_str(
            &fs::read_to_string(format!("platforms/{}/settings.yaml", setup_args.platform)).await?,
        )?;
        if let Some(m) = &setup_args.master_platform {
            if m.contains_key("cpu") {
                settings.nodes.control.cpu = m.get("cpu").unwrap().parse()?;
            }
            if m.contains_key("memory") {
                settings.nodes.control.memory = m.get("memory").unwrap().parse()?;
            }
        }

        if let Some(m) = &setup_args.worker_platform {
            if m.contains_key("cpu") {
                settings.nodes.workers.cpu = m.get("cpu").unwrap().parse()?;
            }
            if m.contains_key("memory") {
                settings.nodes.workers.memory = m.get("memory").unwrap().parse()?;
            }
        }
        settings.nodes.workers.count = setup_args.node_configs.iter().max().unwrap().to_owned() - 1;

        if let Some(args) = &setup_args.platform_args {
            if args.contains_key("storage_pool_path") {
                command_platform(
                    "virsh",
                    &[
                        "pool-define-as",
                        "graph_storage_pool",
                        "--type",
                        "dir",
                        "--target",
                        args.get("storage_pool_path").unwrap(),
                    ],
                    verbose,
                    [
                        "Creating storage pool",
                        "Could not create storage pool",
                        "Storage pool created",
                    ],
                    &setup_args.platform,
                )
                .await?;

                command_platform(
                    "virsh",
                    &["pool-start", "--build", "graph_storage_pool"],
                    verbose,
                    [
                        "Starting storage pool",
                        "Could not create storage pool",
                        "Storage pool started",
                    ],
                    &setup_args.platform,
                )
                .await?;
                settings.nodes.storage_pool_name = "graph_storage_pool".to_owned();
            }
        } else {
            settings.nodes.storage_pool_name = "default".to_owned();
        }

        std::fs::write(
            format!("platforms/{}/settings.yaml", setup_args.platform),
            serde_yaml::to_string(&settings)?,
        )?;

        Ok(())
    }

    async fn setup(&self, setup_args: &config::SetupArgs, verbose: bool) -> Result<()> {
        command_platform("vagrant", &["up"], verbose, SETUP, &setup_args.platform).await
    }

    async fn platform_info(&self, _: &SetupArgs, _: bool) -> Result<PlatformInfo> {
        let conn = virt::connect::Connect::open("qemu:///system")?;
        let mut worker_ips: Vec<IpAddr> = Vec::new();
        let mut master_ip: IpAddr = "0.0.0.0".parse().unwrap();
        let domains = conn.list_all_domains(virt::sys::VIR_CONNECT_LIST_DOMAINS_ACTIVE)?;

        if domains.len() == 0 {
            exit!("Vagrant VMs are not running", "Could not get any VM IPs");
        }

        for d in domains {
            let name = d.get_name()?;
            if !name.starts_with("vagrant-libvirt_graph_") {
                continue;
            }

            let if_addrs =
                d.interface_addresses(virt::sys::VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_LEASE, 0)?;
            if if_addrs.len() == 0 || (if_addrs.len() > 0 && if_addrs[0].addrs.len() == 0) {
                exit!("", "Vagrant VM does not have any network interfaces! you might need to run with sudo!");
            }

            if name == "vagrant-libvirt_graph_master" {
                master_ip = if_addrs[0].addrs[0].addr.clone().parse()?;
            } else {
                worker_ips.push(if_addrs[0].addrs[0].addr.clone().parse()?);
            }
        }

        if worker_ips.len() == 0 || master_ip.eq(&IpAddr::V4(Ipv4Addr::UNSPECIFIED)) {
            exit!(
                "",
                "Could not get any VM IPs, you might need to run as sudo!"
            );
        }

        let home = home::home_dir().unwrap();
        Ok(PlatformInfo {
            worker_ips,
            master_ip,
            ssh_key: home.join(".ssh/id_rsa").to_str().unwrap().to_owned(),
        })
    }

    async fn destroy(&self, setup_args: &config::SetupArgs, verbose: bool) -> Result<()> {
        command_platform(
            "vagrant",
            &["destroy", "-f"],
            verbose,
            DESTROY,
            &setup_args.platform,
        )
        .await?;

        command_platform(
            "virsh",
            &["pool-destroy", "graph_storage_pool"],
            verbose,
            [
                "Destroying storage pool",
                "Could not destroy storage pool",
                "Storage pool destroy",
            ],
            &setup_args.platform,
        )
        .await?;

        command_platform(
            "virsh",
            &["pool-undefine", "graph_storage_pool"],
            verbose,
            [
                "Undefine storage pool",
                "Could not undefine storage pool",
                "Storage pool undefined",
            ],
            &setup_args.platform,
        )
        .await?;
        Ok(())
    }

    fn name(self: &Self) -> String {
        "vagrant".to_owned()
    }
}
