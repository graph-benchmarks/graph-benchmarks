use std::collections::HashMap;

use anyhow::Result;
use common::{command::command_print, provider::PlatformInfo};

pub async fn join_all_nodes(connect_args: &PlatformInfo, verbose: bool) -> Result<()> {
    command_print(
        "ansible-playbook",
        &[
            "k3s-agent.yaml",
            "--private-key",
            &connect_args.ssh_key,
            "-i",
            "inventory/worker-hosts.yaml",
        ],
        verbose,
        [
            &format!("Rejoining all nodes to cluster"),
            &format!("Could not join all nodes to cluster"),
            &format!("All nodes have joined the cluster"),
        ],
        "k3s",
        HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]),
    )
    .await
}

pub async fn clear_dirs(connect_args: &PlatformInfo, verbose: bool) -> Result<()> {
    command_print(
        "ansible-playbook",
        &[
            "clear-visualizations.yaml",
            "--private-key",
            &connect_args.ssh_key,
            "-i",
            "inventory/master-hosts.yaml",
        ],
        verbose,
        [
            &format!("Clearing visualization folder"),
            &format!("Could not clear visualization folder"),
            &format!("Cleared visualization folder"),
        ],
        "k3s",
        HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]),
    )
    .await
}

pub async fn copy_datasets(
    datasets: &[String],
    connect_args: &PlatformInfo,
    username: &str,
    verbose: bool,
) -> Result<()> {
    for dataset in datasets {
        command_print(
            "rsync",
            &[
                "--blocking-io",
                "-av",
                "--progress",
                "--stats",
                "-rsh",
                "-e",
                &format!(
                    "ssh -i {} -o StrictHostKeyChecking=no",
                    connect_args.ssh_key
                ),
                &format!("../datasets/{dataset}/"),
                &format!(
                    "{username}@{}:/cluster-data/bench-storage/",
                    connect_args.master_ip
                ),
            ],
            verbose,
            [
                &format!("Copying dataset {dataset}"),
                &format!("Could not copy dataset {dataset}"),
                &format!("Copied dataset {dataset}"),
            ],
            "k3s",
            HashMap::<&str, &str>::from([("KUBECONFIG", "kube-config")]),
        )
        .await?;
    }

    Ok(())
}

pub async fn copy_generated_graphs(verbose: bool, connect_args: &PlatformInfo) -> Result<()> {
    command_print(
        "ansible-playbook",
        &[
            "copy-graphs.yaml",
            "--private-key",
            &connect_args.ssh_key,
            "-i",
            "inventory/master-hosts.yaml",
        ],
        verbose,
        [
            &format!("Copying generated graphs"),
            &format!("Could not copy generated graphs"),
            &format!("Copied generated graphs"),
        ],
        "k3s",
        HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]),
    )
    .await
}

pub async fn remove_graph_platform(
    driver: &str,
    connect_args: &PlatformInfo,
    extra_vars: Vec<String>,
    verbose: bool,
) -> Result<()> {
    let mut args = vec![
        "remove.yaml",
        "--private-key",
        &connect_args.ssh_key,
        "-i",
        "../../k3s/inventory/master-hosts.yaml",
    ];

    let extra_vars_str = extra_vars.join(" ");
    if extra_vars.len() > 0 {
        args.push("--extra-vars");
        args.push(&extra_vars_str);
    }

    command_print(
        "ansible-playbook",
        &args,
        verbose,
        [
            &format!("Removing driver {driver}"),
            &format!("Could not remove {driver}"),
            &format!("Removed {driver}"),
        ],
        &format!("drivers/{}", driver),
        HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]),
    )
    .await
}
