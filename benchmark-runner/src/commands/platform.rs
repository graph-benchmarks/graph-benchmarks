use std::{fs, process::{Command, Stdio}, time::Duration};

use anyhow::Result;
use indicatif::ProgressBar;
use regex::Regex;

use crate::{config::PlatformArgs, GREEN_TICK, DOTS_STYLE, exit};

pub fn get_vm_map(args: &PlatformArgs) -> Result<hcl::Map<String, hcl::Map<String, hcl::Value>>> {
    let max_nodes = args.node_configs.iter().max().unwrap();
    let mut vm_map: hcl::Map<String, hcl::Value> = hcl::Map::new();
    let worker_name_match = Regex::new(r"worker-[0-9]+")?;
    for i in 0..*max_nodes {
        let mut m: hcl::Map<String, hcl::Value> = hcl::Map::new();
        if i == 0 {
            m.insert("name".to_owned(), "master".into());
            if args.master_platform_env.is_some() {
                args.master_platform_env
                    .as_ref()
                    .unwrap()
                    .iter()
                    .for_each(|(k, v)| {
                        m.insert(k.to_owned(), v.to_owned().into());
                    });
            }
        } else {
            m.insert("name".to_owned(), format!("worker-{}", i).into());
            if args.worker_platform_env.is_some() {
                args.worker_platform_env
                    .as_ref()
                    .unwrap()
                    .iter()
                    .for_each(|(k, v)| {
                        let worker_match = worker_name_match.is_match_at(k.as_str(), 0);
                        if !worker_match || (worker_match && worker_name_match.find(k.as_str()).unwrap().as_str().eq(&format!("worker-{}", i - 1))) {
                            m.insert(worker_name_match.replace(k.as_str(), "").to_string(), v.to_owned().into());
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

pub struct RunCommand<'a> {
    pub args: &'a [&'a str],
    pub ongoing: &'a str,
    pub success: &'a str,
    pub failure: &'a str
}

pub fn run_terraform_command(args: &PlatformArgs, config: RunCommand, verbose: bool) -> Result<()> {
    let vars = get_vm_map(&args)?;
    fs::write(
        format!("platform/{}/vars.tfvars", args.platform),
        hcl::to_string(&vars)?,
    )?;

    let mut cmd = Command::new("terraform");
    let mut terraform_setup = cmd.current_dir(format!("platform/{}", args.platform));
    config.args.iter().for_each(|x| { terraform_setup.arg(x); });
    terraform_setup.arg("-var-file=vars.tfvars");

    let mut pb = None;
    if !verbose {
        terraform_setup = terraform_setup.stdout(Stdio::piped());
        pb = Some(ProgressBar::new_spinner());
        let w = pb.as_ref().unwrap();
        w.set_style(DOTS_STYLE.clone());
        w.enable_steady_tick(Duration::from_millis(80));
        w.set_message(format!(
            "{} ({})",
            config.ongoing,
            args.platform
        ));
    }

    let terraform_setup = terraform_setup.spawn()?;
    let output = terraform_setup.wait_with_output()?;
    if !output.status.success() {
        exit!(
            String::from_utf8(output.stdout)?,
            "{}", config.failure
        );
    }

    let msg = format!("{} ({})", config.success, args.platform);
    if !verbose {
        if let Some(pb) = pb {
            pb.finish_and_clear();
        }
    }
    println!("{} {}", GREEN_TICK.to_string(), msg);
    Ok(())
}

pub fn run_vagrant_command(config: RunCommand, verbose: bool) -> Result<()> {
    let mut cmd = Command::new("vagrant");
    let mut vagrant_setup = cmd.current_dir("platform/vagrant");
    config.args.iter().for_each(|x| { vagrant_setup.arg(x); });

    let mut pb = None;
    if !verbose {
        vagrant_setup = vagrant_setup.stdout(Stdio::piped());
        pb = Some(ProgressBar::new_spinner());
        let w = pb.as_ref().unwrap();
        w.set_style(DOTS_STYLE.clone());
        w.enable_steady_tick(Duration::from_millis(80));
        w.set_message(format!("{} (vagrant)", config.ongoing));
    }

    let vagrant_setup = vagrant_setup.spawn()?;
    let output = vagrant_setup.wait_with_output()?;
    if !output.status.success() {
        exit!(String::from_utf8(output.stdout)?, "{}", config.failure);
    }

    let msg = format!("{} (vagrant)", config.success);
    if !verbose {
        if let Some(pb) = pb {
            pb.finish_and_clear();
        }
    }
    println!("{} {}", GREEN_TICK.to_string(), msg);
    Ok(())
}