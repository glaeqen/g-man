use std::io::Write;

use crate::{
    config,
    gerrit_stream_events::models::{Change, Patchset},
    helpers::LoggingEnricher,
};

pub struct GerritSshCommand {
    config: config::Config,
}

impl GerritSshCommand {
    pub fn new(config: config::Config) -> Self {
        Self { config }
    }

    pub async fn review(
        &self,
        change: &Change,
        patchset: &Patchset,
        review: Review,
    ) -> anyhow::Result<()> {
        let id = LoggingEnricher::new(patchset, change);
        let change_number = change.number;
        let patchset_number = patchset.number;
        let mut command = self.config.gerrit.ssh.command();
        command.args(["gerrit", "review"]);
        command.args(["--message", &format!("\"{}\"", &review.message)]);
        if let Some(verified) = &review.verified {
            let label = "Verified";
            let value = match verified {
                Verified::Reset => "0",
                Verified::Positive => "+1",
                Verified::Negative => "-1",
            };
            let label = format!("{}={}", label, value);
            command.args(["--label", &label]);
        }
        command.arg(format!("{},{}", change_number, patchset_number));
        log::info!(
            "{id} Review message: ({}) with label ({:?})",
            &review.message,
            &review.verified
        );
        log::debug!("{id} Running a command: {:#?}", command);
        let output = command.output().await?;
        if !output.status.success() {
            log::error!("Command failed. Stdout ({} bytes):", output.stdout.len());
            std::io::stdout()
                .write_all(&output.stdout)
                .expect("self stdout write failed?");
            log::error!("Command failed. Stderr ({} bytes):", output.stderr.len());
            std::io::stdout()
                .write_all(&output.stderr)
                .expect("self stderr write failed?");
            return Err(anyhow::anyhow!(
                "Process finished unsuccessfully: {:?}",
                &output.status
            ));
        }
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub enum Verified {
    #[default]
    Reset,
    Positive,
    Negative,
}

#[derive(Default, Debug, Clone)]
pub struct Review {
    pub message: String,
    pub verified: Option<Verified>,
}
