//! Crunchyroll URL parser.
//!
//! Parses Crunchyroll URLs and extracts content IDs and types.

use regex::Regex;
use std::sync::LazyLock;

use super::filter::EpisodeFilter;

/// Content type extracted from a Crunchyroll URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    /// A series (e.g., /series/GXYZ123/title)
    Series,
    /// A specific season of a series
    Season { season_number: u32 },
    /// An episode (e.g., /watch/EXYZ456/title)
    Episode,
    /// A music video or concert
    MusicVideo,
}

/// Parsed Crunchyroll URL with extracted information.
#[derive(Debug, Clone)]
pub struct CrunchyrollUrl {
    /// The content ID (e.g., "GXYZ123" or "EXYZ456")
    pub id: String,
    /// The type of content
    pub content_type: ContentType,
    /// The original URL or ID string
    pub original: String,
    /// Optional episode filter (e.g., [S1E4-S3])
    pub filter: Option<EpisodeFilter>,
}

// Regex patterns for URL parsing
static SERIES_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://(?:www\.)?crunchyroll\.com)?/series/([A-Z0-9]+)(?:/[^?]*)?(?:\?.*)?$")
        .unwrap()
});

static SEASON_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:https?://(?:www\.)?crunchyroll\.com)?/series/([A-Z0-9]+)(?:/[^?]*)?\?.*season=(\d+)",
    )
    .unwrap()
});

static EPISODE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://(?:www\.)?crunchyroll\.com)?/watch/([A-Z0-9]+)(?:/.*)?$").unwrap()
});

static MUSIC_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://(?:www\.)?crunchyroll\.com)?/watch/musicvideo/([A-Z0-9]+)(?:/.*)?$")
        .unwrap()
});

// Pattern to match bare IDs (e.g., "GXYZ123")
static BARE_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z0-9]{6,12}$").unwrap());

impl CrunchyrollUrl {
    /// Parse a Crunchyroll URL or bare ID, optionally with an episode filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use crunchy_cli::cli::CrunchyrollUrl;
    ///
    /// // Full URL
    /// let url = CrunchyrollUrl::parse("https://crunchyroll.com/series/GXYZ123/one-piece").unwrap();
    /// assert_eq!(url.id, "GXYZ123");
    ///
    /// // Bare ID (assumes series)
    /// let url = CrunchyrollUrl::parse("GXYZ123").unwrap();
    /// assert_eq!(url.id, "GXYZ123");
    ///
    /// // With episode filter
    /// let url = CrunchyrollUrl::parse("https://crunchyroll.com/series/GXYZ123/one-piece[S1E4-S3]").unwrap();
    /// assert_eq!(url.id, "GXYZ123");
    /// assert!(url.filter.is_some());
    /// ```
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();

        // Extract filter suffix if present (e.g., "[S1E4-S3]")
        let (base_input, filter) = EpisodeFilter::extract_from_url(input);
        let filter = filter.and_then(|f| EpisodeFilter::parse(f).ok());

        // Check for season URL first (more specific)
        if let Some(caps) = SEASON_REGEX.captures(base_input) {
            let id = caps.get(1)?.as_str().to_string();
            let season_number: u32 = caps.get(2)?.as_str().parse().ok()?;
            return Some(Self {
                id,
                content_type: ContentType::Season { season_number },
                original: input.to_string(),
                filter,
            });
        }

        // Check for music video
        if let Some(caps) = MUSIC_REGEX.captures(base_input) {
            return Some(Self {
                id: caps.get(1)?.as_str().to_string(),
                content_type: ContentType::MusicVideo,
                original: input.to_string(),
                filter,
            });
        }

        // Check for episode
        if let Some(caps) = EPISODE_REGEX.captures(base_input) {
            return Some(Self {
                id: caps.get(1)?.as_str().to_string(),
                content_type: ContentType::Episode,
                original: input.to_string(),
                filter,
            });
        }

        // Check for series URL
        if let Some(caps) = SERIES_REGEX.captures(base_input) {
            return Some(Self {
                id: caps.get(1)?.as_str().to_string(),
                content_type: ContentType::Series,
                original: input.to_string(),
                filter,
            });
        }

        // Check for bare ID (assume series)
        if BARE_ID_REGEX.is_match(base_input) {
            return Some(Self {
                id: base_input.to_string(),
                content_type: ContentType::Series,
                original: input.to_string(),
                filter,
            });
        }

        None
    }

    /// Check if this is a series URL/ID.
    pub fn is_series(&self) -> bool {
        matches!(
            self.content_type,
            ContentType::Series | ContentType::Season { .. }
        )
    }

    /// Check if this is an episode URL/ID.
    pub fn is_episode(&self) -> bool {
        matches!(self.content_type, ContentType::Episode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_series_url() {
        let url =
            CrunchyrollUrl::parse("https://www.crunchyroll.com/series/GXYZ123/one-piece").unwrap();
        assert_eq!(url.id, "GXYZ123");
        assert_eq!(url.content_type, ContentType::Series);
    }

    #[test]
    fn test_parse_series_url_no_slug() {
        let url = CrunchyrollUrl::parse("https://crunchyroll.com/series/GXYZ123").unwrap();
        assert_eq!(url.id, "GXYZ123");
        assert_eq!(url.content_type, ContentType::Series);
    }

    #[test]
    fn test_parse_season_url() {
        let url =
            CrunchyrollUrl::parse("https://crunchyroll.com/series/GXYZ123/one-piece?season=2")
                .unwrap();
        assert_eq!(url.id, "GXYZ123");
        assert_eq!(url.content_type, ContentType::Season { season_number: 2 });
    }

    #[test]
    fn test_parse_episode_url() {
        let url =
            CrunchyrollUrl::parse("https://crunchyroll.com/watch/EXYZ456/episode-title").unwrap();
        assert_eq!(url.id, "EXYZ456");
        assert_eq!(url.content_type, ContentType::Episode);
    }

    #[test]
    fn test_parse_bare_id() {
        let url = CrunchyrollUrl::parse("GXYZ123").unwrap();
        assert_eq!(url.id, "GXYZ123");
        assert_eq!(url.content_type, ContentType::Series);
    }

    #[test]
    fn test_parse_bare_id_long() {
        let url = CrunchyrollUrl::parse("G24H1NQEJ").unwrap();
        assert_eq!(url.id, "G24H1NQEJ");
    }

    #[test]
    fn test_parse_invalid() {
        assert!(CrunchyrollUrl::parse("https://example.com/test").is_none());
        assert!(CrunchyrollUrl::parse("invalid").is_none());
        assert!(CrunchyrollUrl::parse("").is_none());
    }

    #[test]
    fn test_is_series() {
        let series = CrunchyrollUrl::parse("GXYZ123").unwrap();
        assert!(series.is_series());

        let episode = CrunchyrollUrl::parse("https://crunchyroll.com/watch/EXYZ456/title").unwrap();
        assert!(!episode.is_series());
    }

    #[test]
    fn test_parse_with_filter() {
        let url =
            CrunchyrollUrl::parse("https://www.crunchyroll.com/series/GXYZ123/one-piece[S1E4-S3]")
                .unwrap();
        assert_eq!(url.id, "GXYZ123");
        assert_eq!(url.content_type, ContentType::Series);
        assert!(url.filter.is_some());

        let filter = url.filter.unwrap();
        assert!(filter.matches(1, 4)); // S1E4
        assert!(filter.matches(2, 1)); // S2
        assert!(!filter.matches(1, 3)); // Before S1E4
        assert!(!filter.matches(4, 1)); // After S3
    }

    #[test]
    fn test_parse_bare_id_with_filter() {
        let url = CrunchyrollUrl::parse("GXYZ123[E5]").unwrap();
        assert_eq!(url.id, "GXYZ123");
        assert!(url.filter.is_some());

        let filter = url.filter.unwrap();
        assert!(filter.matches(1, 5));
        assert!(filter.matches(2, 5));
        assert!(!filter.matches(1, 4));
    }

    #[test]
    fn test_parse_without_filter() {
        let url = CrunchyrollUrl::parse("https://crunchyroll.com/series/GXYZ123").unwrap();
        assert!(url.filter.is_none());
    }

    #[test]
    fn test_parse_complex_filter() {
        let url = CrunchyrollUrl::parse(
            "https://crunchyroll.com/series/GY8VEQ95Y/darling-in-the-franxx[E1-E5]",
        )
        .unwrap();
        assert_eq!(url.id, "GY8VEQ95Y");
        assert!(url.filter.is_some());

        let filter = url.filter.unwrap();
        assert!(filter.matches(1, 1));
        assert!(filter.matches(1, 5));
        assert!(!filter.matches(1, 6));
    }
}
