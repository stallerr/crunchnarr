//! Media processing module.
//!
//! Handles FFmpeg muxing, subtitle conversion, and external tool integration.

mod ffmpeg;
mod filename;
mod keycrak;
mod mp4decrypt;
mod subtitles;
mod tools;

pub use ffmpeg::FfmpegBuilder;
pub use filename::{FilenameGenerator, FilenameVars};
pub use keycrak::{acquire_keys, DecryptionKey};
pub use mp4decrypt::Mp4DecryptBuilder;
pub use subtitles::SubtitleConverter;
pub use tools::{ensure_ffmpeg, ToolValidator};
