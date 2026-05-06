//! FFmpeg command builder.

use crate::error::{Error, MediaError, Result};
use crate::utils::format_elapsed;
use std::time::Instant;
use tracing::{debug, trace};

/// Builds FFmpeg commands for muxing.
pub struct FfmpegBuilder {
    ffmpeg_path: String,
    inputs: Vec<InputFile>,
    output_path: String,
    output_format: String,
    /// Global metadata (key, value).
    metadata: Vec<(String, String)>,
    /// Per-stream metadata (stream_spec, key, value).
    stream_metadata: Vec<(String, String, String)>,
    maps: Vec<StreamMap>,
    dispositions: Vec<StreamDisposition>,
    subtitle_codecs: Vec<(usize, String)>,
}

/// Input file for FFmpeg.
struct InputFile {
    path: String,
    options: Vec<String>,
}

/// Stream mapping.
struct StreamMap {
    input_index: usize,
    stream_type: Option<char>,
    stream_index: Option<usize>,
    output_options: Vec<String>,
}

/// Stream disposition flags (default, forced, hearing_impaired, etc.).
struct StreamDisposition {
    /// Stream specifier (e.g., "a:0", "s:1").
    stream_spec: String,
    /// Disposition flags (e.g., ["default"], ["default", "forced"]).
    flags: Vec<String>,
}

impl FfmpegBuilder {
    /// Create a new FFmpeg builder.
    pub fn new(ffmpeg_path: &str, output_path: &str) -> Self {
        Self {
            ffmpeg_path: ffmpeg_path.to_string(),
            inputs: Vec::new(),
            output_path: output_path.to_string(),
            output_format: "mkv".to_string(),
            metadata: Vec::new(),
            stream_metadata: Vec::new(),
            maps: Vec::new(),
            dispositions: Vec::new(),
            subtitle_codecs: Vec::new(),
        }
    }

    /// Set output format (mkv, mp4).
    pub fn format(mut self, format: &str) -> Self {
        self.output_format = format.to_string();
        self
    }

    /// Add an input file.
    pub fn input(mut self, path: &str) -> Self {
        self.inputs.push(InputFile {
            path: path.to_string(),
            options: Vec::new(),
        });
        self
    }

    /// Add an input file with options.
    pub fn input_with_options(mut self, path: &str, options: &[&str]) -> Self {
        self.inputs.push(InputFile {
            path: path.to_string(),
            options: options.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Map all streams from an input.
    pub fn map_all(mut self, input_index: usize) -> Self {
        self.maps.push(StreamMap {
            input_index,
            stream_type: None,
            stream_index: None,
            output_options: Vec::new(),
        });
        self
    }

    /// Map a specific stream.
    pub fn map_stream(
        mut self,
        input_index: usize,
        stream_type: char,
        stream_index: usize,
    ) -> Self {
        self.maps.push(StreamMap {
            input_index,
            stream_type: Some(stream_type),
            stream_index: Some(stream_index),
            output_options: Vec::new(),
        });
        self
    }

    /// Add global metadata.
    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.push((key.to_string(), value.to_string()));
        self
    }

    /// Add stream metadata.
    ///
    /// Stream metadata is applied using FFmpeg's `-metadata:stream_spec key=value` format.
    /// For example: `-metadata:s:a:0 language=jpn` or `-metadata:s:s:0 title=English`
    pub fn stream_metadata(mut self, stream_spec: &str, key: &str, value: &str) -> Self {
        self.stream_metadata.push((
            stream_spec.to_string(),
            key.to_string(),
            value.to_string(),
        ));
        self
    }

    /// Set disposition flags for a stream.
    ///
    /// # Arguments
    /// * `stream_spec` - Stream specifier (e.g., "a:0" for first audio, "s:1" for second subtitle)
    /// * `flags` - Disposition flags (e.g., &["default"], &["default", "forced"], &["hearing_impaired"])
    ///
    /// Multiple flags will be joined with "+" (e.g., "default+forced").
    pub fn disposition(mut self, stream_spec: &str, flags: &[&str]) -> Self {
        self.dispositions.push(StreamDisposition {
            stream_spec: stream_spec.to_string(),
            flags: flags.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Set codec for a specific subtitle stream.
    ///
    /// # Arguments
    /// * `index` - Subtitle stream index (0-based)
    /// * `codec` - Codec name (e.g., "ass", "srt")
    pub fn subtitle_codec(mut self, index: usize, codec: &str) -> Self {
        self.subtitle_codecs.push((index, codec.to_string()));
        self
    }

    /// Build the FFmpeg command arguments.
    pub fn build(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Global options
        args.push("-y".to_string()); // Overwrite output
        args.push("-hide_banner".to_string());
        args.push("-loglevel".to_string());
        args.push("warning".to_string());

        // Input files
        for input in &self.inputs {
            for opt in &input.options {
                args.push(opt.clone());
            }
            args.push("-i".to_string());
            args.push(input.path.clone());
        }

        // Stream mappings
        for map in &self.maps {
            args.push("-map".to_string());
            let map_str = if let (Some(t), Some(i)) = (map.stream_type, map.stream_index) {
                format!("{}:{}:{}", map.input_index, t, i)
            } else {
                format!("{}?", map.input_index)
            };
            args.push(map_str);

            for opt in &map.output_options {
                args.push(opt.clone());
            }
        }

        // Codec options - copy video and audio
        args.push("-c:v".to_string());
        args.push("copy".to_string());
        args.push("-c:a".to_string());
        args.push("copy".to_string());

        // Per-subtitle-stream codecs (must come before generic -c:s)
        for (index, codec) in &self.subtitle_codecs {
            args.push(format!("-c:s:{}", index));
            args.push(codec.clone());
        }

        // If no specific subtitle codecs set, default to copy
        if self.subtitle_codecs.is_empty() {
            args.push("-c:s".to_string());
            args.push("copy".to_string());
        }

        // Stream dispositions
        for disp in &self.dispositions {
            args.push(format!("-disposition:{}", disp.stream_spec));
            args.push(disp.flags.join("+"));
        }

        // Global metadata
        for (key, value) in &self.metadata {
            args.push("-metadata".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Per-stream metadata (uses -metadata:stream_spec key=value format)
        for (stream_spec, key, value) in &self.stream_metadata {
            args.push(format!("-metadata:{}", stream_spec));
            args.push(format!("{}={}", key, value));
        }

        // Output format and path
        args.push("-f".to_string());
        args.push(if self.output_format == "mkv" {
            "matroska".to_string()
        } else {
            self.output_format.clone()
        });
        args.push(self.output_path.clone());

        args
    }

    /// Execute the FFmpeg command.
    pub async fn execute(&self) -> Result<()> {
        let args = self.build();
        debug!("FFmpeg args: {:?}", args);
        trace!("FFmpeg command: {} {}", self.ffmpeg_path, args.join(" "));

        let start = Instant::now();
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = super::tools::execute_command(&self.ffmpeg_path, &args_ref, 3600).await?;
        let elapsed = start.elapsed();

        if !output.success {
            trace!("FFmpeg stderr: {}", output.stderr);
            return Err(Error::Media(MediaError::MuxingFailed(format!(
                "FFmpeg failed with code {}: {}",
                output.code, output.stderr
            ))));
        }

        debug!("FFmpeg muxing completed in {}", format_elapsed(elapsed));
        trace!("FFmpeg stdout: {}", output.stdout);

        Ok(())
    }
}
