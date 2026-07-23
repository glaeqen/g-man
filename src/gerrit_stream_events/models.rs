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
    NoCodeChange,
    NoChange,
    /// All other patchset kinds: these kinds all imply
    /// a change that requires a CI rerun.
    #[serde(other)]
    Other,
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

// Test that we can handle TRIVIAL_REBASE_WITH_MESSAGE_UPDATE variants.
#[test]
fn can_deserialize_trivial_rebase_with_msg_update() {
    let object = r#"
    {
        "uploader": {
            "name": "Jona",
            "email": "johannes.draaijer@kiteshield.com",
            "username": "datdenkikniet"
        },
        "patchSet": {
            "number": 2,
            "revision": "fa6b3cf76e9cda2f2e87f31aad124148a49ef49a",
            "parents": [
                "aaab08112f4eb3efab54634d2c1e934728421805"
            ],
            "ref": "refs/changes/82/2082/2",
            "uploader": {
                "name": "Jona",
                "email": "johannes.draaijer@kiteshield.com",
                "username": "datdenkikniet"
            },
            "createdOn": 1784815311,
            "author": {
                "name": "Jona",
                "email": "johannes.draaijer@kiteshield.com",
                "username": "datdenkikniet"
            },
            "kind": "TRIVIAL_REBASE_WITH_MESSAGE_UPDATE",
            "sizeInsertions": 10,
            "sizeDeletions": 0
        },
        "change": {
            "project": "kiteshield/imxrt118x-hal",
            "branch": "master",
            "id": "Ib5fa8c667c3e8ff42e8b2404b5083c8b6af2af6d",
            "number": 2082,
            "subject": "trest 2",
            "owner": {
                "name": "Jona",
                "email": "johannes.draaijer@kiteshield.com",
                "username": "datdenkikniet"
            },
            "url": "https://gerrit.kiteshield.com/c/kiteshield/imxrt118x-hal/+/2082",
            "commitMessage": "trest 2\n\nChange-Id: Ib5fa8c667c3e8ff42e8b2404b5083c8b6af2af6d\n",
            "createdOn": 1784815118,
            "status": "NEW",
            "wip": true
        },
        "project": "kiteshield/imxrt118x-hal",
        "refName": "refs/heads/master",
        "changeKey": {
            "id": "Ib5fa8c667c3e8ff42e8b2404b5083c8b6af2af6d"
        },
        "type": "patchset-created",
        "eventCreatedOn": 1784815311
    }
    "#;

    let _success: Event = serde_json::from_str(object).unwrap();
}
