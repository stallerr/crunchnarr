//! Crunchyroll API client module.
//!
//! This module provides the API client for interacting with Crunchyroll's services.

mod auth;
mod client;
mod content;
mod playback;
pub mod token_store;
pub mod types;

pub use client::CrunchyrollClient;
pub use playback::*;
pub use token_store::{FileTokenStore, TokenStore, Tokens};
pub use types::*;
