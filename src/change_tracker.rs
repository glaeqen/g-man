use std::time::Duration;

use tokio::sync::broadcast;

use crate::{
    config,
    gerrit_ssh_command::{self, Review, Verified},
    gerrit_stream_events::{
        self,
        models::{Change, Patchset},
    },
    git,
    gitlab::{self, PipelineQueryResult, PipelineStatus},
};

// TODO: Replace with config
const TIMEOUT_PER_OBSERVATION: u64 = 3600;

pub struct Args {
    pub config: config::Config,
    pub gerrit_events_rx: broadcast::Receiver<gerrit_stream_events::Event>,
}

pub fn run(args: Args) {
    tokio::spawn(run_impl(args));
}

async fn run_impl(mut args: Args) {
    let change_tracker = ChangeTracker::new(args.config);
    loop {
        let gerrit_event = args
            .gerrit_events_rx
            .recv()
            .await
            .expect("sender closed or lagging, critical failure");
        log::trace!("Received an event: {:?}", gerrit_event);
        change_tracker.process(gerrit_event);
    }
}

#[derive(Clone)]
struct ChangeTracker {
    config: config::Config,
}

fn code_change_in_patchset(patchset: &Patchset) -> bool {
    match patchset.kind {
        gerrit_stream_events::models::PatchsetKind::NoCodeChange => false,
        gerrit_stream_events::models::PatchsetKind::NoChange => false,
        _ => true,
    }
}

impl ChangeTracker {
    fn process(&self, event: gerrit_stream_events::Event) {
        let clone = self.clone();
        tokio::spawn(async {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(TIMEOUT_PER_OBSERVATION)) => {
                    // TODO: Enrich a log a little
                    log::error!("Tracker timed out!");
                }
                _ = clone.process_impl(event) => {}
            };
        });
    }

    // Any gerrit event will spawn a task based on this function
    // Because of that, this function should quite aggressively
    // terminate as soon as possible when it realizes it is outdated.
    async fn process_impl(self, event: gerrit_stream_events::Event) {
        use gerrit_stream_events::Event;
        match event {
            Event::PatchsetCreated { patchset, change }
                if !change.wip && !change.private && code_change_in_patchset(&patchset) =>
            {
                self.new_pipeline(patchset, change).await
            }
            Event::ChangeRestored { patchset, change }
                if !change.wip && !change.private && code_change_in_patchset(&patchset) =>
            {
                self.new_pipeline(patchset, change).await
            }
            Event::WipStateChanged { patchset, change }
                if !change.wip && !change.private && code_change_in_patchset(&patchset) =>
            {
                self.new_pipeline(patchset, change).await
            }
            Event::PrivateStateChanged { patchset, change }
                if !change.wip && !change.private && code_change_in_patchset(&patchset) =>
            {
                self.new_pipeline(patchset, change).await
            }
            Event::ChangeAbandoned { patchset, change } => {
                self.retire_change(patchset, change).await
            }
            Event::ChangeDeleted { patchset, change } => self.retire_change(patchset, change).await,
            Event::ChangeMerged { patchset, change } => self.retire_change(patchset, change).await,
            Event::CommentAdded {
                comment,
                patchset,
                change,
                ..
            } => {
                if comment.ends_with(
                    self.config
                        .gitlab
                        .api
                        .retry_command
                        .as_ref()
                        .map(|v| &v[..])
                        .unwrap_or_else(|| "retry"),
                ) {
                    self.retry_pipeline(patchset, change).await;
                    return;
                }
                if comment.ends_with(
                    self.config
                        .gitlab
                        .api
                        .force_retry_command
                        .as_ref()
                        .map(|v| &v[..])
                        .unwrap_or_else(|| "retry force"),
                ) {
                    self.new_pipeline(patchset, change).await;
                }
            }
            _ => {}
        };
    }

    // TODO: Consider adding impl/non-impl version to catch all errors and log
    async fn retry_pipeline(&self, patchset: Patchset, change: Change) {
        let id = LoggingEnricher::new(&patchset, &change);
        let client = gitlab::Client::new(self.config.clone());
        let git = git::Git::new(self.config.clone());
        let gerrit_reviewer = gerrit_ssh_command::GerritSshCommand::new(self.config.clone());

        let branch = git.generate_branch_name(&change);

        let retry_pipeline_id = match client.latest_pipeline(&branch, &change.project).await {
            Ok(PipelineQueryResult::LatestPipeline { pipeline })
                if matches!(pipeline.status, PipelineStatus::Failed) =>
            {
                log::info!(
                    "User requested retry and a candidate was found ({})",
                    pipeline.id
                );
                pipeline.id
            }
            Ok(PipelineQueryResult::LatestPipeline { pipeline }) => {
                log::info!(
                    "User requested retry but the latest pipeline is not failed but {:?}",
                    pipeline.status
                );
                if let Err(e) = gerrit_reviewer
                    .review(
                        &change,
                        &patchset,
                        Review {
                            message: format!(
                                "Latest pipeline ({}) found was {:?}, cannot retry.",
                                pipeline.web_url, pipeline.status
                            ),
                            ..Default::default()
                        },
                    )
                    .await
                {
                    log::error!("Could not send a review: {e:?}");
                }
                return;
            }
            Ok(PipelineQueryResult::NotFound) => {
                log::info!("User requested retry but the latest pipeline was not found",);
                if let Err(e) = gerrit_reviewer
                    .review(
                        &change,
                        &patchset,
                        Review {
                            message: "Latest pipeline was not found.".into(),
                            ..Default::default()
                        },
                    )
                    .await
                {
                    log::error!("Could not send a review: {e:?}");
                }
                return;
            }
            Err(e) => {
                log::error!("{id} Error when trying to query the latest pipeline status: {e:?}");
                if let Err(e) = gerrit_reviewer
                    .review(
                        &change,
                        &patchset,
                        Review {
                            message: format!(
                                "Error occured when trying to query the latest pipeline status."
                            ),
                            ..Default::default()
                        },
                    )
                    .await
                {
                    log::error!("Could not send a review: {e:?}");
                }
                return;
            }
        };
        match client
            .retry_pipeline(retry_pipeline_id, &change.project)
            .await
        {
            Ok(pipeline) => {
                log::info!("Pipeline ({}) retried", pipeline.id);
                if let Err(e) = gerrit_reviewer
                    .review(
                        &change,
                        &patchset,
                        Review {
                            message: format!("Pipeline retried ({})", pipeline.web_url),
                            ..Default::default()
                        },
                    )
                    .await
                {
                    log::error!("Could not send a review: {e:?}");
                }
            }
            Err(e) => {
                log::error!("{id} Error when trying to retry the latest pipeline status: {e:?}");
                if let Err(e) = gerrit_reviewer
                    .review(
                        &change,
                        &patchset,
                        Review {
                            message: format!(
                                "Error occured when trying to retry the latest pipeline status."
                            ),
                            ..Default::default()
                        },
                    )
                    .await
                {
                    log::error!("Could not send a review: {e:?}");
                }
                return;
            }
        }

        if let Err(e) = self
            .track_pipeline(retry_pipeline_id, &branch, &patchset, &change)
            .await
        {
            log::error!("{id} Error when trying to track pipeline: {e:?}");
        }
    }

    // Create and track a new branch until
    async fn new_pipeline(&self, patchset: Patchset, change: Change) {
        let id = LoggingEnricher::new(&patchset, &change);
        if change.branch.starts_with("refs") {
            log::info!("{id} Skipping internal gerrit ref: {}", &change.branch);
            return;
        }
        let git = git::Git::new(self.config.clone());
        let push_operation = match git.push(&patchset, &change).await {
            Ok(op) => op,
            Err(e) => {
                log::error!("{id} Failed to git-push: {e:?}");
                return;
            }
        };
        log::debug!("{id} git-push operation: {:?}", push_operation);

        // Give some time to GitLab, just in case? Maybe it is not needed.
        tokio::time::sleep(Duration::from_secs(1)).await;

        if matches!(push_operation, git::PushOperation::NoChange { .. }) {
            log::warn!("No change yet new pipeline created? Fine if explicitly forced");
        }

        let branch = push_operation.branch();

        let client = gitlab::Client::new(self.config.clone());
        match client
            .pipelines(&branch, &patchset.revision, &change.project)
            .await
        {
            Ok(pipelines) => {
                // Codepath when pipelines for a given ref & revision
                // exist before an explicit trigger.
                // This can happen if:
                // - a CI YAML contains jobs that run ruleless Thus pipeline will be
                //   autotriggered on push. It is not enough to reuse the pipeline because
                //   some jobs with Gerrit rule (GERRIT var) will not run.
                // - one thread races and triggers a pipeline in the name of some other
                //   patchset We cannot reuse it because we cannot differentiate it from the
                //   former case (maybe we could tell from looking at the user because an
                //   explicit trigger should be caused by the API user, not real one)
                //
                // Solution is to cancel all jobs for a revision and trigger one proper.
                // We cannot be cancelled by others because we are uniquely representing
                // one branch/ref and one revision.
                for pipeline in pipelines.into_iter() {
                    log::warn!(
                                    "{id} Pipeline ({}) got possibly autotriggered or race-triggered by another thread, cancelling",
                                    pipeline.id
                                );
                    let cancel_result = client.cancel_pipeline(pipeline.id, &change.project).await;
                    log::debug!("{id} Cancellation attempt result: {cancel_result:?}");
                }
            }
            Err(e) => {
                log::error!("{id} Error when trying to query the latest pipeline status: {e:?}");
                return;
            }
        }

        // We assume that if pipeline.sha != patchset.revision right after git-push,
        // we should trigger a pipeline corresponding to our patchset.
        // If we trigger a pipeline for a wrong revision in a race
        // between git operations and gitlab queries, we gonna get
        // cancelled and thus eventually terminate.
        let pipeline_id = match client.trigger_pipeline(&branch, &change.project).await {
            Ok(pipeline) => {
                log::debug!("{id} Pipeline ({}) triggered", pipeline.id);
                pipeline.id
            }
            Err(e) => {
                // TODO: BadRequest might mean malformed YAML, should be reported
                // to the user
                // Probably more gerrit_reviewer.review will be needed on error cases
                // But not too much in order not to spam, I guess
                log::error!("{id} Error when trying to trigger a pipeline: {e:?}");
                return;
            }
        };

        if let Err(e) = self
            .track_pipeline(pipeline_id, &branch, &patchset, &change)
            .await
        {
            log::error!("{id} Error when trying to track pipeline: {e:?}");
        }
    }

    async fn retire_change(&self, patchset: Patchset, change: Change) {
        let id = LoggingEnricher::new(&patchset, &change);
        if change.branch.starts_with("refs") {
            log::info!("{id} Skipping internal gerrit ref: {}", &change.branch);
            return;
        }

        let git = git::Git::new(self.config.clone());

        match git.push_delete(&change).await {
            Ok(_) => log::info!("{id} Successfully removed a branch"),
            Err(e) => {
                log::error!("{id} Error when trying to retire change: {e:?}")
            }
        }
    }

    // Logic: If pipeline's
    // 1. current.id != latest.id, terminate
    // 2. current.id == latest.id && status == success/fail, terminate and do the
    //    review
    // 3. otherwise repeat in the loop
    //
    // This function, again, should quite agressively terminate itself when it
    // detects it is irrelevant. Possibly cancel the pipelines that are
    // irrelevant as well.
    async fn track_pipeline(
        &self,
        pipeline_id: u64,
        branch: &str,
        patchset: &Patchset,
        change: &Change,
    ) -> anyhow::Result<()> {
        let id = LoggingEnricher::new(patchset, change);

        let client = gitlab::Client::new(self.config.clone());

        let mut counter: u64 = 0;
        let mut not_found_allowed_counter: u8 = 5;
        loop {
            counter += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;

            let latest_pipeline = {
                use gitlab::PipelineQueryResult::*;
                match client.latest_pipeline(branch, &change.project).await {
                    Ok(LatestPipeline { pipeline }) => pipeline,
                    Ok(NotFound) if not_found_allowed_counter > 0 => {
                        // Weirdly enough, this endpoint becomes blank when new revision of the ref
                        // is pushed. This is were this weird counter comes from.
                        log::warn!("Latest pipeline not found, probably new ref pushed, trying again ({} tries remaining)", not_found_allowed_counter);
                        not_found_allowed_counter -= 1;
                        continue;
                    }
                    v => {
                        return Err(anyhow::anyhow!(
                            "Error when trying to query the latest pipeline status: {v:?}"
                        ));
                    }
                }
            };

            log::trace!("{id} Response body: {latest_pipeline:#?}");

            if latest_pipeline.id != pipeline_id || latest_pipeline.sha != patchset.revision {
                log::info!("{id} Terminating, tracked pipeline (id: {}) is not relevant. Latest: id:{} for rev:{}", pipeline_id, latest_pipeline.id, latest_pipeline.sha);
                let cancel_result = client.cancel_pipeline(pipeline_id, &change.project).await;
                log::debug!("{id} Cancel attempt of the current pipeline: {cancel_result:?}");
                break;
            }

            {
                let gerrit_reviewer =
                    gerrit_ssh_command::GerritSshCommand::new(self.config.clone());

                use gitlab::PipelineStatus::*;
                match latest_pipeline.status {
                    Success => {
                        log::info!("{id} Pipeline succeeded");
                        gerrit_reviewer
                            .review(
                                &change,
                                &patchset,
                                Review {
                                    message: format!(
                                        "Pipeline succeeded ({})",
                                        latest_pipeline.web_url
                                    ),
                                    verified: Some(Verified::Positive),
                                },
                            )
                            .await?;
                        break;
                    }
                    Failed => {
                        log::info!("{id} Pipeline failed");
                        gerrit_reviewer
                            .review(
                                &change,
                                &patchset,
                                Review {
                                    message: format!(
                                        "Pipeline failed ({})",
                                        latest_pipeline.web_url
                                    ),
                                    verified: Some(Verified::Negative),
                                },
                            )
                            .await?;
                        break;
                    }
                    Canceled => {
                        log::info!("{id} Pipeline canceled");
                        gerrit_reviewer
                            .review(
                                &change,
                                &patchset,
                                Review {
                                    message: format!(
                                        "Pipeline was cancelled ({})",
                                        latest_pipeline.web_url
                                    ),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        break;
                    }
                    status @ (Skipped | Manual | Scheduled) => {
                        return Err(anyhow::anyhow!("Unexpected pipeline status: {status:?}?"));
                    }
                    status => {
                        log::info!("{id} Pipeline is in progress, status: {status:?}");
                        if counter == 3 {
                            gerrit_reviewer
                                .review(
                                    &change,
                                    &patchset,
                                    Review {
                                        message: format!(
                                            "Pipeline was started ({})",
                                            latest_pipeline.web_url
                                        ),
                                        ..Default::default()
                                    },
                                )
                                .await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn new(config: config::Config) -> Self {
        Self { config }
    }
}

pub struct LoggingEnricher<'a> {
    change_number: u64,
    change_id: &'a str,
    commit_sha: &'a str,
    patchset_number: u64,
}

impl<'a> LoggingEnricher<'a> {
    pub fn new(patchset: &'a Patchset, change: &'a Change) -> Self {
        Self {
            change_number: change.number,
            change_id: &change.id[..7],
            commit_sha: &patchset.revision[..7],
            patchset_number: patchset.number,
        }
    }
}

impl<'a> core::fmt::Display for LoggingEnricher<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        self.change_number.fmt(f)?;
        f.write_str("/")?;
        self.change_id.fmt(f)?;
        f.write_str("|")?;
        self.commit_sha.fmt(f)?;
        f.write_str("/")?;
        self.patchset_number.fmt(f)?;
        f.write_str("]")
    }
}
