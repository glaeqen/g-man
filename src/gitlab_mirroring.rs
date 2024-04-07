use tokio::sync::broadcast;

use crate::{
    send_gerrit_review,
    config,
    gerrit_stream_events::{
        self,
        models::{Change, Patchset},
        Event,
    }, helpers::LoggingEnricher, git, gerrit_ssh_command,
};

pub struct Args {
    pub config: config::Config,
    pub gerrit_events_rx: broadcast::Receiver<gerrit_stream_events::Event>,
}

pub fn run(args: Args) {
    tokio::spawn(run_impl(args));
}

async fn run_impl(mut args: Args) {
    loop {
        let gerrit_event = args
            .gerrit_events_rx
            .recv()
            .await
            .expect("sender closed or lagging, critical failure");
        match gerrit_event {
            Event::ChangeMerged { patchset, change } => mirror_branch(&args.config, patchset, change).await,
            _ => {}
        }
    }
}

async fn mirror_branch(config: &config::Config, patchset: Patchset, change: Change) {
    let id = LoggingEnricher::new(&patchset, &change);
    // TODO: Turn into a map in the config, instead of hard
    if change.branch != "master" {
        log::info!("{id} Only master branch is mirrored, skipping ref: {}", &change.branch);
        return;
    }

    let git = git::Git::new(config.clone());

    let gerrit_reviewer = gerrit_ssh_command::GerritSshCommand::new(config.clone());
    let branch = &change.branch;
    match git.mirror_in_gitlab(&change).await {
        Ok(_) => {
            log::info!("{id} Branch ({branch}) successfully mirrored in GitLab.");
            send_gerrit_review!(
                gerrit_reviewer,
                change,
                patchset,
                "Branch ({}) successfully mirrored in GitLab.",
                branch
            );
        }
        Err(e) => {
            log::error!(
                "{id} Error when trying to mirror the branch ({}) in GitLab: {e:?}",
                branch
            );
            send_gerrit_review!(
                    gerrit_reviewer,
                    change,
                    patchset,
                    "Fatal: Error occured when trying to mirror the branch ({}) in GitLab. Check the logs.",
                    branch
                );
        }
    }
}
