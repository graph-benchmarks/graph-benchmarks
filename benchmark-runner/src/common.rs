use std::{
    collections::HashMap,
    process::Stdio,
    time::{Duration, Instant},
};

use anyhow::Result;
use console::{style, StyledObject};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::process::Command;

use crate::exit;

lazy_static::lazy_static! {
    static ref DOTS_STYLE: ProgressStyle = ProgressStyle::with_template("{spinner} {msg} {elapsed_precise}").unwrap().tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    static ref GREEN_TICK: StyledObject<&'static str> = style("✔").green();
}

pub fn progress(msg: &str) -> ProgressBar {
    let w = ProgressBar::new_spinner();
    w.set_style(DOTS_STYLE.clone());
    w.enable_steady_tick(Duration::from_millis(80));
    w.set_message(format!("{}", msg));
    w
}

pub async fn command_platform(
    cmd: &str,
    args: &[&str],
    verbose: bool,
    msgs: [&str; 3],
    platform: &str,
) -> Result<()> {
    command(
        cmd,
        args,
        verbose,
        msgs,
        &format!("platform/{platform}"),
        HashMap::<String, String>::new(),
    )
    .await
}

pub async fn command(
    cmd: &str,
    args: &[&str],
    verbose: bool,
    msgs: [&str; 3],
    dir: &str,
    env: HashMap<
        impl AsRef<str> + std::convert::AsRef<std::ffi::OsStr>,
        impl AsRef<str> + std::convert::AsRef<std::ffi::OsStr>,
    >,
) -> Result<()> {
    let mut cmd = Command::new(cmd);
    let mut _cmd = cmd.current_dir(dir).args(args);

    env.iter().for_each(|(k, v)| {
        _cmd.env(k, v);
    });

    let mut pb = None;
    if !verbose {
        _cmd = _cmd.stdout(Stdio::piped());
        pb = Some(progress(msgs[0]));
    }

    let start_time = Instant::now();
    let cmd_spawn = _cmd.spawn()?;
    let output = cmd_spawn.wait_with_output().await?;
    let dur = start_time.elapsed();
    if !output.status.success() {
        exit!(String::from_utf8(output.stdout)?, "{}", msgs[1]);
    }

    finish_progress(msgs[2], dir, dur, pb);
    Ok(())
}

pub fn finish_progress(
    status_message: &str,
    context: &str,
    dur: Duration,
    pb: Option<ProgressBar>,
) {
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    println!(
        "{} {} ({}) took, {}.{:02}s",
        GREEN_TICK.to_string(),
        status_message,
        context,
        dur.as_secs(),
        dur.as_millis() - (dur.as_secs() as u128 * 1000)
    );
}
