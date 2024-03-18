use std::{io::Write, path::PathBuf};

use tokio::{process::Command, sync::Mutex};

use crate::{
    config::{self, FindRepository},
    gerrit_stream_events::models::{Change, Patchset},
};

#[derive(Debug, Clone)]
pub enum PushOperation {
    Updated { branch: String },
    NoChange { branch: String },
}

impl PushOperation {
    pub fn branch(self) -> String {
        match self {
            PushOperation::Updated { branch } => branch,
            PushOperation::NoChange { branch } => branch,
        }
    }
}

pub struct Git {
    config: config::Config,
}

static GLOBAL_GIT_MUTEX: Mutex<()> = Mutex::const_new(());

impl Git {
    pub fn new(config: config::Config) -> Self {
        Self { config }
    }

    pub fn generate_branch_name(&self, change: &Change) -> String {
        format!(
            "{}{}-{}",
            &self.config.gitlab.ci_branch_prefix, &change.number, &change.id
        )
    }

    fn retrieve_remote(&self, change: &Change) -> anyhow::Result<String> {
        self.config.repositories.find_git_remote(&change.project)
    }

    fn retrieve_git_dir(&self, change: &Change) -> PathBuf {
        self.config
            .gerrit
            .internal_git_directory
            .join(format!("{}.git", &change.project))
    }

    async fn run(mut command: Command) -> anyhow::Result<Vec<u8>> {
        let _lock = GLOBAL_GIT_MUTEX.lock().await;
        let output = command.output().await?;
        if !output.status.success() {
            log::error!("Command failed. Stdout:");
            std::io::stdout()
                .write_all(&output.stdout)
                .expect("self stdout write failed?");
            log::error!("Command failed. Stderr:");
            std::io::stdout()
                .write_all(&output.stderr)
                .expect("self stderr write failed?");
            return Err(anyhow::anyhow!(
                "Process finished unsuccessfully: {:?}",
                &output.status
            ));
        }
        Ok(output.stdout)
    }

    pub async fn push(
        &self,
        patchset: &Patchset,
        change: &Change,
    ) -> anyhow::Result<PushOperation> {
        let mut command = Command::new("git");
        if change.branch.starts_with("refs") {
            return Err(anyhow::anyhow!(
                "Tried to push an internal ref: {}",
                &change.branch
            ));
        }
        let branch = self.generate_branch_name(change);
        command
            .env("GIT_DIR", self.retrieve_git_dir(change))
            .arg("push")
            .arg("--porcelain")
            .arg("--force")
            .arg(self.retrieve_remote(change)?)
            .arg(&format!("{}:refs/heads/{}", &patchset.reference, &branch));

        let stdout = Self::run(command).await?;
        for window in stdout.windows(2) {
            if window[0] != b'\n' {
                continue;
            }
            match window[1] {
                b'*' | b'+' => return Ok(PushOperation::Updated { branch }),
                b'=' => return Ok(PushOperation::NoChange { branch }),
                _ => {}
            }
        }
        Err(anyhow::anyhow!(
            "git-push returned 0 but parsing did not find \n[*+=]?"
        ))
    }

    pub async fn push_delete(&self, change: &Change) -> anyhow::Result<()> {
        let mut command = Command::new("git");
        command
            .env("GIT_DIR", self.retrieve_git_dir(change))
            .arg("push")
            .arg("--porcelain")
            .arg("--delete")
            .arg(self.retrieve_remote(change)?)
            .arg(self.generate_branch_name(change));

        let stdout = Self::run(command).await?;
        for window in stdout.windows(2) {
            if window[0] != b'\n' {
                continue;
            }
            match window[1] {
                b'-' => return Ok(()),
                _ => {}
            }
        }
        Err(anyhow::anyhow!(
            "git-push returned 0 but parsing did not find \n[-]?"
        ))
    }
}
