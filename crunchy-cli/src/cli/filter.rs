//! Episode filter parsing for URL-based filtering.
//!
//! Supports filter patterns like:
//! - `[E5]` - Download episode 5 (any season)
//! - `[S1]` - Download all of season 1
//! - `[-S2]` - Download seasons 1-2
//! - `[S3E4-]` - Download from S3E4 onwards
//! - `[S1E4-S3]` - Download S1 from E4, then S2-S3
//! - `[S3,S5]` - Download seasons 3 and 5
//! - `[S1-S3,S4E2-S4E6]` - Complex ranges

use std::fmt;

/// A filter for selecting episodes based on season/episode patterns.
#[derive(Debug, Clone, Default)]
pub struct EpisodeFilter {
    /// Filter segments (OR'd together - episode matches if it matches any segment)
    pub segments: Vec<FilterSegment>,
}

/// A single segment of a filter (e.g., "S1E4-S3" or "E5").
#[derive(Debug, Clone)]
pub struct FilterSegment {
    /// Start point of the range
    pub start: FilterPoint,
    /// End point of the range (None = same as start for single point)
    pub end: Option<FilterPoint>,
    /// True if this is an open-ended range (e.g., "S3E4-" means from S3E4 onwards)
    pub is_open_end: bool,
}

/// A point in the filter (season and/or episode).
#[derive(Debug, Clone, Default)]
pub struct FilterPoint {
    /// Season number (None = any/all seasons)
    pub season: Option<u32>,
    /// Episode number (None = any/all episodes in season)
    pub episode: Option<u32>,
}

/// Error type for filter parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterParseError {
    pub message: String,
    pub input: String,
}

impl fmt::Display for FilterParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid filter '{}': {}", self.input, self.message)
    }
}

impl std::error::Error for FilterParseError {}

impl FilterPoint {
    /// Create a new filter point with optional season and episode.
    pub fn new(season: Option<u32>, episode: Option<u32>) -> Self {
        Self { season, episode }
    }

    /// Check if this point is empty (no season or episode specified).
    pub fn is_empty(&self) -> bool {
        self.season.is_none() && self.episode.is_none()
    }

    /// Compare two points for ordering.
    /// Returns Ordering based on (season, episode) tuple comparison.
    fn cmp_position(&self, season: u32, episode: u32) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match self.season {
            Some(s) => match s.cmp(&season) {
                Ordering::Equal => match self.episode {
                    Some(e) => e.cmp(&episode),
                    None => Ordering::Less, // No episode = start of season
                },
                other => other,
            },
            None => {
                // No season specified - only episode matters
                match self.episode {
                    Some(e) => e.cmp(&episode),
                    None => Ordering::Less, // Empty point = matches everything
                }
            }
        }
    }
}

impl FilterSegment {
    /// Check if a given (season, episode) matches this segment.
    pub fn matches(&self, season: u32, episode: u32) -> bool {
        // Handle episode-only filter (e.g., [E5] matches episode 5 in any season)
        if self.start.season.is_none() && self.end.is_none() && !self.is_open_end {
            if let Some(ep) = self.start.episode {
                return episode == ep;
            }
        }

        // Check if position is >= start
        let after_start = self.start.cmp_position(season, episode) != std::cmp::Ordering::Greater;

        // If this is an open-ended range (e.g., "S3E4-"), only check start
        if self.is_open_end {
            return after_start;
        }

        // Check if position is <= end (if end exists)
        let before_end = match &self.end {
            Some(end) => {
                // For end point, we need special handling
                match end.season {
                    Some(end_season) => {
                        if season < end_season {
                            true
                        } else if season == end_season {
                            // If end has no episode, include whole season
                            match end.episode {
                                Some(end_ep) => episode <= end_ep,
                                None => true,
                            }
                        } else {
                            false
                        }
                    }
                    None => {
                        // No season in end = episode-only end
                        match end.episode {
                            Some(end_ep) => episode <= end_ep,
                            None => true, // Unbounded
                        }
                    }
                }
            }
            None => {
                // No end and not open-ended = single point match
                match (self.start.season, self.start.episode) {
                    (Some(s), Some(e)) => season == s && episode == e,
                    (Some(s), None) => season == s,  // Whole season
                    (None, Some(e)) => episode == e, // Episode in any season
                    (None, None) => true,            // Match all
                }
            }
        };

        after_start && before_end
    }
}

impl EpisodeFilter {
    /// Create an empty filter (matches nothing).
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Check if the filter is empty.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Check if a given (season, episode) matches this filter.
    pub fn matches(&self, season: u32, episode: u32) -> bool {
        if self.segments.is_empty() {
            return true; // Empty filter matches all
        }
        self.segments.iter().any(|seg| seg.matches(season, episode))
    }

    /// Parse a filter string like "[S1E4-S3,E5]".
    ///
    /// # Grammar
    /// ```text
    /// filter     = "[" segment ("," segment)* "]"
    /// segment    = point ("-" point?)?
    /// point      = season? episode?
    /// season     = "S" number
    /// episode    = "E" number
    /// ```
    pub fn parse(input: &str) -> Result<Self, FilterParseError> {
        let input = input.trim();

        // Must start with [ and end with ]
        if !input.starts_with('[') || !input.ends_with(']') {
            return Err(FilterParseError {
                message: "Filter must be enclosed in brackets []".to_string(),
                input: input.to_string(),
            });
        }

        // Extract content between brackets
        let content = &input[1..input.len() - 1];

        if content.is_empty() {
            return Err(FilterParseError {
                message: "Filter cannot be empty".to_string(),
                input: input.to_string(),
            });
        }

        let mut segments = Vec::new();

        // Split by comma for multiple segments
        for segment_str in content.split(',') {
            let segment_str = segment_str.trim();
            if segment_str.is_empty() {
                continue;
            }

            let segment = Self::parse_segment(segment_str, input)?;
            segments.push(segment);
        }

        if segments.is_empty() {
            return Err(FilterParseError {
                message: "No valid segments found".to_string(),
                input: input.to_string(),
            });
        }

        Ok(Self { segments })
    }

    /// Parse a single segment like "S1E4-S3" or "E5" or "-S2".
    fn parse_segment(input: &str, original: &str) -> Result<FilterSegment, FilterParseError> {
        let input = input.trim();

        // Check for range separator
        if let Some(dash_pos) = input.find('-') {
            // Check if dash is at the start (e.g., "-S2")
            if dash_pos == 0 {
                let end_str = &input[1..];
                let end = Self::parse_point(end_str, original)?;
                return Ok(FilterSegment {
                    start: FilterPoint::default(), // From beginning
                    end: Some(end),
                    is_open_end: false,
                });
            }

            // Check if dash is at the end (e.g., "S3E4-")
            if dash_pos == input.len() - 1 {
                let start_str = &input[..dash_pos];
                let start = Self::parse_point(start_str, original)?;
                return Ok(FilterSegment {
                    start,
                    end: None,
                    is_open_end: true, // Open-ended range
                });
            }

            // Normal range (e.g., "S1E4-S3")
            let start_str = &input[..dash_pos];
            let end_str = &input[dash_pos + 1..];

            let start = Self::parse_point(start_str, original)?;
            let end = Self::parse_point(end_str, original)?;

            Ok(FilterSegment {
                start,
                end: Some(end),
                is_open_end: false,
            })
        } else {
            // Single point (e.g., "S1" or "E5" or "S1E4")
            let point = Self::parse_point(input, original)?;
            Ok(FilterSegment {
                start: point,
                end: None,
                is_open_end: false,
            })
        }
    }

    /// Parse a single point like "S1", "E5", or "S1E4".
    fn parse_point(input: &str, original: &str) -> Result<FilterPoint, FilterParseError> {
        let input = input.trim().to_uppercase();

        if input.is_empty() {
            return Ok(FilterPoint::default());
        }

        let mut season: Option<u32> = None;
        let mut episode: Option<u32> = None;
        let mut current_pos = 0;

        // Parse season if present
        if input[current_pos..].starts_with('S') {
            current_pos += 1;
            let num_end = input[current_pos..]
                .find(|c: char| !c.is_ascii_digit())
                .map(|p| current_pos + p)
                .unwrap_or(input.len());

            if num_end == current_pos {
                return Err(FilterParseError {
                    message: "Expected number after 'S'".to_string(),
                    input: original.to_string(),
                });
            }

            season = Some(
                input[current_pos..num_end]
                    .parse()
                    .map_err(|_| FilterParseError {
                        message: format!("Invalid season number: {}", &input[current_pos..num_end]),
                        input: original.to_string(),
                    })?,
            );
            current_pos = num_end;
        }

        // Parse episode if present
        if current_pos < input.len() && input[current_pos..].starts_with('E') {
            current_pos += 1;
            let num_end = input[current_pos..]
                .find(|c: char| !c.is_ascii_digit())
                .map(|p| current_pos + p)
                .unwrap_or(input.len());

            if num_end == current_pos {
                return Err(FilterParseError {
                    message: "Expected number after 'E'".to_string(),
                    input: original.to_string(),
                });
            }

            episode = Some(
                input[current_pos..num_end]
                    .parse()
                    .map_err(|_| FilterParseError {
                        message: format!(
                            "Invalid episode number: {}",
                            &input[current_pos..num_end]
                        ),
                        input: original.to_string(),
                    })?,
            );
            current_pos = num_end;
        }

        // Check for unexpected trailing characters
        if current_pos < input.len() {
            return Err(FilterParseError {
                message: format!("Unexpected characters: {}", &input[current_pos..]),
                input: original.to_string(),
            });
        }

        // Must have at least season or episode
        if season.is_none() && episode.is_none() {
            return Err(FilterParseError {
                message: format!("Invalid point: {}", input),
                input: original.to_string(),
            });
        }

        Ok(FilterPoint { season, episode })
    }

    /// Extract a filter suffix from a URL string.
    /// Returns (base_url, Option<filter_string>).
    ///
    /// # Examples
    /// ```
    /// use crunchy_cli::cli::filter::EpisodeFilter;
    ///
    /// let (base, filter) = EpisodeFilter::extract_from_url("https://crunchyroll.com/series/ABC123[S1E4-S3]");
    /// assert_eq!(base, "https://crunchyroll.com/series/ABC123");
    /// assert_eq!(filter, Some("[S1E4-S3]"));
    /// ```
    pub fn extract_from_url(url: &str) -> (&str, Option<&str>) {
        // Find the last '[' that has a matching ']' at the end
        if let Some(bracket_start) = url.rfind('[') {
            if url.ends_with(']') {
                let filter_part = &url[bracket_start..];
                let base_part = &url[..bracket_start];
                return (base_part, Some(filter_part));
            }
        }
        (url, None)
    }
}

impl fmt::Display for EpisodeFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", segment)?;
        }
        write!(f, "]")
    }
}

impl fmt::Display for FilterSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Handle open-start range
        if self.start.is_empty() {
            if let Some(ref end) = self.end {
                return write!(f, "-{}", end);
            }
        }

        write!(f, "{}", self.start)?;

        if let Some(ref end) = self.end {
            write!(f, "-{}", end)?;
        } else if self.is_open_end {
            // Open-ended range (e.g., "S3E4-")
            write!(f, "-")?;
        }

        Ok(())
    }
}

impl fmt::Display for FilterPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = self.season {
            write!(f, "S{}", s)?;
        }
        if let Some(e) = self.episode {
            write!(f, "E{}", e)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Parsing tests

    #[test]
    fn test_parse_single_episode() {
        let f = EpisodeFilter::parse("[E5]").unwrap();
        assert_eq!(f.segments.len(), 1);
        assert_eq!(f.segments[0].start.episode, Some(5));
        assert!(f.segments[0].start.season.is_none());
    }

    #[test]
    fn test_parse_single_season() {
        let f = EpisodeFilter::parse("[S1]").unwrap();
        assert_eq!(f.segments.len(), 1);
        assert_eq!(f.segments[0].start.season, Some(1));
        assert!(f.segments[0].start.episode.is_none());
    }

    #[test]
    fn test_parse_season_episode() {
        let f = EpisodeFilter::parse("[S1E4]").unwrap();
        assert_eq!(f.segments.len(), 1);
        assert_eq!(f.segments[0].start.season, Some(1));
        assert_eq!(f.segments[0].start.episode, Some(4));
    }

    #[test]
    fn test_parse_range_seasons() {
        let f = EpisodeFilter::parse("[S1-S3]").unwrap();
        assert_eq!(f.segments.len(), 1);
        assert_eq!(f.segments[0].start.season, Some(1));
        assert_eq!(f.segments[0].end.as_ref().unwrap().season, Some(3));
    }

    #[test]
    fn test_parse_open_start() {
        let f = EpisodeFilter::parse("[-S2]").unwrap();
        assert_eq!(f.segments.len(), 1);
        assert!(f.segments[0].start.is_empty());
        assert_eq!(f.segments[0].end.as_ref().unwrap().season, Some(2));
    }

    #[test]
    fn test_parse_open_end() {
        let f = EpisodeFilter::parse("[S3E4-]").unwrap();
        assert_eq!(f.segments.len(), 1);
        assert_eq!(f.segments[0].start.season, Some(3));
        assert_eq!(f.segments[0].start.episode, Some(4));
        assert!(f.segments[0].end.is_none());
    }

    #[test]
    fn test_parse_complex_range() {
        let f = EpisodeFilter::parse("[S1E4-S3]").unwrap();
        assert_eq!(f.segments.len(), 1);
        assert_eq!(f.segments[0].start.season, Some(1));
        assert_eq!(f.segments[0].start.episode, Some(4));
        assert_eq!(f.segments[0].end.as_ref().unwrap().season, Some(3));
    }

    #[test]
    fn test_parse_multiple_segments() {
        let f = EpisodeFilter::parse("[S3,S5]").unwrap();
        assert_eq!(f.segments.len(), 2);
        assert_eq!(f.segments[0].start.season, Some(3));
        assert_eq!(f.segments[1].start.season, Some(5));
    }

    #[test]
    fn test_parse_complex_multiple() {
        let f = EpisodeFilter::parse("[S1-S3,S4E2-S4E6]").unwrap();
        assert_eq!(f.segments.len(), 2);
    }

    #[test]
    fn test_parse_lowercase() {
        let f = EpisodeFilter::parse("[s1e4]").unwrap();
        assert_eq!(f.segments[0].start.season, Some(1));
        assert_eq!(f.segments[0].start.episode, Some(4));
    }

    #[test]
    fn test_parse_episode_range() {
        let f = EpisodeFilter::parse("[E1-E5]").unwrap();
        assert_eq!(f.segments.len(), 1);
        assert_eq!(f.segments[0].start.episode, Some(1));
        assert_eq!(f.segments[0].end.as_ref().unwrap().episode, Some(5));
    }

    // Error tests

    #[test]
    fn test_parse_no_brackets() {
        assert!(EpisodeFilter::parse("S1E4").is_err());
    }

    #[test]
    fn test_parse_empty_brackets() {
        assert!(EpisodeFilter::parse("[]").is_err());
    }

    #[test]
    fn test_parse_invalid_format() {
        assert!(EpisodeFilter::parse("[X1]").is_err());
        assert!(EpisodeFilter::parse("[S]").is_err());
        assert!(EpisodeFilter::parse("[E]").is_err());
    }

    // Matching tests

    #[test]
    fn test_match_single_episode() {
        let f = EpisodeFilter::parse("[E5]").unwrap();
        assert!(f.matches(1, 5)); // S1E5 matches
        assert!(f.matches(2, 5)); // S2E5 matches (any season)
        assert!(!f.matches(1, 4)); // S1E4 doesn't match
        assert!(!f.matches(1, 6)); // S1E6 doesn't match
    }

    #[test]
    fn test_match_single_season() {
        let f = EpisodeFilter::parse("[S1]").unwrap();
        assert!(f.matches(1, 1));
        assert!(f.matches(1, 10));
        assert!(f.matches(1, 99));
        assert!(!f.matches(2, 1));
    }

    #[test]
    fn test_match_specific_episode() {
        let f = EpisodeFilter::parse("[S1E4]").unwrap();
        assert!(f.matches(1, 4));
        assert!(!f.matches(1, 3));
        assert!(!f.matches(1, 5));
        assert!(!f.matches(2, 4));
    }

    #[test]
    fn test_match_season_range() {
        let f = EpisodeFilter::parse("[S1-S3]").unwrap();
        assert!(f.matches(1, 1));
        assert!(f.matches(2, 50));
        assert!(f.matches(3, 99));
        assert!(!f.matches(4, 1));
    }

    #[test]
    fn test_match_open_start() {
        let f = EpisodeFilter::parse("[-S2]").unwrap();
        assert!(f.matches(1, 1));
        assert!(f.matches(1, 99));
        assert!(f.matches(2, 1));
        assert!(f.matches(2, 99));
        assert!(!f.matches(3, 1));
    }

    #[test]
    fn test_match_open_end() {
        let f = EpisodeFilter::parse("[S3E4-]").unwrap();
        assert!(!f.matches(1, 1)); // Before S3
        assert!(!f.matches(3, 3)); // S3 but before E4
        assert!(f.matches(3, 4)); // Exactly S3E4
        assert!(f.matches(3, 99)); // S3 after E4
        assert!(f.matches(4, 1)); // Season after S3
        assert!(f.matches(99, 1)); // Any future season
    }

    #[test]
    fn test_match_complex_range() {
        let f = EpisodeFilter::parse("[S1E4-S3]").unwrap();
        assert!(!f.matches(1, 3)); // Before S1E4
        assert!(f.matches(1, 4)); // Exactly S1E4
        assert!(f.matches(1, 10)); // S1 after E4
        assert!(f.matches(2, 1)); // All of S2
        assert!(f.matches(3, 1)); // S3
        assert!(f.matches(3, 99)); // End of S3
        assert!(!f.matches(4, 1)); // After S3
    }

    #[test]
    fn test_match_multiple_seasons() {
        let f = EpisodeFilter::parse("[S3,S5]").unwrap();
        assert!(!f.matches(1, 1));
        assert!(!f.matches(2, 1));
        assert!(f.matches(3, 1));
        assert!(f.matches(3, 99));
        assert!(!f.matches(4, 1));
        assert!(f.matches(5, 1));
        assert!(!f.matches(6, 1));
    }

    #[test]
    fn test_match_episode_range() {
        let f = EpisodeFilter::parse("[E1-E5]").unwrap();
        assert!(f.matches(1, 1));
        assert!(f.matches(1, 5));
        assert!(f.matches(2, 3)); // Any season
        assert!(!f.matches(1, 6));
    }

    #[test]
    fn test_match_complex_multiple() {
        let f = EpisodeFilter::parse("[S1-S3,S4E2-S4E6]").unwrap();
        assert!(f.matches(1, 1)); // S1
        assert!(f.matches(2, 5)); // S2
        assert!(f.matches(3, 10)); // S3
        assert!(!f.matches(4, 1)); // S4E1 not in range
        assert!(f.matches(4, 2)); // S4E2
        assert!(f.matches(4, 6)); // S4E6
        assert!(!f.matches(4, 7)); // S4E7 not in range
        assert!(!f.matches(5, 1)); // S5 not in range
    }

    // URL extraction tests

    #[test]
    fn test_extract_from_url() {
        let (base, filter) =
            EpisodeFilter::extract_from_url("https://crunchyroll.com/series/ABC123[S1E4-S3]");
        assert_eq!(base, "https://crunchyroll.com/series/ABC123");
        assert_eq!(filter, Some("[S1E4-S3]"));
    }

    #[test]
    fn test_extract_from_url_no_filter() {
        let (base, filter) =
            EpisodeFilter::extract_from_url("https://crunchyroll.com/series/ABC123");
        assert_eq!(base, "https://crunchyroll.com/series/ABC123");
        assert!(filter.is_none());
    }

    #[test]
    fn test_extract_from_bare_id() {
        let (base, filter) = EpisodeFilter::extract_from_url("ABC123[E5]");
        assert_eq!(base, "ABC123");
        assert_eq!(filter, Some("[E5]"));
    }

    // Display tests

    #[test]
    fn test_display() {
        let f = EpisodeFilter::parse("[S1E4-S3,E5]").unwrap();
        let s = f.to_string();
        assert!(s.contains("S1E4"));
        assert!(s.contains("S3"));
        assert!(s.contains("E5"));
    }
}
