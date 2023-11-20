use std::{
    collections::HashMap,
    fs::{self, remove_file, set_permissions, Permissions},
    net::IpAddr,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::Result;
use hcl::{Body, Expression};
use indicatif::{MultiProgress, ProgressBar};
use log::info;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{args::*, config::*, exit, terraform_output, DOTS_STYLE, GREEN_TICK, commands::{platform::{run_terraform_command, RunCommand, run_vagrant_command}, vagrant_settings}};

use super::platform::get_vm_map;

#[derive(Debug, Serialize, Deserialize)]
pub struct Item<'a> {
    pub hosts: HashMap<String, ()>,
    #[serde(borrow)]
    pub vars: HashMap<&'a str, String>,
}

fn setup_master_node(connect_args: &PlatformConnectInfo, verbose: bool) -> Result<()> {
    let master_hosts_file = PathBuf::from("k3s/inventory/master-hosts.yaml");
    if master_hosts_file.exists() {
        remove_file(master_hosts_file.as_path())?;
    }

    let hosts = HashMap::from([(connect_args.ips[0].to_string(), ())]);
    let vars = HashMap::from([(
        "ansible_user",
        connect_args
            .host_username
            .clone()
            .unwrap_or("root".to_owned()),
    )]);
    let master_hosts = HashMap::from([("master", Item { hosts, vars })]);

    fs::write(master_hosts_file, serde_yaml::to_string(&master_hosts)?)?;

    let mut cmd = Command::new("ansible-playbook");
    let mut setup_master = cmd
        .current_dir("k3s")
        .env("ANSIBLE_HOST_KEY_CHECKING", "False")
        .arg("master.yaml")
        .arg("--private-key")
        .arg(&connect_args.private_key_file)
        .arg("-i")
        .arg("inventory/master-hosts.yaml");

    if verbose {
        setup_master.env("DEBUG_ANSIBLE", "1");
    }

    if !verbose {
        setup_master = setup_master.stdout(Stdio::piped());
    }
    let setup_master = setup_master.spawn()?;

    let mut m = None;
    let mut master_p = None;
    if !verbose {
        m = Some(MultiProgress::new());
        master_p = Some(m.as_ref().unwrap().add(ProgressBar::new_spinner()));
        let w = master_p.as_ref().unwrap();
        w.set_style(DOTS_STYLE.clone());
        w.enable_steady_tick(Duration::from_millis(80));
        w.set_message("Setting up master node");
    }

    let output = setup_master.wait_with_output()?;
    if !output.status.success() {
        exit!(
            String::from_utf8(output.stdout)?,
            "Could not setup master node"
        );
    }

    if master_p.is_some() {
        master_p.as_ref().unwrap().finish_and_clear();
        m.as_ref().unwrap().clear()?;
    }
    println!("{} {}", GREEN_TICK.to_string(), "Master node ready");
    Ok(())
}

fn setup_worker_node(connect_args: &PlatformConnectInfo, verbose: bool) -> Result<()> {
    let mut cmd = Command::new("ansible-playbook");
    let mut setup_worker = cmd
        .current_dir("k3s")
        .env("ANSIBLE_HOST_KEY_CHECKING", "False")
        .arg("worker.yaml")
        .arg("--private-key")
        .arg(&connect_args.private_key_file)
        .arg("-i")
        .arg("inventory/worker-hosts.yaml");

    if !verbose {
        setup_worker = setup_worker.stdout(Stdio::piped());
    } else {
        setup_worker.env("DEBUG_ANSIBLE", "1");
    }

    let setup_master = setup_worker.spawn()?;

    let mut m = None;
    let mut worker_p = None;
    if !verbose {
        m = Some(MultiProgress::new());
        worker_p = Some(m.as_ref().unwrap().add(ProgressBar::new_spinner()));
        let w = worker_p.as_ref().unwrap();
        w.set_style(DOTS_STYLE.clone());
        w.enable_steady_tick(Duration::from_millis(80));
        w.set_message("Setting up worker node");
    }

    let mut hosts = HashMap::new();
    connect_args.ips.iter().skip(1).for_each(|x| {
        hosts.insert(x.to_string(), ());
    });
    let vars = HashMap::from([
        ("master", connect_args.ips[0].to_string()),
        ("node_token", fs::read_to_string("k3s/node-token")?.trim().to_owned()),
        (
            "ansible_user",
            connect_args
                .host_username
                .clone()
                .unwrap_or("root".to_owned()),
        ),
    ]);
    let worker_hosts = HashMap::from([("workers", Item { hosts, vars })]);

    let worker_hosts_file = PathBuf::from("k3s/inventory/worker-hosts.yaml");
    if worker_hosts_file.exists() {
        remove_file(worker_hosts_file.as_path())?;
    }

    fs::write(
        worker_hosts_file.as_path(),
        serde_yaml::to_string(&worker_hosts)?,
    )?;
    let output = setup_master.wait_with_output()?;
    if !output.status.success() {
        exit!(
            String::from_utf8(output.stdout)?,
            "Could not setup worker node"
        );
    }

    if worker_p.is_some() {
        worker_p.as_ref().unwrap().finish_and_clear();
        m.as_ref().unwrap().clear()?;
    }
    println!("{} {}", GREEN_TICK.to_string(), "Worker nodes ready");
    Ok(())
}

pub fn setup_platform(
    args: &PlatformArgs,
    cli: &SetupArgs,
    verbose: bool,
) -> Result<PlatformConnectInfo> {
    if args.platform == "vagrant" {
        let mut settings: vagrant_settings::Root = serde_yaml::from_str(&fs::read_to_string("platform/vagrant/settings.yaml")?)?;
        if let Some(m) = &args.master_platform_env {
            if m.contains_key("cpu") {
                settings.nodes.control.cpu = m.get("cpu").unwrap().parse()?;
            }
            if m.contains_key("memory") {
                settings.nodes.control.memory = m.get("cpu").unwrap().parse()?;
            }
        }

        if let Some(m) = &args.worker_platform_env {
            if m.contains_key("cpu") {
                settings.nodes.workers.cpu = m.get("cpu").unwrap().parse()?;
            }
            if m.contains_key("memory") {
                settings.nodes.workers.memory = m.get("cpu").unwrap().parse()?;
            }
        }
        settings.nodes.workers.count = args.node_configs.iter().max().unwrap().to_owned() - 1;
        std::fs::write("platform/vagrant/settings.yaml", serde_yaml::to_string(&settings)?)?;
    }

    if !cli.only_platform_outputs {
        let mut cmd = RunCommand {
            args: &["apply", "--auto-approve"],
            ongoing: "Spinning up platform resources",
            success: "Platform resources up",
            failure: "Could not setup platform resources"
        };

        match args.platform.as_str() {
            "vagrant" => {
                cmd.args = &["up"];
                run_vagrant_command(cmd, verbose)?
            },
            _ => run_terraform_command(&args, cmd, verbose)?
        }
    } else {
        info!("Skipping platform setup");
    }

    if args.platform == "vagrant" {
        let worker_name_match = Regex::new(r"vagrant_graph_node[0-9]+")?;
        let conn = virt::connect::Connect::open("qemu:///session")?;
        let mut master_ip = String::new();
        let mut worker_ips = Vec::new();
        let domains = conn.list_all_domains(virt::sys::VIR_CONNECT_LIST_DOMAINS_ACTIVE)?;
        for d in domains {
            let name = d.get_name()?;
            if !name.starts_with("vagrant_") {
                continue;
            }

            let if_addrs = d.interface_addresses(virt::sys::VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_LEASE, 0)?;
            if if_addrs.len() == 0 || (if_addrs.len() > 0 && if_addrs[0].addrs.len() == 0) {
                exit!("", "Vagrant VM does not have any network interfaces! you might need to run with sudo!");
            }

            if name.eq("vagrant_graph_master") {
                master_ip = if_addrs[0].addrs[0].addr.clone();
            }

            if worker_name_match.is_match_at(&name, 0) {
                worker_ips.push(if_addrs[0].addrs[0].addr.clone());
            }
        }

        if worker_ips.len() == 0 {
            exit!("", "Could not get any VM IPs, you might need to run as sudo!");
        }

        let mut args_copy = args.clone();
        if args_copy.master_platform_env.is_none() {
            args_copy.master_platform_env = Some(HashMap::from([("public_ip".to_owned(), master_ip)]));
        } else {
            args_copy.worker_platform_env.as_mut().unwrap().insert("public_ip".to_owned(), master_ip);
        }

        if args_copy.worker_platform_env.is_none() {
            args_copy.worker_platform_env = Some(HashMap::new());
        }

        let map = args_copy.worker_platform_env.as_mut().unwrap();
        for w in 0..worker_ips.len() {
            map.insert(format!("worker-{}public_ip", w), worker_ips[w].to_owned());
        }

        let vars = get_vm_map(&args_copy)?;
        let home = home::home_dir().unwrap();
        let vars = Body::builder()
            .add_attribute(("key_data", Expression::String(std::fs::read_to_string(home.join(".ssh/id_rsa"))?)))
            .add_attribute(("vm_map", hcl::Value::Object(vars.get("vm_map").unwrap().clone()))).build();
        fs::write("platform/vagrant/vars.tfvars", hcl::to_string(&vars)?)?;

        let mut cmd = Command::new("terraform");
        cmd
            .current_dir("platform/vagrant")
            .arg("refresh")
            .arg("-var-file=vars.tfvars")
            .output()?;
    }

    let mut cmd = Command::new("terraform");
    let output = cmd
        .current_dir(format!("platform/{}", args.platform))
        .arg("output")
        .arg("-json")
        .output()?;
    let json = String::from_utf8(output.stdout)?;
    let mut connect_info: terraform_output::Root = serde_json::from_str(&json)?;

    let private_key_file = format!("platform/{}/key.pem", args.platform);
    std::fs::write(&private_key_file, connect_info.key_data.value)?;
    set_permissions(&private_key_file, Permissions::from_mode(0o600))?;

    let ips: Result<Vec<IpAddr>, _> = connect_info
        .linux_virtual_machine_ips
        .value
        .drain(..)
        .map(|x| x.first().unwrap().parse::<IpAddr>())
        .collect();
    Ok(PlatformConnectInfo {
        private_key_file: format!("../{}", private_key_file),
        ips: ips?,
        host_username: args.host_username.clone(),
    })
}

pub fn setup(args: &SetupArgs, cli: &Cli) -> Result<()> {
    let config = parse_config(&cli.file)?;
    let connect_args = match &config.setup {
        Setup::PreConfiguredPlatform(p) => p.clone(),
        Setup::Platform(platform_args) => match setup_platform(&platform_args, &args, cli.verbose) {
            Ok(p) => p,
            Err(err) => exit!(err, "Could not setup platform"),
        },
    };
    info!("{connect_args:#?}");

    if connect_args.ips.len() < 2 {
        exit!(
            "Check platform setup output",
            "Need at least two nodes for kubernetes, only got {}",
            connect_args.ips.len()
        );
    }

    setup_master_node(&connect_args, cli.verbose)?;
    setup_worker_node(&connect_args, cli.verbose)
}
