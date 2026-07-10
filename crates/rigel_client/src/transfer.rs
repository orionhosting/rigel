use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

/// The direction of a data transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
}

/// Shared progress state for a transfer.
///
/// - `update()` is called when bytes are moved
/// - `snapshot()` is called to get the current progress
#[derive(Clone)]
pub struct ProgressReporter {
    done: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
}

impl ProgressReporter {
    pub fn new() -> Self {
        Self {
            done: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Update the progress values.
    pub fn update(&self, done: u64, total: u64) {
        self.done.store(done, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
    }

    // Get the current progress.
    pub fn snapshot(&self) -> (u64, u64) {
        (
            self.done.load(Ordering::Relaxed),
            self.total.load(Ordering::Relaxed),
        )
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

/// The status of a transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferStatus {
    Queued,
    Running,
    Done,
    Failed,
}

impl TransferStatus {
    fn to_u8(self) -> u8 {
        match self {
            TransferStatus::Queued => 0,
            TransferStatus::Running => 1,
            TransferStatus::Done => 2,
            TransferStatus::Failed => 3,
        }
    }

    /// Returns [`TransferStatus::Failed`] if the value is invalid.
    fn from_u8(value: u8) -> Self {
        match value {
            0 => TransferStatus::Queued,
            1 => TransferStatus::Running,
            2 => TransferStatus::Done,
            _ => TransferStatus::Failed,
        }
    }
}

/// A current transfer status handle.
#[derive(Clone)]
pub struct StatusHandle(Arc<AtomicU8>);

impl StatusHandle {
    pub fn new(initial: TransferStatus) -> Self {
        Self(Arc::new(AtomicU8::new(initial.to_u8())))
    }

    /// Set the new status.
    pub fn set(&self, status: TransferStatus) {
        self.0.store(status.to_u8(), Ordering::Relaxed);
    }

    /// Get the status.
    pub fn get(&self) -> TransferStatus {
        TransferStatus::from_u8(self.0.load(Ordering::Relaxed))
    }
}

/// A single transfer job.
pub struct TransferJob {
    pub id: u64,
    pub direction: TransferDirection,
    pub local_path: PathBuf,
    pub remote_path: String,
    pub progress: ProgressReporter,
    pub status: StatusHandle,
}
