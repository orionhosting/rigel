use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::{ClientError, Result};
use crate::ssh::Auth;

/// A saved host.
///
/// TODO: password & passphrase should be moved to
/// the OS keychain using the `keyring` crate so it is more secure.
/// (currently this is stored in toml)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedHost {
    pub label: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// The auth data.
    pub auth: SavedAuth,
    /// Remote directory to open on connect.
    pub start_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SavedAuth {
    Password(String),
    KeyFile {
        path: String,
        passphrase: Option<String>,
    },
}

impl From<&SavedAuth> for Auth {
    fn from(saved: &SavedAuth) -> Self {
        match saved {
            SavedAuth::Password(p) => Auth::Password(p.clone()),
            SavedAuth::KeyFile { path, passphrase } => Auth::KeyFile {
                path: path.clone(),
                passphrase: passphrase.clone(),
            },
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct HostsFile {
    #[serde(default)]
    hosts: Vec<SavedHost>,
}

/// TODO: projects dirs path should be customizable & centralized with the one in known hosts.
fn hosts_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("dev", "orionhosting", "rigel")
        .ok_or_else(|| ClientError::Config("could not determine config directory".into()))?;
    let dir = dirs.config_dir();
    std::fs::create_dir_all(dir)?;
    Ok(dir.join("hosts.toml"))
}

pub fn load_hosts() -> Result<Vec<SavedHost>> {
    let path = hosts_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path)?;
    let parsed: HostsFile = toml::from_str(&text)
        .map_err(|e| ClientError::Config(format!("failed to parse {}: {e}", path.display())))?;
    Ok(parsed.hosts)
}

pub fn save_hosts(hosts: &[SavedHost]) -> Result<()> {
    let path = hosts_path()?;
    let file = HostsFile {
        hosts: hosts.to_vec(),
    };
    let text = toml::to_string_pretty(&file)
        .map_err(|e| ClientError::Config(format!("failed to serialize hosts: {e}")))?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Add or replace (by label) a saved host.
pub fn upsert_host(new_host: SavedHost) -> Result<Vec<SavedHost>> {
    let mut hosts = load_hosts()?;
    if let Some(existing) = hosts.iter_mut().find(|h| h.label == new_host.label) {
        *existing = new_host;
    } else {
        hosts.push(new_host);
    }
    save_hosts(&hosts)?;
    Ok(hosts)
}

pub fn delete_host(label: &str) -> Result<Vec<SavedHost>> {
    let mut hosts = load_hosts()?;
    hosts.retain(|h| h.label != label);
    save_hosts(&hosts)?;
    Ok(hosts)
}
