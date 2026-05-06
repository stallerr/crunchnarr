//! Crunchy-CLI: A command-line tool for downloading content from Crunchyroll
//!
//! This library provides the core functionality for the CLI application,
//! including API client, download management, media processing, and configuration.

pub mod api;
pub mod cli;
pub mod config;
pub mod download;
pub mod error;
pub mod media;
pub mod queue;
pub mod storage;
pub mod utils;

pub use error::{Error, Result};
