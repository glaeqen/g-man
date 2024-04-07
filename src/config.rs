use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::ssh;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub gerrit: GerritConfig,
    pub gitlab: GitlabConfig,
    #[serde(rename = "repository")]
    pub repositories: Vec<RepositoryConfig>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RepositoryConfig {
    pub name: String,
    pub gitlab_git_remote: String,
    pub gitlab_project_id: u64,
    pub private_access_token: Option<String>,
}

pub trait FindRepository {
    fn find_git_remote(&self, name: &str) -> anyhow::Result<String>;
    fn find_project_id(&self, name: &str) -> anyhow::Result<u64>;
    fn find_private_access_token(&self, name: &str) -> anyhow::Result<Option<String>>;
}

impl FindRepository for Vec<RepositoryConfig> {
    fn find_git_remote(&self, name: &str) -> anyhow::Result<String> {
        self.iter().filter(|v| v.name == name).next().map_or_else(
            || {
                Err(anyhow::anyhow!(
                    "Repository {} is not registered in a configuration",
                    name
                ))
            },
            |v| Ok(v.gitlab_git_remote.clone()),
        )
    }

    fn find_project_id(&self, name: &str) -> anyhow::Result<u64> {
        self.iter().filter(|v| v.name == name).next().map_or_else(
            || {
                Err(anyhow::anyhow!(
                    "Repository {} is not registered in a configuration",
                    name
                ))
            },
            |v| Ok(v.gitlab_project_id),
        )
    }

    fn find_private_access_token(&self, name: &str) -> anyhow::Result<Option<String>> {
        self.iter().filter(|v| v.name == name).next().map_or_else(
            || {
                Err(anyhow::anyhow!(
                    "Repository {} is not registered in a configuration",
                    name
                ))
            },
            |v| Ok(v.private_access_token.clone()),
        )
    }
}

// TODO: Custom keys support is broken, for now - use default keys from `$USER/.ssh`
#[derive(Deserialize, Clone, Debug)]
pub struct GerritConfig {
    pub ssh: ssh::Config,
    pub internal_git_directory: PathBuf,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GitlabConfig {
    pub ci_branch_prefix: String,
    #[serde(default)]
    pub ssh: GitlabSshConfig,
    #[serde(default)]
    pub api: GitlabApiConfig,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct GitlabSshConfig {
    pub private_key_path: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct GitlabApiConfig {
    pub retry_command: Option<String>,
    pub force_retry_command: Option<String>,
    pub private_access_token: Option<String>,
}

pub fn load_from(config_path: impl AsRef<Path>) -> Config {
    toml::from_str(&std::fs::read_to_string(config_path).expect("could not read the config"))
        .unwrap()
    // TODO: Better error print-out
}
