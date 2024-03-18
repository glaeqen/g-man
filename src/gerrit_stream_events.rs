pub mod models;

pub use models::Event;

use std::{process::Stdio, time::Duration};

use serde_json::error::Category;
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::ChildStdout,
    sync::broadcast,
};

use crate::config;

#[derive(Clone, Debug)]
pub struct Args {
    pub config: config::Config,
    pub gerrit_events_tx: broadcast::Sender<Event>,
}

pub fn run(args: Args) {
    tokio::spawn(run_impl(args));
}

async fn run_impl(args: Args) {
    let mut command = args.config.gerrit.ssh.command();
    command
        .args(["gerrit", "stream-events"])
        .stdout(Stdio::piped());

    loop {
        log::debug!("Running a command: {:#?}", command);
        let mut child = command.spawn().expect("failed to spawn an ssh command");
        let stdout = child
            .stdout
            .take()
            .expect("child does not have an stdout handle");

        let stdout_line_reader = BufReader::new(stdout).lines();

        tokio::select! {
            process_result = child.wait() => {
                log::error!("Process unexpectedly ended: {:?}", process_result);
            },
            stdout_processing_result = process_lines(stdout_line_reader, &args.gerrit_events_tx) => {
                log::error!(
                    "Stdout processing unexpectedly ended: {:?}",
                    stdout_processing_result
                );
            }
        };

        child
            .kill()
            .await
            .unwrap_or_else(|e| log::error!("Failed to kill the process: {e}"));

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn process_lines(
    mut stdout_line_reader: Lines<BufReader<ChildStdout>>,
    gerrit_events_tx: &broadcast::Sender<Event>,
) -> anyhow::Result<()> {
    log::debug!("Started processing the stdout");
    while let Some(line) = stdout_line_reader.next_line().await? {
        log::trace!("{}", line);
        let event: Event = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(e) if e.classify() == Category::Data => {
                // Most event variants are not implemented, that (should be) fine
                log::trace!("Event deserialization failure: {}", e);
                continue;
            }
            Err(e) => {
                log::error!("Unexpected event deserialization failure: {}", e);
                continue;
            }
        };
        log::debug!("{:#?}", event);
        gerrit_events_tx
            .send(event)
            .expect("all receiving parties closed, critical failure");
    }
    Err(anyhow::anyhow!("Stream ended on stdout?"))
}
