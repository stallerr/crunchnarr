//! Widevine CDM (Content Decryption Module) implementation in Rust
#![warn(missing_docs, clippy::todo, clippy::dbg_macro)]

pub mod device;

mod cdm;
mod error;
mod key;
mod mp4;
mod pssh;

pub use cdm::{Cdm, CdmLicenseRequest, CdmSession, LicenseType, ServiceCertificate};
pub use device::Device;
pub use error::Error;
pub use key::{Key, KeySet, KeyType};
pub use pssh::Pssh;
