//! Segment downloader for DASH streams.
//!
//! Supports parallel downloads and caching for resumable downloads.

use crate::download::cache::{compute_file_hash, verify_checksum, StreamCache};
use crate::download::throttle::Throttle;
use crate::error::{DownloadError, Error, Result};
use crate::utils::{format_bytes, format_elapsed};
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, trace, warn};

/// Downloads and concatenates DASH segments with parallel download support.
pub struct SegmentDownloader {
    client: Client,
    max_concurrent: usize,
    throttle: Throttle,
}

/// Result of downloading a single segment.
#[derive(Debug)]
#[allow(dead_code)]
struct SegmentResult {
    index: usize,
    path: PathBuf,
    size: u64,
    checksum: String,
}

impl SegmentDownloader {
    /// Create a new segment downloader.
    pub fn new(client: Client, max_concurrent: usize, max_speed_kbps: u32) -> Self {
        let rate_bps = if max_speed_kbps > 0 {
            Some(max_speed_kbps as u64 * 1024)
        } else {
            None
        };

        let throttle = Throttle::from_bps(rate_bps);
        if throttle.is_limited() {
            debug!(
                "Download speed limited to {} KB/s",
                max_speed_kbps
            );
        }

        Self {
            client,
            max_concurrent: max_concurrent.max(1),
            throttle,
        }
    }

    /// Download segments sequentially and concatenate to output file.
    ///
    /// This is the original method, kept for backwards compatibility.
    pub async fn download_segments(
        &self,
        segment_urls: &[String],
        output_path: &Path,
        progress: Option<&ProgressBar>,
    ) -> Result<()> {
        debug!(
            "Downloading {} segments to {:?}",
            segment_urls.len(),
            output_path
        );

        // Create output file
        let mut output_file = File::create(output_path).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to create output file: {}",
                e
            )))
        })?;

        // Download segments in order (sequential for now, can be parallelized)
        for (i, url) in segment_urls.iter().enumerate() {
            let data = self.download_segment_data(url).await?;
            output_file.write_all(&data).await.map_err(|e| {
                Error::Download(DownloadError::SegmentFailed(format!(
                    "Failed to write segment: {}",
                    e
                )))
            })?;

            if let Some(pb) = progress {
                pb.set_position((i + 1) as u64);
            }
        }

        output_file.flush().await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to flush output file: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Download segments in parallel with caching support.
    ///
    /// Segments are downloaded to individual files in `work_dir`, verified,
    /// and then concatenated to the final output.
    ///
    /// Returns the path to the concatenated output file.
    pub async fn download_segments_cached(
        &self,
        segment_urls: &[String],
        work_dir: &Path,
        stream_id: &str,
        cache: Arc<Mutex<StreamCache>>,
        progress: Option<&ProgressBar>,
        save_cache: impl Fn() -> futures::future::BoxFuture<'static, Result<()>> + Send + Sync + 'static,
    ) -> Result<PathBuf> {
        let total_segments = segment_urls.len();
        debug!(
            "Downloading {} segments in parallel (max {} concurrent) to {:?}",
            total_segments, self.max_concurrent, work_dir
        );

        // Ensure work directory exists
        fs::create_dir_all(work_dir).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to create work directory: {}",
                e
            )))
        })?;

        // Create subdirectory for this stream's segments
        let segments_dir = work_dir.join(stream_id);
        fs::create_dir_all(&segments_dir).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to create segments directory: {}",
                e
            )))
        })?;

        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let client = self.client.clone();
        let save_cache = Arc::new(save_cache);

        // Check which segments need downloading
        let pending_indices: Vec<usize> = {
            let cache_guard = cache.lock().await;
            cache_guard.pending_indices()
        };

        let cached_count = total_segments - pending_indices.len();
        if cached_count > 0 {
            debug!(
                "{} segments already verified, {} remaining",
                cached_count,
                pending_indices.len()
            );
            if let Some(pb) = progress {
                pb.set_position(cached_count as u64);
            }
        }

        // Download pending segments in parallel
        let throttle = self.throttle.clone();
        let results: Vec<Result<SegmentResult>> = stream::iter(pending_indices)
            .map(|index| {
                let semaphore = semaphore.clone();
                let client = client.clone();
                let url = segment_urls[index].clone();
                let segment_path = segments_dir.join(format!("segment_{:05}.m4s", index));
                let cache = cache.clone();
                let progress = progress.cloned();
                let save_cache = save_cache.clone();
                let throttle = throttle.clone();

                async move {
                    // Acquire semaphore permit
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        Error::Download(DownloadError::SegmentFailed(format!(
                            "Semaphore error: {}",
                            e
                        )))
                    })?;

                    // Mark as downloading
                    {
                        let mut cache_guard = cache.lock().await;
                        cache_guard.mark_downloading(index);
                    }

                    // Check if segment file exists and verify it
                    if segment_path.exists() {
                        let existing_checksum = {
                            let cache_guard = cache.lock().await;
                            cache_guard
                                .get_segment(index)
                                .and_then(|s| s.checksum.clone())
                        };

                        if let Some(expected) = existing_checksum {
                            match verify_checksum(&segment_path, &expected).await {
                                Ok(true) => {
                                    debug!("Segment {} already verified", index);
                                    let metadata = fs::metadata(&segment_path).await?;
                                    let mut cache_guard = cache.lock().await;
                                    cache_guard.mark_verified(index, expected.clone());
                                    drop(cache_guard);

                                    if let Some(pb) = &progress {
                                        pb.inc(1);
                                    }

                                    return Ok(SegmentResult {
                                        index,
                                        path: segment_path,
                                        size: metadata.len(),
                                        checksum: expected,
                                    });
                                }
                                Ok(false) => {
                                    debug!("Segment {} checksum mismatch, re-downloading", index);
                                    fs::remove_file(&segment_path).await.ok();
                                }
                                Err(e) => {
                                    warn!("Failed to verify segment {}: {}", index, e);
                                    fs::remove_file(&segment_path).await.ok();
                                }
                            }
                        } else {
                            // File exists but no cached checksum — likely a
                            // partial download from a previous failed attempt.
                            // Delete and re-download to avoid corruption.
                            debug!("Segment {} has no cached checksum, re-downloading", index);
                            fs::remove_file(&segment_path).await.ok();
                        }
                    }

                    // Download segment with per-segment retry on transient errors
                    const MAX_SEGMENT_RETRIES: u32 = 3;
                    let mut last_err = None;

                    for attempt in 0..=MAX_SEGMENT_RETRIES {
                        if attempt > 0 {
                            let delay = std::time::Duration::from_millis(500 * 2u64.pow(attempt - 1));
                            warn!(
                                "Segment {} failed, retrying ({}/{}) after {:?}",
                                index, attempt, MAX_SEGMENT_RETRIES, delay
                            );
                            tokio::time::sleep(delay).await;
                            // Clean up partial file before retry
                            fs::remove_file(&segment_path).await.ok();
                        }

                        match download_segment_to_file(&client, &url, &segment_path, index, &throttle).await {
                            Ok((size, checksum)) => {
                                // Update cache
                                {
                                    let mut cache_guard = cache.lock().await;
                                    cache_guard.mark_verified(index, checksum.clone());
                                }

                                // Save cache periodically (every 5 segments)
                                if index % 5 == 0 {
                                    if let Err(e) = save_cache().await {
                                        warn!("Failed to save cache: {}", e);
                                    }
                                }

                                if let Some(pb) = &progress {
                                    pb.inc(1);
                                }

                                return Ok(SegmentResult {
                                    index,
                                    path: segment_path,
                                    size,
                                    checksum,
                                });
                            }
                            Err(e) => {
                                last_err = Some(e);
                            }
                        }
                    }

                    // All retries exhausted
                    let e = last_err.unwrap();
                    warn!("Segment {} failed after {} retries: {}", index, MAX_SEGMENT_RETRIES, e);
                    let mut cache_guard = cache.lock().await;
                    cache_guard.mark_failed(index);
                    Err(e)
                }
            })
            .buffer_unordered(self.max_concurrent)
            .collect()
            .await;

        // Check for errors
        for result in &results {
            if let Err(e) = result {
                return Err(Error::Download(DownloadError::SegmentFailed(format!(
                    "Segment download failed: {}",
                    e
                ))));
            }
        }

        // Final cache save
        if let Err(e) = save_cache().await {
            warn!("Failed to save final cache: {}", e);
        }

        // Concatenate all segments in order
        let output_path = work_dir.join(format!("{}.mp4", stream_id));

        // Skip concatenation if already done and output file still exists
        let already_concatenated = {
            let cache_guard = cache.lock().await;
            cache_guard.concatenated && output_path.exists()
        };

        if !already_concatenated {
            self.concatenate_segments(&segments_dir, total_segments, &output_path)
                .await?;

            // Mark as concatenated
            {
                let mut cache_guard = cache.lock().await;
                cache_guard.concatenated = true;
                cache_guard.output_path = Some(output_path.clone());
            }
        } else {
            debug!("Stream {} already concatenated at {:?}, skipping", stream_id, output_path);
        }

        Ok(output_path)
    }

    /// Concatenate segment files in order to output.
    async fn concatenate_segments(
        &self,
        segments_dir: &Path,
        total_segments: usize,
        output_path: &Path,
    ) -> Result<()> {
        debug!(
            "Concatenating {} segments to {:?}",
            total_segments, output_path
        );

        let mut output_file = File::create(output_path).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to create output file: {}",
                e
            )))
        })?;

        for i in 0..total_segments {
            let segment_path = segments_dir.join(format!("segment_{:05}.m4s", i));

            let data = fs::read(&segment_path).await.map_err(|e| {
                Error::Download(DownloadError::SegmentFailed(format!(
                    "Failed to read segment {}: {}",
                    i, e
                )))
            })?;

            output_file.write_all(&data).await.map_err(|e| {
                Error::Download(DownloadError::SegmentFailed(format!(
                    "Failed to write segment {}: {}",
                    i, e
                )))
            })?;
        }

        output_file.flush().await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to flush output file: {}",
                e
            )))
        })?;

        // Get output file size
        let output_size = fs::metadata(output_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        debug!("Concatenation complete: {:?}", output_path);
        trace!(
            "Concatenated {} segments into {} file",
            total_segments,
            format_bytes(output_size)
        );
        Ok(())
    }

    /// Download a single segment and return its data.
    async fn download_segment_data(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to download segment: {}",
                e
            )))
        })?;

        if !response.status().is_success() {
            return Err(Error::Download(DownloadError::SegmentFailed(format!(
                "Segment download failed with status: {}",
                response.status()
            ))));
        }

        // Stream the response with throttling
        let mut data = Vec::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                Error::Download(DownloadError::SegmentFailed(format!(
                    "Failed to read segment chunk: {}",
                    e
                )))
            })?;

            // Acquire throttle tokens before processing chunk
            self.throttle.acquire(chunk.len() as u64).await;
            data.extend_from_slice(&chunk);
        }

        Ok(data)
    }
}

/// Download a segment to a file and compute its checksum.
async fn download_segment_to_file(
    client: &Client,
    url: &str,
    path: &Path,
    index: usize,
    throttle: &Throttle,
) -> Result<(u64, String)> {
    let start = Instant::now();

    let response = client.get(url).send().await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to download segment {}: {}",
            index, e
        )))
    })?;

    if !response.status().is_success() {
        return Err(Error::Download(DownloadError::SegmentFailed(format!(
            "Segment {} download failed with status: {}",
            index,
            response.status()
        ))));
    }

    // Create file for writing
    let mut file = File::create(path).await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to create segment {} file: {}",
            index, e
        )))
    })?;

    // Stream the response with throttling
    let mut total_size: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to read segment {} chunk: {}",
                index, e
            )))
        })?;

        // Acquire throttle tokens before processing chunk
        throttle.acquire(chunk.len() as u64).await;

        file.write_all(&chunk).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to write segment {}: {}",
                index, e
            )))
        })?;

        total_size += chunk.len() as u64;
    }

    file.flush().await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to flush segment {}: {}",
            index, e
        )))
    })?;

    // Compute checksum
    let checksum = compute_file_hash(path).await?;

    let elapsed = start.elapsed();
    trace!(
        "Segment {}: {} in {} ({}/s)",
        index,
        format_bytes(total_size),
        format_elapsed(elapsed),
        format_bytes((total_size as f64 / elapsed.as_secs_f64()) as u64)
    );

    Ok((total_size, checksum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_downloader_new() {
        let client = Client::new();
        let downloader = SegmentDownloader::new(client, 5, 1000);

        assert_eq!(downloader.max_concurrent, 5);
        assert!(downloader.throttle.is_limited());
    }

    #[test]
    fn test_segment_downloader_min_concurrent() {
        let client = Client::new();
        let downloader = SegmentDownloader::new(client, 0, 0);

        // Should be at least 1
        assert_eq!(downloader.max_concurrent, 1);
        assert!(!downloader.throttle.is_limited());
    }
}
