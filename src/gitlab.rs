use reqwest::{
    header::{HeaderMap, HeaderValue},
    StatusCode,
};

use crate::config::{self, FindRepository};

pub struct Client {
    config: config::Config,
    client: reqwest::Client,
}

#[derive(Clone, Debug)]
pub enum PipelineQueryResult {
    LatestPipeline { pipeline: Pipeline },
    NotFound,
}

impl Client {
    pub fn new(config: config::Config) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    pub async fn latest_pipeline(
        &self,
        branch: &str,
        project_name: &str,
    ) -> anyhow::Result<PipelineQueryResult> {
        let url = format!(
            "https://gitlab.com/api/v4/projects/{}/pipelines/latest?ref={}",
            self.project_id(project_name)?,
            branch,
        );
        log::trace!("latest_pipeline query with a URL: {url}");
        let auth_headers = self.auth_headers(project_name)?;
        let response = self.client.get(url).headers(auth_headers).send().await?;
        if response.status() == StatusCode::FORBIDDEN {
            return Ok(PipelineQueryResult::NotFound);
        }
        let pipeline = response.error_for_status()?.json().await?;
        Ok(PipelineQueryResult::LatestPipeline { pipeline })
    }

    pub async fn pipelines(
        &self,
        branch: &str,
        sha: &str,
        project_name: &str,
    ) -> anyhow::Result<Vec<Pipeline>> {
        let url = format!(
            "https://gitlab.com/api/v4/projects/{}/pipelines?ref={}&sha={}",
            self.project_id(project_name)?,
            branch,
            sha
        );
        log::trace!("pipelines_for_branch query with a URL: {url}");
        let auth_headers = self.auth_headers(project_name)?;
        let response = self.client.get(url).headers(auth_headers).send().await?;
        let pipelines = response.error_for_status()?.json().await?;
        Ok(pipelines)
    }

    pub async fn trigger_pipeline(&self, branch: &str, project_name: &str) -> anyhow::Result<Pipeline> {
        let url = format!(
            "https://gitlab.com/api/v4/projects/{}/pipeline",
            self.project_id(project_name)?,
        );
        log::trace!("trigger_pipeline query with a URL: {url}");
        let auth_headers = self.auth_headers(project_name)?;
        let response = self
            .client
            .post(url)
            .headers(auth_headers)
            .json(&TriggerPipelineRequestBody::new_gerrit_pipeline(branch))
            .send()
            .await?;
        let pipeline = response.error_for_status()?.json().await?;
        Ok(pipeline)
    }

    pub async fn cancel_pipeline(&self, id: u64, project_name: &str) -> anyhow::Result<Pipeline> {
        let url = format!(
            "https://gitlab.com/api/v4/projects/{}/pipelines/{}/cancel",
            self.project_id(project_name)?,
            id,
        );
        log::trace!("cancel_pipeline query with a URL: {url}");
        let auth_headers = self.auth_headers(project_name)?;
        let response = self
            .client
            .post(url)
            .headers(auth_headers)
            .send()
            .await?;
        let pipeline = response.error_for_status()?.json().await?;
        Ok(pipeline)
    }

    pub async fn retry_pipeline(&self, id: u64, project_name: &str) -> anyhow::Result<Pipeline> {
        let url = format!(
            "https://gitlab.com/api/v4/projects/{}/pipelines/{}/retry",
            self.project_id(project_name)?,
            id
        );
        log::trace!("retry_pipeline query with a URL: {url}");
        let auth_headers = self.auth_headers(project_name)?;
        let response = self
            .client
            .post(url)
            .headers(auth_headers)
            .send()
            .await?;
        let pipeline = response.error_for_status()?.json().await?;
        Ok(pipeline)
    }

    fn project_id(&self, project_name: &str) -> anyhow::Result<u64> {
        self.config.repositories.find_project_id(project_name)
    }

    fn auth_headers(&self, project_name: &str) -> anyhow::Result<reqwest::header::HeaderMap> {
        let maybe_project_pat = self
            .config
            .repositories
            .find_private_access_token(project_name)?;
        let maybe_project_pat = maybe_project_pat.as_ref();
        let maybe_global_pat = || self.config.gitlab.api.private_access_token.as_ref();

        let private_access_token =
            maybe_project_pat.or_else(maybe_global_pat).ok_or_else(|| {
                anyhow::anyhow!("No private access tokens available for API authentication")
            })?;
        let mut headers = HeaderMap::new();
        headers.append(
            "PRIVATE-TOKEN",
            HeaderValue::from_str(private_access_token)?,
        );
        Ok(headers)
    }
}

#[derive(serde::Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    Created,
    WaitingForResource,
    Preparing,
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
    Skipped,
    Manual,
    Scheduled,
}

impl PipelineStatus {
    pub fn is_running(self) -> bool {
        use PipelineStatus::*;
        match self {
            Created | WaitingForResource | Preparing | Pending | Running => true,
            Success | Failed | Canceled => false,
            // I hope that is true?
            Skipped | Manual | Scheduled => false,
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Pipeline {
    pub id: u64,
    pub status: PipelineStatus,
    pub sha: String,
    pub web_url: String,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct TriggerPipelineRequestBody {
    #[serde(rename = "ref")]
    pub reference: String,
    pub variables: Option<ArrayOfHashes>,
}

impl TriggerPipelineRequestBody {
    fn new_gerrit_pipeline(branch: &str) -> Self {
        Self {
            reference: String::from(branch),
            // Mimick the merge request pipeline
            variables: Some(ArrayOfHashes::new([("CI_PIPELINE_SOURCE", "merge_request_event")])),
        }
    }
}

// https://docs.gitlab.com/ee/api/rest/index.html#array-of-hashes
#[derive(serde::Serialize, Debug, Clone)]
#[serde(transparent)]
pub struct ArrayOfHashes {
    variables: Vec<HashKeyValuePair>,
}

impl ArrayOfHashes {
    fn new<K: AsRef<str>, V: AsRef<str>>(v: impl IntoIterator<Item = (K, V)>) -> Self {
        Self {
            variables: v
                .into_iter()
                .map(|(k, v)| HashKeyValuePair {
                    key: String::from(k.as_ref()),
                    value: String::from(v.as_ref()),
                })
                .collect(),
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct HashKeyValuePair {
    key: String,
    value: String,
}
