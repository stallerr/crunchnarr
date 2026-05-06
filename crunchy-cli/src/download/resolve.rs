//! Episode resolution logic.
//!
//! Resolves Crunchyroll URLs to lists of episodes for downloading.

use crate::api::types::CREpisode;
use crate::api::CrunchyrollClient;
use crate::cli::{ContentType, CrunchyrollUrl, EpisodeFilter};
use crate::error::Result;
use tracing::info;

/// Resolve a parsed Crunchyroll URL to a list of episodes.
pub async fn resolve_episodes(
    client: &CrunchyrollClient,
    url: &CrunchyrollUrl,
) -> Result<Vec<CREpisode>> {
    match &url.content_type {
        ContentType::Episode | ContentType::MusicVideo => {
            let episode = client.get_episode(&url.id).await?;
            Ok(vec![episode])
        }
        ContentType::Series | ContentType::Season { .. } => {
            resolve_series_episodes(client, &url.id, url.filter.as_ref()).await
        }
    }
}

/// Resolve a series ID to a list of episodes, optionally filtered.
///
/// If a URL-based episode filter is provided (e.g., `[S1E4-S3]`), only matching
/// episodes are returned. Otherwise, all episodes from the series are returned.
pub async fn resolve_series_episodes(
    client: &CrunchyrollClient,
    series_id: &str,
    url_filter: Option<&EpisodeFilter>,
) -> Result<Vec<CREpisode>> {
    if let Some(filter) = url_filter {
        info!("Using URL episode filter: {}", filter);
    }

    let series = client.get_series(series_id).await?;
    info!(
        "Series: {} ({} seasons)",
        series.title, series.season_count
    );

    let seasons = client.get_seasons(series_id).await?;
    let mut all_episodes = Vec::new();

    for season in &seasons {
        info!(
            "Fetching Season {} - {} ({} episodes)",
            season.season_sequence_number, season.title, season.number_of_episodes
        );

        let episodes = client.get_episodes(&season.id).await?;

        for episode in episodes {
            if let Some(filter) = url_filter {
                let ep_num = episode
                    .episode_number
                    .map(|n| n as u32)
                    .or_else(|| episode.episode.parse().ok())
                    .unwrap_or(0);

                if !filter.matches(season.season_sequence_number, ep_num) {
                    continue;
                }
            }

            all_episodes.push(episode);
        }
    }

    // Sort by season then episode number
    all_episodes.sort_by(|a, b| {
        let sa = if a.season_sequence_number > 0 {
            a.season_sequence_number
        } else {
            a.season_number
        };
        let sb = if b.season_sequence_number > 0 {
            b.season_sequence_number
        } else {
            b.season_number
        };
        if sa != sb {
            return sa.cmp(&sb);
        }
        let ea = a.episode_number.unwrap_or(0.0);
        let eb = b.episode_number.unwrap_or(0.0);
        ea.partial_cmp(&eb).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(all_episodes)
}
