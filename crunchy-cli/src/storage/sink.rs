//! Pluggable output destinations for completed downloads.
//!
//! After the download pipeline produces a final muxed file, it hands off to
//! an [`OutputSink`] which decides where the bytes ultimately live (local
//! disk, S3 bucket, etc.). The sink returns a canonical URI for the
//! published artifact (`file://`, `s3://`, ...).

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::Result;

/// Logical destination for a published file.
///
/// Sinks interpret these fields however suits their backend — a filesystem
/// sink renders `dirs/.../stem.ext` as a path under its base directory; an S3
/// sink uses the same components to build an object key.
#[derive(Debug, Clone)]
pub struct OutputTarget {
    /// Folder/key components, sanitized for the target system.
    /// e.g. `["My Series", "Season 01"]`.
    pub dirs: Vec<String>,
    /// Filename stem (no extension). e.g. `"S01E05 - Title"`.
    pub stem: String,
    /// File extension (no leading dot). e.g. `"mkv"`.
    pub ext: String,
}

impl OutputTarget {
    /// Render as a relative path: `{dirs}/{stem}.{ext}`.
    pub fn relative_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        for d in &self.dirs {
            path.push(d);
        }
        path.push(format!("{}.{}", self.stem, self.ext));
        path
    }

    /// Build a target from a relative path of the form `{dirs}/{stem}.{ext}`.
    /// Returns `None` if the path lacks a filename or extension.
    pub fn from_relative_path(relative: &Path) -> Option<Self> {
        let stem = relative.file_stem()?.to_str()?.to_string();
        let ext = relative.extension()?.to_str()?.to_string();
        let dirs: Vec<String> = relative
            .parent()
            .map(|parent| {
                parent
                    .components()
                    .filter_map(|c| match c {
                        std::path::Component::Normal(s) => s.to_str().map(|s| s.to_string()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(Self { dirs, stem, ext })
    }
}

/// Persistence strategy for a finalized download.
#[async_trait]
pub trait OutputSink: Send + Sync {
    /// Publish `source` to the sink's destination according to `target`.
    /// Returns a canonical URI for the published file.
    async fn publish(&self, source: &Path, target: &OutputTarget) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_round_trips() {
        let target = OutputTarget {
            dirs: vec!["Series".into(), "Season 01".into()],
            stem: "S01E05 - Title".into(),
            ext: "mkv".into(),
        };
        let rel = target.relative_path();
        assert_eq!(
            rel,
            PathBuf::from("Series/Season 01/S01E05 - Title.mkv")
        );

        let parsed = OutputTarget::from_relative_path(&rel).unwrap();
        assert_eq!(parsed.dirs, target.dirs);
        assert_eq!(parsed.stem, target.stem);
        assert_eq!(parsed.ext, target.ext);
    }

    #[test]
    fn handles_compound_stems() {
        // "Title.subtitles.mkv" → stem="Title.subtitles", ext="mkv"
        let target = OutputTarget::from_relative_path(Path::new("Show/Title.subtitles.mkv")).unwrap();
        assert_eq!(target.stem, "Title.subtitles");
        assert_eq!(target.ext, "mkv");
    }
}
