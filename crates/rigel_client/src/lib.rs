//! Rigel SFTP client core library.
//!
//! This crate has the types and utilities for connections,
//! file listings, transfers, saved host storage, etc.

pub mod error;
pub mod hosts;
pub mod known_hosts;
pub mod session;
pub mod sftp;
pub mod ssh;
pub mod transfer;

pub use error::{ClientError, Result};
pub use session::{LocalEntry, Session};
pub use sftp::RemoteEntry;
pub use ssh::{Auth, HostKeyPolicy};
