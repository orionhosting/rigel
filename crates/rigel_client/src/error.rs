use thiserror::Error;

/// All errors that can happen in the client.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("ssh error: {0}")]
    Ssh(#[from] russh::Error),

    #[error("sftp error: {0}")]
    Sftp(#[from] russh_sftp::client::error::Error),

    #[error("authentication failed for user '{0}'")]
    AuthFailed(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error(
        "host key for {host} does not match the one on record. Possible server change or MITM. recorded: {recorded_fingerprint}, offered: {offered_fingerprint}. If you trust this change, remove the old entry and reconnect."
    )]
    HostKeyMismatch {
        host: String,
        recorded_fingerprint: String,
        offered_fingerprint: String,
    },

    #[error("no active connection")]
    NotConnected,

    #[error("path has no parent: {0}")]
    NoParent(String),
}

/// A specialized Result type for the library.
pub type Result<T> = std::result::Result<T, ClientError>;
