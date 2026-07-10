use std::sync::{Arc, Mutex};

use russh::client::{self, Handle};
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKey};
use russh_sftp::client::SftpSession;

use crate::error::{ClientError, Result};
use crate::known_hosts::{self, HostKeyCheck};

/// SSH authentication method.
#[derive(Clone, Debug)]
pub enum Auth {
    Password(String),
    /// Path to a private key file.
    KeyFile {
        path: String,
        /// Optional passphrase.
        passphrase: Option<String>,
    },
}

/// Host-key verification policy.
#[derive(Clone, Copy, Debug, Default)]
pub enum HostKeyPolicy {
    /// Unknown hosts are recorded and accepted, hosts
    /// with a different key than what's on file are rejected.
    #[default]
    TrustOnFirstUse,
    /// Accept any key with no verification.
    ///
    /// This is not recommended to use.
    AcceptAny,
}

/// The `check_server_key` method from the russh Handler cannot really
/// return a custom error, so we store it on the Handler itself.
type LastCheckResult = Arc<Mutex<Option<HostKeyCheck>>>;

struct Handler {
    policy: HostKeyPolicy,
    host: String,
    port: u16,
    last_check: LastCheckResult,
}

impl client::Handler for Handler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        match self.policy {
            HostKeyPolicy::AcceptAny => Ok(true),
            HostKeyPolicy::TrustOnFirstUse => {
                let check = known_hosts::check_and_record(&self.host, self.port, server_public_key)
                    .unwrap_or(HostKeyCheck::Mismatch {
                        recorded_fingerprint: "?".into(),
                        offered_fingerprint: "?".into(),
                    });

                let accept = matches!(check, HostKeyCheck::NewlyTrusted | HostKeyCheck::Matches);
                *self.last_check.lock().unwrap() = Some(check);
                Ok(accept)
            }
        }
    }
}

/// A SSH connection.
///
/// Use [`SshConnection::open_sftp`] to get the SFTP session.
pub struct SshConnection {
    handle: Handle<Handler>,
}

impl SshConnection {
    /// Try to connect.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        auth: &Auth,
        policy: HostKeyPolicy,
    ) -> Result<Self> {
        let config = Arc::new(client::Config::default());
        let last_check: LastCheckResult = Arc::new(Mutex::new(None));
        let handler = Handler {
            policy,
            host: host.to_string(),
            port,
            last_check: last_check.clone(),
        };

        let connect_result = client::connect(config, (host, port), handler).await;

        let mut handle = match connect_result {
            Ok(h) => h,
            Err(e) => {
                // If the handshake failed because we rejected the host key,
                // returns the reason which was stored in `last_check`
                if let Some(HostKeyCheck::Mismatch {
                    recorded_fingerprint,
                    offered_fingerprint,
                }) = last_check.lock().unwrap().take()
                {
                    return Err(ClientError::HostKeyMismatch {
                        host: format!("{host}:{port}"),
                        recorded_fingerprint,
                        offered_fingerprint,
                    });
                }
                return Err(e.into());
            }
        };

        let authenticated = match auth {
            Auth::Password(password) => handle
                .authenticate_password(user, password)
                .await?
                .success(),
            Auth::KeyFile { path, passphrase } => {
                let key_pair = russh::keys::load_secret_key(path, passphrase.as_deref())
                    .map_err(|e| ClientError::Config(format!("failed to load key {path}: {e}")))?;
                let key_with_alg =
                    PrivateKeyWithHashAlg::new(Arc::new(key_pair), Some(HashAlg::Sha256));
                handle
                    .authenticate_publickey(user, key_with_alg)
                    .await?
                    .success()
            }
        };

        if !authenticated {
            return Err(ClientError::AuthFailed(user.to_string()));
        }

        Ok(Self { handle })
    }

    /// Open the SFTP subsystem.
    pub async fn open_sftp(&self) -> Result<SftpSession> {
        let channel = self.handle.channel_open_session().await?;

        channel.request_subsystem(true, "sftp").await?;

        let sftp = SftpSession::new(channel.into_stream()).await?;
        Ok(sftp)
    }

    /// Send a command through the SSH and wait for the result.
    pub async fn exec(&self, command: &str) -> Result<String> {
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut output = Vec::new();
        while let Some(msg) = channel.wait().await {
            if let russh::ChannelMsg::Data { data } = msg {
                output.extend_from_slice(&data);
            }
        }
        Ok(String::from_utf8_lossy(&output).into_owned())
    }
}
