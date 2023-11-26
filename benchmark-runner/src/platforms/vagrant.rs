use std::net::{IpAddr, Ipv4Addr};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{common::command_platform, config::PlatformArgs, exit};

use super::{Platform, PlatformInfo, DESTROY, SETUP};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub network: Network,
    pub nodes: Nodes,
    pub software: Software,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    #[serde(rename = "dns_servers")]
    pub dns_servers: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nodes {
    pub disk_size: usize,
    pub control: Control,
    pub workers: Workers,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Control {
    pub cpu: usize,
    pub memory: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workers {
    pub count: usize,
    pub cpu: usize,
    pub memory: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Software {
    #[serde(rename = "box")]
    pub box_field: String,
}

pub struct Vagrant;

#[async_trait::async_trait]
impl Platform for Vagrant {
    async fn pre_setup(&self, platform_args: &PlatformArgs, _: bool) -> Result<()> {
        let mut settings: Root =
            serde_yaml::from_str(&fs::read_to_string("platform/vagrant/settings.yaml").await?)?;
        if let Some(m) = &platform_args.master_platform_env {
            if m.contains_key("cpu") {
                settings.nodes.control.cpu = m.get("cpu").unwrap().parse()?;
            }
            if m.contains_key("memory") {
                settings.nodes.control.memory = m.get("memory").unwrap().parse()?;
            }
        }

        if let Some(m) = &platform_args.worker_platform_env {
            if m.contains_key("cpu") {
                settings.nodes.workers.cpu = m.get("cpu").unwrap().parse()?;
            }
            if m.contains_key("memory") {
                settings.nodes.workers.memory = m.get("memory").unwrap().parse()?;
            }
        }
        settings.nodes.workers.count =
            platform_args.node_configs.iter().max().unwrap().to_owned() - 1;
        std::fs::write(
            "platform/vagrant/settings.yaml",
            serde_yaml::to_string(&settings)?,
        )?;
        Ok(())
    }

    async fn setup(&self, _: &crate::config::PlatformArgs, verbose: bool) -> Result<()> {
        command_platform("vagrant", &["up"], verbose, SETUP, &self.name()).await
    }

    async fn platform_info(&self, _: &PlatformArgs, _: bool) -> Result<PlatformInfo> {
        let conn = virt::connect::Connect::open("qemu:///session")?;
        let mut worker_ips: Vec<IpAddr> = Vec::new();
        let mut master_ip: IpAddr = "0.0.0.0".parse().unwrap();
        let domains = conn.list_all_domains(virt::sys::VIR_CONNECT_LIST_DOMAINS_ACTIVE)?;

        if domains.len() == 0 {
            exit!("Vagrant VMs are not running", "Could not get any VM IPs");
        }

        for d in domains {
            let name = d.get_name()?;
            if !name.starts_with("vagrant_graph_") {
                continue;
            }

            let if_addrs =
                d.interface_addresses(virt::sys::VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_LEASE, 0)?;
            if if_addrs.len() == 0 || (if_addrs.len() > 0 && if_addrs[0].addrs.len() == 0) {
                exit!("", "Vagrant VM does not have any network interfaces! you might need to run with sudo!");
            }

            if name == "vagrant_graph_master" {
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
        Ok(super::PlatformInfo {
            worker_ips,
            master_ip,
            ssh_key: home.join(".ssh/id_rsa").to_str().unwrap().to_owned(),
        })
    }

    async fn destroy(&self, verbose: bool) -> Result<()> {
        command_platform(
            "vagrant",
            &["destroy", "-f"],
            verbose,
            DESTROY,
            &self.name(),
        )
        .await
    }

    fn name(self: &Self) -> String {
        "vagrant".to_owned()
    }
}
