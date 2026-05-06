//! CLI command definitions and argument parsing.
//!
//! This module defines the command-line interface using `clap`.

mod commands;
pub mod filter;
mod url_parser;

pub use commands::{
    CacheArgs, CacheCommands, Cli, Commands, ConfigArgs, ConfigCommands, DownloadArgs, InfoArgs,
    LoginArgs, QueueArgs, QueueCommands, SearchArgs, VideoQuality,
};
pub use filter::EpisodeFilter;
pub use url_parser::{ContentType, CrunchyrollUrl};
