//! Adapter constants and snapshot data with its async IO.

use super::*;

pub(super) const ADAPTER_FORMAT_VERSION: u16 = 4;
pub(super) const LOCAL_DATABASE_DIRECTORY: &str = "raft-local-v4";
pub(super) const SNAPSHOT_OBJECT_DIRECTORY: &str = "snapshot-objects";
pub(super) const SNAPSHOT_SPOOL_DIRECTORY: &str = "snapshot-spool";
pub const APPLICATION_OBJECT_DIRECTORY: &str = "application-objects";
pub(super) const KEY_STATE_CONFIG: &[u8] = b"rrflow/raft/v4/state/config";
pub(super) const KEY_LOCAL_CONFIG: &[u8] = b"rrflow/raft/v4/local/config";
pub(super) const KEY_VOTE: &[u8] = b"rrflow/raft/v4/local/vote";
pub(super) const KEY_COMMITTED: &[u8] = b"rrflow/raft/v4/local/committed";
pub(super) const KEY_PURGED: &[u8] = b"rrflow/raft/v4/local/purged";
pub(super) const KEY_STATE: &[u8] = b"rrflow/raft/v4/state/current";
pub(super) const KEY_SNAPSHOT: &[u8] = b"rrflow/raft/v4/local/snapshot";
pub(super) const LOG_PREFIX: &[u8] = b"rrflow/raft/v4/local/log/";
pub(super) const SNAPSHOT_MEDIA_TYPE: &str = "application/vnd.rrflow.raft-state-snapshot.v4";
pub const RRFLOW_RAFT_REQUEST_RETENTION_LOGS: u64 = 4096;
// JSON is only the correctness-gate codec. Keep worst-case byte-array expansion
// comfortably below RRD LSM's 8 MiB value ceiling until a compact codec lands.
pub(super) const MAX_COMMAND_BYTES: usize = 1024 * 1024;
pub(super) static SNAPSHOT_SPOOL_ORDINAL: AtomicU64 = AtomicU64::new(1);

/// File-backed OpenRaft snapshot handle with bounded writes and explicit
/// ephemeral cleanup. Durable object handles opt out of deletion.
#[derive(Debug)]
pub struct RrdSnapshotData {
    // Declaration order is intentional: close the handle before cleanup tries
    // to unlink the ephemeral path, including on Windows.
    file: tokio::fs::File,
    path: PathBuf,
    position: u64,
    writable: bool,
    _cleanup: SnapshotCleanup,
}

#[derive(Debug)]
pub(super) struct SnapshotCleanup(pub(super) Option<AnchoredPath>);

impl Drop for SnapshotCleanup {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = path.remove_file();
        }
    }
}

impl RrdSnapshotData {
    pub(super) fn from_file(
        file: std::fs::File,
        path: PathBuf,
        cleanup: Option<AnchoredPath>,
    ) -> Self {
        Self {
            file: tokio::fs::File::from_std(file),
            _cleanup: SnapshotCleanup(cleanup),
            path,
            position: 0,
            writable: false,
        }
    }

    pub async fn create_ephemeral(path: impl AnchorInput) -> io::Result<Self> {
        let path = path.anchor()?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)?;
        let mut snapshot = Self::from_file(file, path.diagnostic_path().to_owned(), Some(path));
        snapshot.writable = true;
        Ok(snapshot)
    }

    pub(super) async fn independent_file(&self) -> io::Result<std::fs::File> {
        let handle = self.file.try_clone().await?.into_std().await;
        rrd_lsm::filesystem::reopen_read(&handle)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(super) async fn sync_all(&self) -> io::Result<()> {
        // Received spools must be flushed. Built and cached snapshots already
        // crossed their durable publication boundary and carry read-only
        // handles; Windows correctly refuses FlushFileBuffers on those.
        if self.writable {
            self.file.sync_all().await
        } else {
            Ok(())
        }
    }
}

impl AsyncRead for RrdSnapshotData {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buffer.filled().len();
        let result = Pin::new(&mut self.file).poll_read(context, buffer);
        if let Poll::Ready(Ok(())) = &result {
            self.position = self
                .position
                .saturating_add((buffer.filled().len() - before) as u64);
        }
        result
    }
}

impl AsyncWrite for RrdSnapshotData {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        if !self.writable {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "published snapshot is read-only",
            )));
        }
        if self
            .position
            .checked_add(buffer.len() as u64)
            .is_none_or(|end| end > SNAPSHOT_BUNDLE_MAX_BYTES)
        {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                format!("snapshot exceeds {SNAPSHOT_BUNDLE_MAX_BYTES} bytes"),
            )));
        }
        let result = Pin::new(&mut self.file).poll_write(context, buffer);
        if let Poll::Ready(Ok(written)) = result {
            self.position += written as u64;
            Poll::Ready(Ok(written))
        } else {
            result
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.file).poll_flush(context)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.file).poll_shutdown(context)
    }
}

impl AsyncSeek for RrdSnapshotData {
    fn start_seek(mut self: Pin<&mut Self>, position: std::io::SeekFrom) -> io::Result<()> {
        Pin::new(&mut self.file).start_seek(position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<u64>> {
        let result = Pin::new(&mut self.file).poll_complete(context);
        if let Poll::Ready(Ok(position)) = result {
            self.position = position;
            Poll::Ready(Ok(position))
        } else {
            result
        }
    }
}
