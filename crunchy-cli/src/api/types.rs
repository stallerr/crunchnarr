//! Crunchyroll API type definitions.
//!
//! These types mirror the Crunchyroll API responses.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// Deserialize a Vec that may be null in the JSON.
/// Returns an empty Vec if the value is null or missing.
fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt = Option::<Vec<T>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Supported language codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CRLanguage {
    #[serde(rename = "ja-JP")]
    Japanese,
    #[serde(rename = "en-US")]
    EnglishUS,
    #[serde(rename = "en-GB")]
    EnglishUK,
    #[serde(rename = "es-LA")]
    SpanishLA,
    #[serde(rename = "es-ES")]
    SpanishES,
    #[serde(rename = "pt-BR")]
    PortugueseBR,
    #[serde(rename = "pt-PT")]
    PortuguesePT,
    #[serde(rename = "fr-FR")]
    French,
    #[serde(rename = "de-DE")]
    German,
    #[serde(rename = "it-IT")]
    Italian,
    #[serde(rename = "ar-SA")]
    Arabic,
    #[serde(rename = "hi-IN")]
    Hindi,
    #[serde(rename = "zh-CN")]
    ChineseCN,
    #[serde(rename = "zh-TW")]
    ChineseTW,
    #[serde(rename = "ko-KR")]
    Korean,
    #[serde(rename = "id-ID")]
    Indonesian,
    #[serde(rename = "ms-MY")]
    Malay,
    #[serde(rename = "th-TH")]
    Thai,
    #[serde(rename = "vi-VN")]
    Vietnamese,
    #[serde(rename = "pl-PL")]
    Polish,
    #[serde(rename = "tr-TR")]
    Turkish,
}

impl CRLanguage {
    /// Returns all supported language codes as strings.
    pub fn all_codes() -> Vec<String> {
        vec![
            "ja-JP", "en-US", "en-GB", "es-LA", "es-ES", "pt-BR", "pt-PT",
            "fr-FR", "de-DE", "it-IT", "ar-SA", "hi-IN", "zh-CN",
            "zh-TW", "ko-KR", "id-ID", "ms-MY", "th-TH", "vi-VN", "pl-PL",
            "tr-TR",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    /// Get the language code string (e.g., "ja-JP").
    pub fn code(&self) -> &'static str {
        match self {
            CRLanguage::Japanese => "ja-JP",
            CRLanguage::EnglishUS => "en-US",
            CRLanguage::EnglishUK => "en-GB",
            CRLanguage::SpanishLA => "es-LA",
            CRLanguage::SpanishES => "es-ES",
            CRLanguage::PortugueseBR => "pt-BR",
            CRLanguage::PortuguesePT => "pt-PT",
            CRLanguage::French => "fr-FR",
            CRLanguage::German => "de-DE",
            CRLanguage::Italian => "it-IT",
            CRLanguage::Arabic => "ar-SA",
            CRLanguage::Hindi => "hi-IN",
            CRLanguage::ChineseCN => "zh-CN",
            CRLanguage::ChineseTW => "zh-TW",
            CRLanguage::Korean => "ko-KR",
            CRLanguage::Indonesian => "id-ID",
            CRLanguage::Malay => "ms-MY",
            CRLanguage::Thai => "th-TH",
            CRLanguage::Vietnamese => "vi-VN",
            CRLanguage::Polish => "pl-PL",
            CRLanguage::Turkish => "tr-TR",
        }
    }

    /// Get human-readable language name.
    pub fn name(&self) -> &'static str {
        match self {
            CRLanguage::Japanese => "Japanese",
            CRLanguage::EnglishUS => "English (US)",
            CRLanguage::EnglishUK => "English (UK)",
            CRLanguage::SpanishLA => "Spanish (Latin America)",
            CRLanguage::SpanishES => "Spanish (Spain)",
            CRLanguage::PortugueseBR => "Portuguese (Brazil)",
            CRLanguage::PortuguesePT => "Portuguese (Portugal)",
            CRLanguage::French => "French",
            CRLanguage::German => "German",
            CRLanguage::Italian => "Italian",
            CRLanguage::Arabic => "Arabic",
            CRLanguage::Hindi => "Hindi",
            CRLanguage::ChineseCN => "Chinese (Simplified)",
            CRLanguage::ChineseTW => "Chinese (Traditional)",
            CRLanguage::Korean => "Korean",
            CRLanguage::Indonesian => "Indonesian",
            CRLanguage::Malay => "Malay",
            CRLanguage::Thai => "Thai",
            CRLanguage::Vietnamese => "Vietnamese",
            CRLanguage::Polish => "Polish",
            CRLanguage::Turkish => "Turkish",
        }
    }

    /// Parse from string language code.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "ja-JP" => Some(CRLanguage::Japanese),
            "en-US" => Some(CRLanguage::EnglishUS),
            "en-GB" => Some(CRLanguage::EnglishUK),
            "es-LA" | "es-419" => Some(CRLanguage::SpanishLA),
            "es-ES" => Some(CRLanguage::SpanishES),
            "pt-BR" => Some(CRLanguage::PortugueseBR),
            "pt-PT" => Some(CRLanguage::PortuguesePT),
            "fr-FR" => Some(CRLanguage::French),
            "de-DE" => Some(CRLanguage::German),
            "it-IT" => Some(CRLanguage::Italian),
            "ar-SA" | "ar-ME" => Some(CRLanguage::Arabic),
            "hi-IN" => Some(CRLanguage::Hindi),
            "zh-CN" => Some(CRLanguage::ChineseCN),
            "zh-TW" => Some(CRLanguage::ChineseTW),
            "ko-KR" => Some(CRLanguage::Korean),
            "id-ID" => Some(CRLanguage::Indonesian),
            "ms-MY" => Some(CRLanguage::Malay),
            "th-TH" => Some(CRLanguage::Thai),
            "vi-VN" => Some(CRLanguage::Vietnamese),
            "pl-PL" => Some(CRLanguage::Polish),
            "tr-TR" => Some(CRLanguage::Turkish),
            _ => None,
        }
    }
}

impl std::fmt::Display for CRLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Authentication token response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub scope: String,
    pub country: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
}

/// User profile information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRProfile {
    pub username: String,
    pub email: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub preferred_communication_language: String,
    #[serde(default)]
    pub preferred_content_subtitle_language: String,
    #[serde(default)]
    pub preferred_content_audio_language: String,
}

/// Series information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSeries {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub slug_title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub season_count: u32,
    #[serde(default)]
    pub episode_count: u32,
    #[serde(default)]
    pub is_simulcast: bool,
    #[serde(default)]
    pub is_mature: bool,
    #[serde(default)]
    pub maturity_ratings: Vec<String>,
    #[serde(default)]
    pub content_provider: String,
    #[serde(default)]
    pub images: CRImages,
}

/// Season information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSeason {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub slug_title: String,
    #[serde(default)]
    pub series_id: String,
    #[serde(default)]
    pub season_number: u32,
    #[serde(default)]
    pub season_sequence_number: u32,
    #[serde(default)]
    pub number_of_episodes: u32,
    #[serde(default)]
    pub is_subbed: bool,
    #[serde(default)]
    pub is_dubbed: bool,
    #[serde(default)]
    pub is_simulcast: bool,
    #[serde(default)]
    pub audio_locale: String,
    #[serde(default)]
    pub audio_locales: Vec<String>,
    #[serde(default)]
    pub subtitle_locales: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub versions: Vec<CRSeasonVersion>,
}

/// Season version information (for different dubs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSeasonVersion {
    pub audio_locale: String,
    pub guid: String,
    #[serde(default)]
    pub is_premium_only: bool,
    #[serde(default)]
    pub original: bool,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub season_guid: String,
}

/// Episode information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CREpisode {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub slug_title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub series_id: String,
    #[serde(default)]
    pub series_title: String,
    #[serde(default)]
    pub series_slug_title: String,
    #[serde(default)]
    pub season_id: String,
    #[serde(default)]
    pub season_title: String,
    #[serde(default)]
    pub season_number: u32,
    #[serde(default)]
    pub season_sequence_number: u32,
    #[serde(default)]
    pub episode: String,
    #[serde(default)]
    pub episode_number: Option<f32>,
    #[serde(default)]
    pub sequence_number: f32,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub is_premium_only: bool,
    #[serde(default)]
    pub is_subbed: bool,
    #[serde(default)]
    pub is_dubbed: bool,
    #[serde(default)]
    pub is_mature: bool,
    #[serde(default)]
    pub audio_locale: String,
    #[serde(default)]
    pub subtitle_locales: Vec<String>,
    #[serde(default)]
    pub versions: Vec<CREpisodeVersion>,
    #[serde(default)]
    pub streams_link: String,
    #[serde(default)]
    pub images: CRImages,
    #[serde(default)]
    pub episode_air_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub premium_available_date: Option<DateTime<Utc>>,
}

/// Episode version (different audio tracks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CREpisodeVersion {
    pub audio_locale: String,
    pub guid: String,
    pub media_guid: String,
    #[serde(default)]
    pub is_premium_only: bool,
    #[serde(default)]
    pub original: bool,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub season_guid: String,
}

/// Images container.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CRImages {
    #[serde(default)]
    pub poster_tall: Vec<Vec<CRImage>>,
    #[serde(default)]
    pub poster_wide: Vec<Vec<CRImage>>,
    #[serde(default)]
    pub thumbnail: Vec<Vec<CRImage>>,
}

/// Single image entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRImage {
    pub source: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(rename = "type")]
    #[serde(default)]
    pub image_type: String,
}

/// Stream/playback data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRStreamData {
    #[serde(default)]
    pub media_id: String,
    #[serde(default)]
    pub audio_locale: String,
    #[serde(default)]
    pub subtitles: HashMap<String, CRSubtitle>,
    #[serde(default)]
    pub closed_captions: HashMap<String, CRSubtitle>,
    #[serde(default)]
    pub versions: Vec<CRStreamVersion>,
    #[serde(default)]
    pub bifs: Vec<String>,
    #[serde(flatten)]
    pub urls: CRStreamUrls,
    /// Playback token for license requests (DRM).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Stream URLs in different formats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CRStreamUrls {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub dash: String,
    #[serde(default)]
    pub drm_dash: HashMap<String, String>,
    #[serde(default)]
    pub hls: String,
    #[serde(default)]
    pub drm_hls: HashMap<String, String>,
}

/// Stream version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRStreamVersion {
    pub audio_locale: String,
    pub guid: String,
    pub media_guid: String,
    #[serde(default)]
    pub is_premium_only: bool,
    #[serde(default)]
    pub original: bool,
}

/// Subtitle information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSubtitle {
    pub locale: String,
    pub url: String,
    #[serde(default)]
    pub format: String,
}

/// Search result item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSearchResult {
    #[serde(rename = "type")]
    pub result_type: String,
    #[serde(default)]
    pub count: u32,
    pub items: Vec<CRSearchItem>,
}

/// Individual search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSearchItem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub slug_title: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub images: CRImages,
}

/// Generic API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRApiResponse<T> {
    #[serde(default)]
    pub total: u32,
    pub data: Vec<T>,
    #[serde(default)]
    pub meta: CRMeta,
}

/// Response metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CRMeta {
    #[serde(default)]
    pub total_before_filter: u32,
    #[serde(default)]
    pub total_count: u32,
}

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRApiError {
    pub error: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub context: Vec<CRErrorContext>,
}

/// Error context information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRErrorContext {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub field: String,
    #[serde(default)]
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_code() {
        assert_eq!(CRLanguage::Japanese.code(), "ja-JP");
        assert_eq!(CRLanguage::EnglishUS.code(), "en-US");
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(CRLanguage::from_code("ja-JP"), Some(CRLanguage::Japanese));
        assert_eq!(CRLanguage::from_code("invalid"), None);
    }

    #[test]
    fn test_language_display() {
        assert_eq!(format!("{}", CRLanguage::Japanese), "ja-JP");
    }

    #[test]
    fn test_deserialize_token() {
        let json = r#"{
            "access_token": "test_access",
            "refresh_token": "test_refresh",
            "expires_in": 3600,
            "token_type": "Bearer",
            "scope": "account",
            "country": "US"
        }"#;
        let token: CRToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.access_token, "test_access");
        assert_eq!(token.expires_in, 3600);
    }
}
