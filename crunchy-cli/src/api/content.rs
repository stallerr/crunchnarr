//! Content API module for series, seasons, and episodes.

use super::client::{endpoints, CrunchyrollClient};
use super::types::{CRApiResponse, CREpisode, CRSearchItem, CRSearchResult, CRSeason, CRSeries};
use crate::error::{ApiError, DownloadError, Error, Result};
use serde::Deserialize;
use tracing::{debug, trace};

/// Search response wrapper.
#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Vec<SearchResultWrapper>,
}

#[derive(Debug, Deserialize)]
struct SearchResultWrapper {
    #[serde(rename = "type")]
    result_type: String,
    #[serde(default)]
    count: u32,
    items: Vec<CRSearchItem>,
}

impl From<SearchResultWrapper> for CRSearchResult {
    fn from(w: SearchResultWrapper) -> Self {
        CRSearchResult {
            result_type: w.result_type,
            count: w.count,
            items: w.items,
        }
    }
}

/// Content API operations.
impl CrunchyrollClient {
    /// Search for content.
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<CRSearchResult>> {
        let url = format!(
            "{}?q={}&n={}&type=series,movie_listing",
            self.url(endpoints::SEARCH),
            urlencoding::encode(query),
            limit
        );

        debug!("Searching for: {} (limit: {})", query, limit);

        let response = self.get(&url).await?;
        
        // Get raw text first for debugging
        let text = response.text().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to get search response text: {}",
                e
            )))
        })?;
        
        trace!("Raw search response: {}", text);
        
        let search_response: SearchResponse = serde_json::from_str(&text).map_err(|e| {
            // Log the raw response on parse failure for debugging
            debug!("Failed to parse search response. Raw JSON:\n{}", text);
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to parse search response: {}",
                e
            )))
        })?;

        let results: Vec<CRSearchResult> = search_response.data.into_iter().map(Into::into).collect();
        trace!(
            "Search returned {} result groups, {} total items",
            results.len(),
            results.iter().map(|r| r.items.len()).sum::<usize>()
        );

        Ok(results)
    }

    /// Get series information by ID.
    pub async fn get_series(&self, series_id: &str) -> Result<CRSeries> {
        let url = format!("{}/{}", self.url(endpoints::SERIES), series_id);

        debug!("Fetching series: {}", series_id);

        let response = self.get(&url).await?;
        let api_response: CRApiResponse<CRSeries> = response.json().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to parse series response: {}",
                e
            )))
        })?;

        let series = api_response.data.into_iter().next().ok_or_else(|| {
            Error::Download(DownloadError::NotFound(format!(
                "Series not found: {}",
                series_id
            )))
        })?;

        trace!(
            "Series: {} - {} seasons, {} episodes",
            series.title,
            series.season_count,
            series.episode_count
        );

        Ok(series)
    }

    /// Get all seasons for a series.
    pub async fn get_seasons(&self, series_id: &str) -> Result<Vec<CRSeason>> {
        let url = format!(
            "{}/{}/seasons?force_locale=&preferred_audio_language=ja-JP",
            self.url(endpoints::SERIES),
            series_id
        );

        debug!("Fetching seasons for series: {}", series_id);

        let response = self.get(&url).await?;
        
        // Get raw text first for debugging
        let text = response.text().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to get seasons response text: {}",
                e
            )))
        })?;
        
        trace!("Raw seasons response: {}", text);
        
        let api_response: CRApiResponse<CRSeason> = serde_json::from_str(&text).map_err(|e| {
            // Log the raw response on parse failure for debugging
            debug!("Failed to parse seasons response. Raw JSON:\n{}", text);
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to parse seasons response: {}",
                e
            )))
        })?;

        let seasons = api_response.data;
        trace!(
            "Found {} seasons: {:?}",
            seasons.len(),
            seasons.iter().map(|s| format!("S{} ({})", s.season_sequence_number, s.audio_locale)).collect::<Vec<_>>()
        );

        Ok(seasons)
    }

    /// Get all episodes for a season.
    pub async fn get_episodes(&self, season_id: &str) -> Result<Vec<CREpisode>> {
        let url = format!("{}/{}/episodes", self.url(endpoints::SEASONS), season_id);

        debug!("Fetching episodes for season: {}", season_id);

        let response = self.get(&url).await?;
        let api_response: CRApiResponse<CREpisode> = response.json().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to parse episodes response: {}",
                e
            )))
        })?;

        let episodes = api_response.data;
        trace!("Found {} episodes for season {}", episodes.len(), season_id);

        Ok(episodes)
    }

    /// Get a single episode by ID.
    pub async fn get_episode(&self, episode_id: &str) -> Result<CREpisode> {
        let url = format!("{}/{}", self.url(endpoints::EPISODES), episode_id);

        debug!("Fetching episode: {}", episode_id);

        let response = self.get(&url).await?;
        let api_response: CRApiResponse<CREpisode> = response.json().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to parse episode response: {}",
                e
            )))
        })?;

        let episode = api_response.data.into_iter().next().ok_or_else(|| {
            Error::Download(DownloadError::NotFound(format!(
                "Episode not found: {}",
                episode_id
            )))
        })?;

        let season_num = if episode.season_sequence_number > 0 { episode.season_sequence_number } else { episode.season_number };
        trace!(
            "Episode: {} - S{}E{} - {} ({})",
            episode.series_title,
            season_num,
            episode.episode,
            episode.title,
            episode.audio_locale
        );

        Ok(episode)
    }

    /// Get multiple episodes by their IDs.
    pub async fn get_episodes_by_ids(&self, episode_ids: &[String]) -> Result<Vec<CREpisode>> {
        if episode_ids.is_empty() {
            return Ok(vec![]);
        }

        let ids = episode_ids.join(",");
        let url = format!("{}?ids={}", self.url(endpoints::EPISODES), ids);

        debug!("Fetching {} episodes by ID", episode_ids.len());

        let response = self.get(&url).await?;
        let api_response: CRApiResponse<CREpisode> = response.json().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to parse episodes response: {}",
                e
            )))
        })?;

        Ok(api_response.data)
    }

    /// Get a season by ID.
    pub async fn get_season(&self, season_id: &str) -> Result<CRSeason> {
        let url = format!("{}/{}", self.url(endpoints::SEASONS), season_id);

        debug!("Fetching season: {}", season_id);

        let response = self.get(&url).await?;
        let api_response: CRApiResponse<CRSeason> = response.json().await.map_err(|e| {
            Error::Api(ApiError::InvalidResponse(format!(
                "Failed to parse season response: {}",
                e
            )))
        })?;

        api_response.data.into_iter().next().ok_or_else(|| {
            Error::Download(DownloadError::NotFound(format!(
                "Season not found: {}",
                season_id
            )))
        })
    }
}
