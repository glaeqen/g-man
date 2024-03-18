use std::path::PathBuf;

use serde::Deserialize;
use tokio::process::Command;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub user: String,
    pub host: String,
    pub port: Option<u16>,
    pub private_key_path: Option<PathBuf>,
}

impl Config {
    /// Yields a preconfigured command
    pub fn command(&self) -> Command {
        let mut command = Command::new("ssh");
        if let Some(private_key_path) = &self.private_key_path {
            command.arg("-i").arg(private_key_path);
        }
        if let Some(port) = self.port {
            command.args(["-p", &format!("{}", port)]);
        }
        command
            .args(["-o", "ServerAliveInterval=3"])
            .args(["-o", "ServerAliveCountMax=3"])
            .args(["-o", "ConnectTimeout=3"])
            .arg(format!("{}@{}", &self.user, &self.host));
        command
    }
}
