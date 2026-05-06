//! Playback API module for streams and manifests.

use super::client::CrunchyrollClient;
use super::types::{CRStreamData, CRStreamUrls, CRStreamVersion, CRSubtitle};
use crate::error::{ApiError, Error, Result};
use crate::utils::{format_bytes, format_elapsed, redact};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, trace};

/// Playback service base URL (cr-play-service).
const PLAYBACK_BASE: &str = "https://cr-play-service.prd.crunchyrollsvc.com";

/// Token activation base URL.
const TOKEN_BASE: &str = "https://cr-play-service.prd.crunchyrollsvc.com";

/// Stream endpoint types supported by Crunchyroll.
#[derive(Debug, Clone, Copy)]
pub enum StreamEndpoint {
    /// Web browser (Firefox) - good compatibility
    WebFirefox,
    /// Web browser (Chrome)
    WebChrome,
    /// Android TV - often has best quality
    TvAndroidTv,
    /// Android phone
    AndroidPhone,
    /// iOS
    IosIphone,
    /// Console (PS4)
    ConsolePs4,
    /// Console (PS5)
    ConsolePs5,
}

impl StreamEndpoint {
    /// Get the API path for this endpoint.
    pub fn as_str(&self) -> &'static str {
        match self {
            StreamEndpoint::WebFirefox => "web/firefox",
            StreamEndpoint::WebChrome => "web/chrome",
            StreamEndpoint::TvAndroidTv => "tv/android_tv",
            StreamEndpoint::AndroidPhone => "android/phone",
            StreamEndpoint::IosIphone => "ios/iphone",
            StreamEndpoint::ConsolePs4 => "console/ps4",
            StreamEndpoint::ConsolePs5 => "console/ps5",
        }
    }
}

impl Default for StreamEndpoint {
    fn default() -> Self {
        // Android TV usually provides the best streams
        StreamEndpoint::TvAndroidTv
    }
}

/// Playback response from the cr-play-service API.
/// This is a flat structure, NOT wrapped in a data array.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PlaybackResponse {
    /// Main manifest URL (MPD or M3U8)
    #[serde(default)]
    url: String,
    /// Playback token for license requests
    #[serde(default)]
    token: String,
    /// Asset ID
    #[serde(default)]
    asset_id: String,
    /// Audio locale of this stream
    #[serde(default)]
    audio_locale: String,
    /// Session info (not always present)
    #[serde(default)]
    session: Option<SessionInfo>,
    /// Subtitles (soft subs) - key is locale like "en-US"
    #[serde(default)]
    subtitles: HashMap<String, SubtitleEntry>,
    /// Closed captions
    #[serde(default)]
    captions: HashMap<String, SubtitleEntry>,
    /// Hard-coded subtitles (burned in)
    #[serde(default)]
    hard_subs: HashMap<String, HardSubEntry>,
    /// Available versions (different audio tracks)
    #[serde(default)]
    versions: Vec<VersionEntry>,
    /// BIF file URL (thumbnails/previews)
    #[serde(default)]
    bifs: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct SessionInfo {
    #[serde(default)]
    renew_seconds: Option<u64>,
    #[serde(default)]
    session_expiration_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SubtitleEntry {
    #[serde(default)]
    url: String,
    #[serde(default)]
    format: String,
    #[serde(default)]
    language: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HardSubEntry {
    /// Hardcoded subtitle locale
    #[serde(default)]
    hlang: String,
    /// Stream URL with hardcoded subs
    #[serde(default)]
    url: String,
    #[serde(default)]
    quality: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct VersionEntry {
    #[serde(default)]
    audio_locale: String,
    #[serde(default)]
    guid: String,
    #[serde(default)]
    media_guid: String,
    #[serde(default)]
    is_premium_only: bool,
    #[serde(default)]
    original: bool,
    #[serde(default)]
    season_guid: String,
}

impl CrunchyrollClient {
    /// Activate a playback token.
    ///
    /// This should be called before getting playback data for some content.
    pub async fn activate_token(&self, guid: &str, token: &str) -> Result<()> {
        let url = format!("{}/v1/token/{}/{}/inactive", TOKEN_BASE, guid, token);

        debug!("Activating playback token for {}", guid);
        trace!("Token: {}", redact(token));

        let start = Instant::now();
        self.patch(&url).await?;
        let elapsed = start.elapsed();

        trace!("Token activation completed in {}", format_elapsed(elapsed));

        Ok(())
    }

    /// Get playback data for an episode using default stream endpoint.
    pub async fn get_playback(&self, episode_guid: &str) -> Result<CRStreamData> {
        self.get_playback_with_endpoint(episode_guid, StreamEndpoint::default())
            .await
    }

    /// Get playback data for an episode with a specific stream endpoint.
    ///
    /// # Arguments
    /// * `episode_guid` - Episode GUID (the episode ID)
    /// * `endpoint` - Stream endpoint type (e.g., WebFirefox, TvAndroidTv)
    pub async fn get_playback_with_endpoint(
        &self,
        episode_guid: &str,
        endpoint: StreamEndpoint,
    ) -> Result<CRStreamData> {
        let url = format!(
            "{}/v2/{}/{}/play",
            PLAYBACK_BASE,
            episode_guid,
            endpoint.as_str()
        );

        debug!(
            "Fetching playback data for {} (endpoint: {})",
            episode_guid,
            endpoint.as_str()
        );

        let start = Instant::now();
        let response = self.get(&url).await?;
        let elapsed = start.elapsed();
        trace!("Playback API response received in {}", format_elapsed(elapsed));

        let playback: PlaybackResponse = response.json().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to parse playback response: {}",
                e
            )))
        })?;

        trace!(
            "Playback data: audio_locale={}, asset_id={}, versions={}",
            playback.audio_locale,
            playback.asset_id,
            playback.versions.len()
        );
        if !playback.subtitles.is_empty() {
            trace!(
                "Available subtitles: {:?}",
                playback.subtitles.keys().collect::<Vec<_>>()
            );
        }
        if !playback.versions.is_empty() {
            trace!(
                "Available versions: {:?}",
                playback.versions.iter().map(|v| &v.audio_locale).collect::<Vec<_>>()
            );
        }

        // Convert subtitles
        let subtitles: HashMap<String, CRSubtitle> = playback
            .subtitles
            .into_iter()
            .map(|(key, entry)| {
                (
                    key,
                    CRSubtitle {
                        locale: entry.language,
                        url: entry.url,
                        format: entry.format,
                    },
                )
            })
            .collect();

        // Convert captions (closed captions)
        let closed_captions: HashMap<String, CRSubtitle> = playback
            .captions
            .into_iter()
            .map(|(key, entry)| {
                (
                    key,
                    CRSubtitle {
                        locale: entry.language,
                        url: entry.url,
                        format: entry.format,
                    },
                )
            })
            .collect();

        // Convert versions
        let versions: Vec<CRStreamVersion> = playback
            .versions
            .into_iter()
            .map(|v| CRStreamVersion {
                audio_locale: v.audio_locale,
                guid: v.guid,
                media_guid: v.media_guid,
                is_premium_only: v.is_premium_only,
                original: v.original,
            })
            .collect();

        // Build URLs struct
        // The main URL from playback response is the manifest URL
        let urls = CRStreamUrls {
            url: playback.url.clone(),
            dash: playback.url.clone(), // Same URL - it's the manifest
            drm_dash: HashMap::new(),   // DRM info comes from manifest, not API
            hls: String::new(),
            drm_hls: HashMap::new(),
        };

        Ok(CRStreamData {
            media_id: playback.asset_id,
            audio_locale: playback.audio_locale,
            subtitles,
            closed_captions,
            versions,
            bifs: playback.bifs.map(|b| vec![b]).unwrap_or_default(),
            urls,
            // Store the token for license requests
            token: Some(playback.token),
        })
    }

    /// Get the MPD manifest content.
    pub async fn get_manifest(&self, manifest_url: &str) -> Result<String> {
        debug!("Fetching manifest: {}", manifest_url);

        let start = Instant::now();
        // Manifests can be fetched with or without auth depending on content
        // Try with auth first, fall back to anonymous
        let response = self.get(manifest_url).await.or_else(|_| {
            debug!("Manifest fetch with auth failed, trying anonymous");
            futures::executor::block_on(self.get_anonymous(manifest_url))
        })?;
        let elapsed = start.elapsed();

        let text = response.text().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to read manifest: {}",
                e
            )))
        })?;

        trace!(
            "Manifest fetched: {} in {}",
            format_bytes(text.len() as u64),
            format_elapsed(elapsed)
        );

        Ok(text)
    }
}
