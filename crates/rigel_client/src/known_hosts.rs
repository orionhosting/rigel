use std::{fs, path::PathBuf};

use directories::ProjectDirs;
use russh::keys::PublicKey;

use crate::error::{ClientError, Result};

/// Result type of a server host key checked against the knownhosts store.
#[derive(Debug, PartialEq, Eq)]
pub enum HostKeyCheck {
    /// Never seen this host before; key was recorded (trust-on-first-use).
    NewlyTrusted,
    /// The key matches what is already stored.
    Matches,
    /// The stored key is different.
    ///
    /// So that means that either:
    /// - the server key changed
    /// - or someone is MITM the connection
    Mismatch {
        recorded_fingerprint: String,
        offered_fingerprint: String,
    },
}

/// TODO: the project dirs should be customizable?
fn known_hosts_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("dev", "orionhosting", "rigel")
        .ok_or_else(|| ClientError::Config("could not determine config directory".into()))?;
    let dir = dirs.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("known_hosts"))
}

/// Normalize a known-host entry key.
fn entry_key(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    }
}

/// Load entries.
///
/// Note:
///
/// They are stored in plain text, and not compatible with ~/.ssh/known_hosts.
/// TODO: make this compatible; a openssh parser is in ssh_key known hosts
fn load_entries() -> Result<Vec<(String, String)>> {
    let path = known_hosts_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(path)?;

    Ok(text
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect())
}

/// Saves new entries to the known host path.
fn save_entries(entries: &[(String, String)]) -> Result<()> {
    let path = known_hosts_path()?;
    let text = entries
        .iter()
        .map(|(host, key)| format!("{host} {key}\n"))
        .collect::<String>();

    fs::write(path, text)?;
    Ok(())
}

/// Check `offered_key` against the stored entry for this host and port.
///
/// On a mismatch, it should not overwrite the stored key and tell the user about the mismatch so
/// he can chose what to do instead.
pub fn check_and_record(host: &str, port: u16, offered_key: &PublicKey) -> Result<HostKeyCheck> {
    let mut entries = load_entries()?;
    let key = entry_key(host, port);
    let offered_encoded = offered_key
        .to_openssh()
        .map_err(|e| ClientError::Config(format!("failed to encode host key: {e}")))?;

    if let Some((_, recorded_encoded)) = entries.iter().find(|(h, _)| h == &key) {
        if recorded_encoded == &offered_encoded {
            return Ok(HostKeyCheck::Matches);
        }
        let recorded_key = PublicKey::from_openssh(recorded_encoded)
            .map_err(|e| ClientError::Config(format!("failed to parse stored host key: {e}")))?;
        return Ok(HostKeyCheck::Mismatch {
            recorded_fingerprint: recorded_key.fingerprint(Default::default()).to_string(),
            offered_fingerprint: offered_key.fingerprint(Default::default()).to_string(),
        });
    }

    entries.push((key, offered_encoded));
    save_entries(&entries)?;
    Ok(HostKeyCheck::NewlyTrusted)
}

/// Forget an entry, e.g. after the user confirms a legitimate server
/// rekey and wants to accept the new key.
pub fn forget_host(host: &str, port: u16) -> Result<()> {
    let mut entries = load_entries()?;
    let key = entry_key(host, port);
    entries.retain(|(h, _)| h != &key);

    save_entries(&entries)
}
