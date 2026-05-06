//! Pluggable storage backends for download outputs.

pub mod local;
pub mod sink;

pub use local::LocalFsSink;
pub use sink::{OutputSink, OutputTarget};
