//! Local filesystem [`OutputSink`] — moves finalized files into a
//! configured base directory.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;
use tokio::io::{self, AsyncWriteExt};

use crate::error::Result;

use super::sink::{OutputSink, OutputTarget};

/// Persists files to a directory on the local filesystem.
pub struct LocalFsSink {
    base_dir: PathBuf,
}

impl LocalFsSink {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Absolute path the given target resolves to under this sink.
    pub fn destination(&self, target: &OutputTarget) -> PathBuf {
        self.base_dir.join(target.relative_path())
    }
}

#[async_trait]
impl OutputSink for LocalFsSink {
    async fn publish(&self, source: &Path, target: &OutputTarget) -> Result<String> {
        let dest = self.destination(target);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        if source != dest {
            move_or_copy(source, &dest).await?;
        }

        let canonical = dest.canonicalize().unwrap_or_else(|_| dest.clone());
        Ok(format!("file://{}", canonical.display()))
    }
}

/// Move `source` to `dest`. Falls back to copy-then-remove when the two paths
/// live on different filesystems — `rename(2)` returns `EXDEV` (os error 18)
/// in that case, e.g. when the temp dir is on the local FS and the configured
/// output dir is a mounted NAS share.
async fn move_or_copy(source: &Path, dest: &Path) -> Result<()> {
    match fs::rename(source, dest).await {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) => {
            // Don't use `fs::copy` here — on macOS it goes through `copyfile()`
            // which tries to preserve ACLs / xattrs / flags, and SMB or NFS
            // mounts (e.g. TrueNAS shares) reject those operations with EPERM
            // (os error 1). Stream bytes manually instead.
            stream_copy(source, dest).await?;
            // Best-effort cleanup of the temp source; the file is already at dest.
            let _ = fs::remove_file(source).await;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Byte-by-byte copy with no metadata preservation. Safe for NAS mounts that
/// don't support `copyfile`-style ACL/xattr operations.
async fn stream_copy(source: &Path, dest: &Path) -> Result<()> {
    let mut reader = fs::File::open(source).await?;
    // Truncate any pre-existing dest from a prior failed attempt.
    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest)
        .await?;
    io::copy(&mut reader, &mut writer).await?;
    writer.flush().await?;
    // Drop reader/writer so close() runs before the caller observes success.
    drop(reader);
    drop(writer);
    Ok(())
}

fn is_cross_device(err: &std::io::Error) -> bool {
    // `EXDEV` is 18 on Linux and macOS. The stable `ErrorKind::CrossesDevices`
    // also matches, but we keep the raw check for portability across Rust
    // versions that don't have the variant.
    err.raw_os_error() == Some(18)
}
