use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

use russh_sftp::client::SftpSession;

use crate::error::{ClientError, Result};
use crate::sftp::{self, RemoteEntry};
use crate::ssh::{Auth, HostKeyPolicy, SshConnection};
use crate::transfer::{
    ProgressReporter, StatusHandle, TransferDirection, TransferJob, TransferStatus,
};

/// A local directory entry, similar to RemoteEntry.
#[derive(Clone, Debug)]
pub struct LocalEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// List entries in a local directory.
///
/// Sorting: alphabetically with directories first
pub fn list_local_dir(path: &Path) -> Result<Vec<LocalEntry>> {
    let mut entries = Vec::new();
    for item in std::fs::read_dir(path)? {
        let item = item?;
        let metadata = item.metadata()?;

        entries.push(LocalEntry {
            name: item.file_name().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }
    entries.sort_by(|a, b| {
        (!a.is_dir, a.name.to_lowercase()).cmp(&(!b.is_dir, b.name.to_lowercase()))
    });

    Ok(entries)
}

/// A Rigel session.
///
/// The whole app state, without anything specific of
/// where it will be used. Meaning it can be used
/// by the TUI and GUI or anything else.
pub struct Session {
    /// The SSH connection.
    ssh: SshConnection,
    /// The SFTP session, which was obtained
    /// from the SSH connection.
    sftp: Arc<SftpSession>,

    pub remote_cwd: String,
    pub remote_entries: Vec<RemoteEntry>,

    pub local_cwd: PathBuf,
    pub local_entries: Vec<LocalEntry>,

    /// Current transfer jobs.
    pub transfers: Vec<TransferJob>,
    next_transfer_id: u64,
}

impl Session {
    /// Connect and load the directories
    /// for both local and remote.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        auth: &Auth,
        start_remote_path: &str,
        start_local_path: PathBuf,
    ) -> Result<Self> {
        let ssh =
            SshConnection::connect(host, port, user, auth, HostKeyPolicy::TrustOnFirstUse).await?;
        let sftp = Arc::new(ssh.open_sftp().await?);

        let remote_entries = sftp::list_dir(&sftp, start_remote_path).await?;
        let local_entries = list_local_dir(&start_local_path)?;

        Ok(Self {
            ssh,
            sftp,
            remote_cwd: start_remote_path.to_string(),
            remote_entries,
            local_cwd: start_local_path,
            local_entries,
            transfers: Vec::new(),
            next_transfer_id: 0,
        })
    }

    /// Run a command through SSH.
    pub async fn exec(&self, command: &str) -> Result<String> {
        self.ssh.exec(command).await
    }
}

// remote nav
impl Session {
    pub async fn enter_remote_dir(&mut self, name: &str) -> Result<()> {
        let new_path = join_remote(&self.remote_cwd, name);
        self.remote_entries = sftp::list_dir(&self.sftp, &new_path).await?;
        self.remote_cwd = new_path;
        Ok(())
    }

    pub async fn remote_up(&mut self) -> Result<()> {
        let parent = parent_remote(&self.remote_cwd)?;
        self.remote_entries = sftp::list_dir(&self.sftp, &parent).await?;
        self.remote_cwd = parent;
        Ok(())
    }

    pub async fn refresh_remote(&mut self) -> Result<()> {
        self.remote_entries = sftp::list_dir(&self.sftp, &self.remote_cwd).await?;
        Ok(())
    }
}

// local nav
impl Session {
    pub fn enter_local_dir(&mut self, name: &str) -> Result<()> {
        let new_path = self.local_cwd.join(name);
        self.local_entries = list_local_dir(&new_path)?;
        self.local_cwd = new_path;
        Ok(())
    }

    pub fn local_up(&mut self) -> Result<()> {
        let parent = self
            .local_cwd
            .parent()
            .ok_or_else(|| ClientError::NoParent(self.local_cwd.display().to_string()))?
            .to_path_buf();
        self.local_entries = list_local_dir(&parent)?;
        self.local_cwd = parent;
        Ok(())
    }

    pub fn refresh_local(&mut self) -> Result<()> {
        self.local_entries = list_local_dir(&self.local_cwd)?;
        Ok(())
    }
}

// fs functions
impl Session {
    pub async fn remote_mkdir(&mut self, name: &str) -> Result<()> {
        let path = join_remote(&self.remote_cwd, name);
        sftp::create_dir(&self.sftp, &path).await?;
        self.refresh_remote().await
    }

    pub async fn remote_delete_file(&mut self, name: &str) -> Result<()> {
        let path = join_remote(&self.remote_cwd, name);
        sftp::remove_file(&self.sftp, &path).await?;
        self.refresh_remote().await
    }

    pub async fn remote_delete_dir(&mut self, name: &str) -> Result<()> {
        let path = join_remote(&self.remote_cwd, name);
        sftp::remove_dir(&self.sftp, &path).await?;
        self.refresh_remote().await
    }

    pub async fn remote_rename(&mut self, from_name: &str, to_name: &str) -> Result<()> {
        let from = join_remote(&self.remote_cwd, from_name);
        let to = join_remote(&self.remote_cwd, to_name);
        sftp::rename(&self.sftp, &from, &to).await?;
        self.refresh_remote().await
    }
}

// transfers
impl Session {
    /// Queue and immediately spawn a background download of `name` (in the current
    /// remote dir) into the current local dir.
    ///
    /// It returns the job id.
    pub fn queue_download(&mut self, name: &str) -> u64 {
        let remote_path = join_remote(&self.remote_cwd, name);
        let local_path = self.local_cwd.join(name);
        self.queue_transfer(TransferDirection::Download, local_path, remote_path)
    }

    pub fn queue_upload(&mut self, name: &str) -> u64 {
        let local_path = self.local_cwd.join(name);
        let remote_path = join_remote(&self.remote_cwd, name);
        self.queue_transfer(TransferDirection::Upload, local_path, remote_path)
    }

    fn queue_transfer(
        &mut self,
        direction: TransferDirection,
        local_path: PathBuf,
        remote_path: String,
    ) -> u64 {
        let id = self.next_transfer_id;
        self.next_transfer_id += 1;

        let progress = ProgressReporter::new();
        let status = StatusHandle::new(TransferStatus::Queued);

        self.transfers.push(TransferJob {
            id,
            direction,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            progress: progress.clone(),
            status: status.clone(),
        });

        let sftp = self.sftp.clone();
        tokio::spawn(async move {
            status.set(TransferStatus::Running);
            let result =
                sftp::run_transfer(&sftp, direction, &local_path, &remote_path, progress).await;
            status.set(if result.is_ok() {
                TransferStatus::Done
            } else {
                TransferStatus::Failed
            });
        });

        id
    }
}

// editing
impl Session {
    /// Download a file in a temp dir so we can edit it.
    pub async fn download_for_edit(&self, name: &str) -> Result<PathBuf> {
        let remote_path = join_remote(&self.remote_cwd, name);

        // TODO: tempname customizable?
        let temp_dir = env::temp_dir().join("__rigel-temp");

        fs::create_dir_all(&temp_dir)?;
        let temp_path = temp_dir.join(name);

        sftp::download(&self.sftp, &remote_path, &temp_path, |_, _| {}).await?;
        Ok(temp_path)
    }

    /// Upload the temp file back to its original
    /// remote path, and refresh the remote listing.
    pub async fn upload_edited(&mut self, temp_path: &Path, name: &str) -> Result<()> {
        let remote_path = join_remote(&self.remote_cwd, name);
        sftp::upload(&self.sftp, temp_path, &remote_path, |_, _| {}).await?;
        self.refresh_remote().await
    }
}

fn join_remote(cwd: &str, name: &str) -> String {
    if cwd == "/" {
        format!("/{name}")
    } else {
        format!("{cwd}/{name}")
    }
}

fn parent_remote(cwd: &str) -> Result<String> {
    match cwd.rfind('/') {
        Some(0) => Ok("/".to_string()),
        Some(i) => Ok(cwd[..i].to_string()),
        None => Err(ClientError::NoParent(cwd.to_string())),
    }
}
