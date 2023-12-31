use std::{fs::Permissions, net::IpAddr, os::unix::fs::PermissionsExt, process::Command};

use anyhow::Result;
use common::{command::command_platform, config::SetupArgs, provider::*};
use regex::Regex;
use tokio::fs::{self, set_permissions};

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    #[serde(rename = "key_data")]
    pub key_data: KeyData,
    #[serde(rename = "ips")]
    pub ips: LinuxVirtualMachineIps,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyData {
    pub sensitive: bool,
    #[serde(rename = "type")]
    pub type_field: String,
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxVirtualMachineIps {
    pub sensitive: bool,
    #[serde(rename = "type")]
    pub type_field: (String, Vec<(String, Vec<String>)>),
    pub value: Vec<Vec<String>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxVirtualMachineNames {
    pub sensitive: bool,
    #[serde(rename = "type")]
    pub type_field: (String, Vec<(String, Vec<String>)>),
    pub value: Vec<Vec<String>>,
}

pub struct Terraform;

#[async_trait::async_trait]
impl Platform for Terraform {
    async fn pre_setup(&self, _: &SetupArgs, _: bool) -> Result<()> {
        Ok(())
    }

    async fn setup(&self, setup_args: &SetupArgs, verbose: bool) -> Result<()> {
        let vars = get_vm_map(&setup_args)?;
        fs::write(
            format!("platforms/{}/vars.tfvars", setup_args.platform),
            hcl::to_string(&vars)?,
        )
        .await?;

        command_platform(
            "terraform",
            &["apply", "--auto-approve", "-var-file=vars.tfvars"],
            verbose,
            SETUP,
            &setup_args.platform,
        )
        .await
    }

    async fn platform_info(&self, setup_args: &SetupArgs, _: bool) -> Result<PlatformInfo> {
        let mut cmd = Command::new("terraform");
        let output = cmd
            .current_dir(format!("platforms/{}", setup_args.platform))
            .args(["output", "-json"])
            .output()?;
        let json = String::from_utf8(output.stdout)?;
        let mut connect_info: Root = serde_json::from_str(&json)?;

        let private_key_file = format!("platforms/{}/key.pem", setup_args.platform);
        std::fs::write(&private_key_file, connect_info.key_data.value)?;
        set_permissions(&private_key_file, Permissions::from_mode(0o600)).await?;

        let ips: Result<Vec<IpAddr>, _> = connect_info
            .ips
            .value
            .drain(..)
            .map(|x| x.first().unwrap().parse::<IpAddr>())
            .collect();

        let ips = ips?;
        Ok(PlatformInfo {
            master_ip: ips[0],
            worker_ips: ips.into_iter().skip(1).collect(),
            ssh_key: private_key_file,
        })
    }

    async fn destroy(&self, setup_args: &SetupArgs, verbose: bool) -> Result<()> {
        command_platform(
            "terraform",
            &["destroy", "--auto-approve", "-var-file=vars.tfvars"],
            verbose,
            DESTROY,
            &setup_args.platform,
        )
        .await
    }

    fn name(self: &Self) -> String {
        "terraform".to_owned()
    }
}

fn get_vm_map(args: &SetupArgs) -> Result<hcl::Map<String, hcl::Map<String, hcl::Value>>> {
    let max_nodes = args.node_configs.iter().max().unwrap();
    let mut vm_map: hcl::Map<String, hcl::Value> = hcl::Map::new();
    let worker_name_match = Regex::new(r"worker-[0-9]+")?;
    for i in 0..*max_nodes {
        let mut m: hcl::Map<String, hcl::Value> = hcl::Map::new();
        if i == 0 {
            m.insert("name".to_owned(), "master".into());
            if args.master_platform.is_some() {
                args.master_platform
                    .as_ref()
                    .unwrap()
                    .iter()
                    .for_each(|(k, v)| {
                        m.insert(k.to_owned(), v.to_owned().into());
                    });
            }
        } else {
            m.insert("name".to_owned(), format!("worker-{}", i).into());
            if args.worker_platform.is_some() {
                args.worker_platform
                    .as_ref()
                    .unwrap()
                    .iter()
                    .for_each(|(k, v)| {
                        let worker_match = worker_name_match.is_match_at(k.as_str(), 0);
                        if !worker_match
                            || (worker_match
                                && worker_name_match
                                    .find(k.as_str())
                                    .unwrap()
                                    .as_str()
                                    .eq(&format!("worker-{}", i - 1)))
                        {
                            m.insert(
                                worker_name_match.replace(k.as_str(), "").to_string(),
                                v.to_owned().into(),
                            );
                        }
                    });
            }
        }

        vm_map.insert(
            format!("node-{}", (i + 1).to_string()),
            hcl::Value::Object(m),
        );
    }
    let mut vars = hcl::Map::new();
    vars.insert("vm_map".into(), vm_map);
    Ok(vars)
}
