// TODO:
// 400 from trigger_pipeline should provide feedback to a user
// - Create a service-user that has an exclusive right to vote on a new "Verified" label
// - More errors reported to the user
use tokio::sync::broadcast;

mod change_tracker;
mod cli;
mod config;
mod gerrit_ssh_command;
mod gerrit_stream_events;
mod git;
mod gitlab;
mod ssh;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting");

    let cli = cli::cli();
    let config = config::load_from(cli.config_path);

    let (gerrit_events_tx, gerrit_events_rx) = broadcast::channel(100);

    change_tracker::run(change_tracker::Args {
        config: config.clone(),
        gerrit_events_rx,
    });

    gerrit_stream_events::run(gerrit_stream_events::Args {
        config: config.clone(),
        gerrit_events_tx,
    });

    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for SIGINT");

    log::info!("SIGINT received, closing.");
    Ok(())
}
