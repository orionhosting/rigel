use std::path::Path;

use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use tokio::fs::File as LocalFile;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::Result;
use crate::transfer::{ProgressReporter, TransferDirection};

/// A remote entry.
#[derive(Clone, Debug)]
pub struct RemoteEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    /// Unix permission bits, if the server returned them.
    pub permissions: Option<u32>,
}

/// List entries in a remote directory.
///
/// Sorting: alphabetically with directories first
pub async fn list_dir(sftp: &SftpSession, path: &str) -> Result<Vec<RemoteEntry>> {
    let raw = sftp.read_dir(path).await?;

    let mut entries: Vec<RemoteEntry> = raw
        .map(|e| RemoteEntry {
            name: e.file_name(),
            is_dir: e.file_type().is_dir(),
            size: e.metadata().size.unwrap_or(0),
            permissions: e.metadata().permissions,
        })
        .collect();
    entries.sort_by(|a, b| {
        (!a.is_dir, a.name.to_lowercase()).cmp(&(!b.is_dir, b.name.to_lowercase()))
    });

    Ok(entries)
}

pub async fn create_dir(sftp: &SftpSession, path: &str) -> Result<()> {
    sftp.create_dir(path).await?;
    Ok(())
}

pub async fn remove_file(sftp: &SftpSession, path: &str) -> Result<()> {
    sftp.remove_file(path).await?;
    Ok(())
}

pub async fn remove_dir(sftp: &SftpSession, path: &str) -> Result<()> {
    sftp.remove_dir(path).await?;
    Ok(())
}

pub async fn rename(sftp: &SftpSession, from: &str, to: &str) -> Result<()> {
    sftp.rename(from, to).await?;
    Ok(())
}

/// Download `remote_path` to `local_path`.
///
/// `on_progress` is a callback `(bytes_done, total_bytes)` which is called
/// every time new bytes were downloaded.
pub async fn download(
    sftp: &SftpSession,
    remote_path: &str,
    local_path: &Path,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<()> {
    let mut remote_file = sftp.open(remote_path).await?;
    let metadata = remote_file.metadata().await?;
    let total = metadata.size.unwrap_or(0);

    let mut local_file = LocalFile::create(local_path).await?;
    let mut buf = vec![0u8; 32 * 1024];
    let mut done = 0u64;

    loop {
        let n = remote_file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        local_file.write_all(&buf[..n]).await?;
        done += n as u64;
        on_progress(done, total);
    }

    Ok(())
}

/// Upload `local_path` to `remote_path`.
///
/// `on_progress` is a callback `(bytes_done, total_bytes)` which is called
/// every time new bytes were uploaded.
pub async fn upload(
    sftp: &SftpSession,
    local_path: &Path,
    remote_path: &str,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<()> {
    let mut local_file = LocalFile::open(local_path).await?;
    let total = local_file.metadata().await?.len();

    let mut remote_file = sftp
        .open_with_flags(
            remote_path,
            OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
        )
        .await?;

    let mut buf = vec![0u8; 32 * 1024];
    let mut done = 0u64;

    loop {
        let n = local_file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        remote_file.write_all(&buf[..n]).await?;
        done += n as u64;
        on_progress(done, total);
    }

    Ok(())
}

/// Utility function to run a transfer while updating the progress reporter.
pub async fn run_transfer(
    sftp: &SftpSession,
    direction: TransferDirection,
    local_path: &Path,
    remote_path: &str,
    reporter: ProgressReporter,
) -> Result<()> {
    match direction {
        TransferDirection::Download => {
            download(sftp, remote_path, local_path, |done, total| {
                reporter.update(done, total)
            })
            .await
        }
        TransferDirection::Upload => {
            upload(sftp, local_path, remote_path, |done, total| {
                reporter.update(done, total)
            })
            .await
        }
    }
}
