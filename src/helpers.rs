use crate::gerrit_stream_events::models::{Change, Patchset};

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
