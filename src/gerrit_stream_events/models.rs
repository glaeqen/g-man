use serde::Deserialize;

/// Gerrit Event
///
/// According to the documentation, any field can be possibly
/// missing so that's.. nice.
///
/// https://gerrit-review.googlesource.com/Documentation/json.html
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    PatchsetCreated {
        #[serde(rename = "patchSet")]
        patchset: Patchset,
        change: Change,
    },
    #[serde(rename_all = "camelCase")]
    ChangeAbandoned {
        #[serde(rename = "patchSet")]
        patchset: Patchset,
        change: Change,
    },
    #[serde(rename_all = "camelCase")]
    ChangeDeleted {
        #[serde(rename = "patchSet")]
        patchset: Patchset,
        change: Change,
    },
    #[serde(rename_all = "camelCase")]
    ChangeMerged {
        #[serde(rename = "patchSet")]
        patchset: Patchset,
        change: Change,
    },
    #[serde(rename_all = "camelCase")]
    ChangeRestored {
        #[serde(rename = "patchSet")]
        patchset: Patchset,
        change: Change,
    },
    #[serde(rename_all = "camelCase")]
    CommentAdded {
        comment: String,
        #[serde(rename = "patchSet")]
        patchset: Patchset,
        change: Change,
        // approvals: Vec<Approvals>,
    },
    #[serde(rename_all = "camelCase")]
    WipStateChanged {
        #[serde(rename = "patchSet")]
        patchset: Patchset,
        change: Change,
    },
    #[serde(rename_all = "camelCase")]
    PrivateStateChanged {
        #[serde(rename = "patchSet")]
        patchset: Patchset,
        change: Change,
    },
}

// Maybe will be useful at some point with some fancier gating
// system.
/*
#[serde_with::serde_as]
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Approvals {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub old_value: i8,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub value: i8,
}
*/

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Patchset {
    #[serde(rename = "ref")]
    pub reference: String,
    pub kind: PatchsetKind,
    pub revision: String,
    pub number: u64,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PatchsetKind {
    Rework,
    TrivialRebase,
    MergeFirstParentUpdate,
    NoCodeChange,
    NoChange,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub project: String,
    pub branch: String,
    pub id: String,
    pub number: u64,
    pub status: ChangeStatus,
    #[serde(default)]
    pub wip: bool,
    #[serde(default)]
    pub private: bool,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum ChangeStatus {
    New,
    Merged,
    Abandoned,
}
