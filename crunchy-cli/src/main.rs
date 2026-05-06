//! Crunchy-CLI: A command-line tool for downloading content from Crunchyroll.

use anyhow::Result;
use clap::Parser;
use crunchy_cli::api::types::{CREpisode, CRLanguage};
use crunchy_cli::api::CrunchyrollClient;
use crunchy_cli::cli::{
    CacheCommands,
    Cli,
    Commands,
    ConfigCommands,
    CrunchyrollUrl,
    QueueCommands,
};
use crunchy_cli::config::Config;
use crunchy_cli::download::{
    cache::format_bytes,
    cleanup_stale_caches,
    list_caches,
    resolve_episodes,
    DownloadManager,
    DownloadMode,
    DownloadOptions,
    DownloadResult,
};
use crunchy_cli::utils::tree::{child_prefix, format_locales, print_tree_item};
use crunchy_cli::queue::{
    DownloadOptions as QueueDownloadOptions,
    QueueItem,
    QueueManager,
    QueueStatus,
};

use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, Level};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging
    let log_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let filter = if cli.quiet {
        EnvFilter::new("error")
    } else {
        EnvFilter::new(format!("crunchy_cli={}", log_level))
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Log verbose level info
    if cli.verbose > 0 {
        debug!(
            "Verbose mode enabled (level {}): {}",
            cli.verbose,
            match cli.verbose {
                1 => "INFO",
                2 => "DEBUG",
                _ => "TRACE",
            }
        );
    }

    // Load configuration
    let config_path = cli.config.as_ref();
    let config = if let Some(path) = config_path {
        Config::load_from(path)?
    } else {
        Config::load()?
    };

    // Auto-cleanup stale cache entries in the background
    let cache_dir = config.downloads.get_cache_dir();
    let retention_hours = config.downloads.cache_retention_hours;
    if retention_hours > 0 {
        tokio::spawn(async move {
            let max_age = chrono::Duration::hours(retention_hours as i64);
            match cleanup_stale_caches(&cache_dir, max_age).await {
                Ok(stats) if stats.removed > 0 => {
                    info!(
                        "Auto-cleaned {} stale cache entries ({})",
                        stats.removed,
                        format_bytes(stats.bytes_freed)
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!("Cache auto-cleanup failed: {}", e);
                }
            }
        });
    }

    let config = Arc::new(RwLock::new(config));

    // Execute command
    match cli.command {
        Commands::Login(args) => cmd_login(config, args).await,
        Commands::Logout => cmd_logout(config).await,
        Commands::Whoami { json } => cmd_whoami(config, json).await,
        Commands::Search(args) => cmd_search(config, args).await,
        Commands::Info(args) => cmd_info(config, args).await,
        Commands::Download(args) => cmd_download(config, args).await,
        Commands::Queue(args) => cmd_queue(config, args).await,
        Commands::Cache(args) => cmd_cache(config, args).await,
        Commands::Config(args) => cmd_config(config, args).await,
    }
}

/// Handle login command.
async fn cmd_login(config: Arc<RwLock<Config>>, args: crunchy_cli::cli::LoginArgs) -> Result<()> {
    let client = CrunchyrollClient::new(config.clone()).await?;

    let token = if let Some(refresh_token) = args.token {
        info!("Logging in with refresh token");
        client.login_with_token(&refresh_token).await?
    } else {
        let username = args
            .username
            .ok_or_else(|| anyhow::anyhow!("Username required. Use -u/--username or --token"))?;
        let password = args
            .password
            .ok_or_else(|| anyhow::anyhow!("Password required. Use -p/--password"))?;

        client.login(&username, &password).await?
    };

    // Save tokens to config
    {
        let mut cfg = config.write().await;
        cfg.set_tokens(
            token.access_token,
            token.refresh_token,
            token.expires_in,
            token.account_id,
            token.profile_id,
        );
        cfg.save()?;
    }

    println!("Login successful!");
    Ok(())
}

/// Handle logout command.
async fn cmd_logout(config: Arc<RwLock<Config>>) -> Result<()> {
    let mut cfg = config.write().await;
    cfg.clear_tokens();
    cfg.save()?;
    println!("Logged out successfully.");
    Ok(())
}

/// Handle whoami command.
async fn cmd_whoami(config: Arc<RwLock<Config>>, json: bool) -> Result<()> {
    let client = CrunchyrollClient::new(config.clone()).await?;
    let profile = client.get_profile().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
    } else {
        println!("Username: {}", profile.username);
        println!("Email: {}", profile.email);
        if !profile.preferred_content_audio_language.is_empty() {
            println!(
                "Preferred Audio: {}",
                profile.preferred_content_audio_language
            );
        }
        if !profile.preferred_content_subtitle_language.is_empty() {
            println!(
                "Preferred Subtitles: {}",
                profile.preferred_content_subtitle_language
            );
        }
    }

    Ok(())
}

/// Handle search command.
async fn cmd_search(config: Arc<RwLock<Config>>, args: crunchy_cli::cli::SearchArgs) -> Result<()> {
    let client = CrunchyrollClient::new(config.clone()).await?;
    let results = client.search(&args.query, args.limit).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
    }

    // --episodes implies --expand
    let should_expand = args.expand || args.episodes;

    // Collect all series items from results
    let series_items: Vec<_> = results
        .iter()
        .filter(|r| r.result_type == "series")
        .flat_map(|r| r.items.iter())
        .collect();

    if series_items.is_empty() {
        println!("No series found for '{}'", args.query);
        return Ok(());
    }

    println!("Found {} series for '{}':\n", series_items.len(), args.query);

    for (idx, item) in series_items.iter().enumerate() {
        let series_num = idx + 1;

        // Print series header
        println!("{}. {}", series_num, item.title);
        println!("   https://www.crunchyroll.com/series/{}", item.id);

        // Expand seasons if requested and within limit
        if should_expand && idx < args.expand_limit as usize {
            match client.get_seasons(&item.id).await {
                Ok(seasons) => {
                    if seasons.is_empty() {
                        println!("   (no seasons available)");
                    } else {
                        let season_count = seasons.len();
                        for (s_idx, season) in seasons.iter().enumerate() {
                            let is_last_season = s_idx == season_count - 1;

                            // Format audio locales
                            let audio_info = if season.audio_locales.is_empty() {
                                season.audio_locale.clone()
                            } else {
                                format_locales(&season.audio_locales)
                            };

                            // Season line: Season N - Title [audio_locales]
                            let season_title = if season.title.is_empty()
                                || season.title == format!("Season {}", season.season_sequence_number)
                            {
                                format!("Season {}", season.season_sequence_number)
                            } else {
                                format!("Season {} - {}", season.season_sequence_number, season.title)
                            };

                            let season_line = format!("{} [{}]", season_title, audio_info);
                            print_tree_item("   ", is_last_season, &season_line);

                            // Expand episodes if requested
                            if args.episodes {
                                match client.get_episodes(&season.id).await {
                                    Ok(episodes) => {
                                        let ep_prefix = child_prefix("   ", is_last_season);
                                        let ep_count = episodes.len();

                                        for (e_idx, ep) in episodes.iter().enumerate() {
                                            let is_last_ep = e_idx == ep_count - 1;

                                            // Format episode number
                                            let ep_num = if ep.episode.is_empty() {
                                                format!("{}", ep.sequence_number)
                                            } else {
                                                ep.episode.clone()
                                            };

                                            // Truncate long titles
                                            let title = truncate(&ep.title, 40);

                                            // Print episode line
                                            let ep_line = format!(
                                                "E{} - {} [{}]",
                                                ep_num, title, ep.audio_locale
                                            );
                                            print_tree_item(&ep_prefix, is_last_ep, &ep_line);

                                            // Print subs on a separate line if --subs flag is set
                                            if args.subs && !ep.subtitle_locales.is_empty() {
                                                let subs_prefix = child_prefix(&ep_prefix, is_last_ep);
                                                println!("{}subs: {}", subs_prefix, format_locales(&ep.subtitle_locales));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let ep_prefix = child_prefix("   ", is_last_season);
                                        print_tree_item(
                                            &ep_prefix,
                                            true,
                                            &format!("(error fetching episodes: {})", e),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("   (error fetching seasons: {})", e);
                }
            }
        }

        // Add spacing between series
        if idx < series_items.len() - 1 {
            println!();
        }
    }

    Ok(())
}

/// Handle info command.
async fn cmd_info(config: Arc<RwLock<Config>>, args: crunchy_cli::cli::InfoArgs) -> Result<()> {
    use crunchy_cli::cli::CrunchyrollUrl;

    let url = CrunchyrollUrl::parse(&args.url)
        .ok_or_else(|| anyhow::anyhow!("Invalid Crunchyroll URL or ID: {}", args.url))?;

    let client = CrunchyrollClient::new(config.clone()).await?;

    if url.is_series() {
        let series = client.get_series(&url.id).await?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&series)?);
        } else {
            println!("Series: {} ({})", series.title, series.id);
            println!("  Seasons: {}", series.season_count);
            println!("  Episodes: {}", series.episode_count);
            if !series.description.is_empty() {
                println!("  Description: {}", truncate(&series.description, 200));
            }
        }

        if args.seasons || args.episodes {
            let seasons = client.get_seasons(&url.id).await?;

            for season in &seasons {
                println!(
                    "\n  Season {} - {} ({})",
                    season.season_sequence_number, season.title, season.id
                );
                println!("    Audio: {}", season.audio_locale);
                println!("    Episodes: {}", season.number_of_episodes);

                if args.episodes {
                    let episodes = client.get_episodes(&season.id).await?;
                    for ep in &episodes {
                        println!("      E{} - {} ({})", ep.episode, ep.title, ep.id);
                    }
                }
            }
        }
    } else {
        let episode = client.get_episode(&url.id).await?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&episode)?);
        } else {
            println!("Episode: {} ({})", episode.title, episode.id);
            println!("  Series: {}", episode.series_title);
            println!("  Season: {}", if episode.season_sequence_number > 0 { episode.season_sequence_number } else { episode.season_number });
            println!("  Episode: {}", episode.episode);
            println!("  Duration: {} min", episode.duration_ms / 60000);
            println!("  Audio: {}", episode.audio_locale);
            if !episode.subtitle_locales.is_empty() {
                println!("  Subtitles: {}", episode.subtitle_locales.join(", "));
            }
        }
    }

    Ok(())
}

/// Handle download command.
async fn cmd_download(
    config: Arc<RwLock<Config>>,
    args: crunchy_cli::cli::DownloadArgs,
) -> Result<()> {
    let url = CrunchyrollUrl::parse(&args.url)
        .ok_or_else(|| anyhow::anyhow!("Invalid Crunchyroll URL or ID: {}", args.url))?;

    let client = Arc::new(CrunchyrollClient::new(config.clone()).await?);

    // Resolve URL to episode list
    let episodes = resolve_episodes(&client, &url).await?;

    if episodes.is_empty() {
        println!("No episodes found matching your criteria.");
        return Ok(());
    }

    println!("Found {} episode(s) to download", episodes.len());

    // Build download options from CLI args
    let cfg = config.read().await;

    // Determine download mode from CLI flags
    let download_mode = if args.only_subs {
        DownloadMode::OnlySubs
    } else if args.only_audio {
        DownloadMode::OnlyAudio
    } else if args.only_video {
        DownloadMode::OnlyVideo
    } else {
        DownloadMode::Full
    };

    let download_options = DownloadOptions {
        video_quality: Some(quality_to_string(&args.quality)),
        audio_languages: if args.all_audio {
            Some(CRLanguage::all_codes())
        } else if args.audio_langs.is_empty() {
            None
        } else {
            Some(args.audio_langs.clone())
        },
        subtitle_languages: if args.all_subs {
            Some(CRLanguage::all_codes())
        } else if args.sub_langs.is_empty() {
            None // Use config defaults
        } else {
            Some(args.sub_langs.clone())
        },
        output_dir: args.output.clone(),
        skip_existing: true, // Always skip existing by default
        download_mode,
        resume_cache: args.experimental_resume,
        include_cc: args.experimental_cc || cfg.languages.include_cc,
    };

    let simultaneous = cfg.downloads.simultaneous as usize;
    let retry_count = args.retries.unwrap_or(cfg.downloads.retry_count);
    drop(cfg);

    // Create download manager
    let manager = DownloadManager::new(client.clone(), config.clone());

    // Download episodes (parallel or sequential based on config)
    let results =
        download_episodes(&manager, &episodes, &download_options, simultaneous, retry_count).await;

    // Print summary
    print_download_summary(&results);

    // Return error if any downloads failed
    let failed_count = results.iter().filter(|r| r.is_err()).count();
    if failed_count > 0 {
        anyhow::bail!("{} download(s) failed", failed_count);
    }

    Ok(())
}


/// Download multiple episodes with parallel execution.
async fn download_episodes(
    manager: &DownloadManager,
    episodes: &[CREpisode],
    options: &DownloadOptions,
    simultaneous: usize,
    retry_count: u8,
) -> Vec<Result<DownloadResult, String>> {
    let total = episodes.len();
    println!("\nDownloading {} episode(s)...\n", total);

    // Use streaming for parallel downloads
    let results: Vec<_> = stream::iter(episodes.iter().enumerate())
        .map(|(i, episode)| {
            let episode_id = episode.id.clone();
            let season_num = if episode.season_sequence_number > 0 { episode.season_sequence_number } else { episode.season_number };
            let episode_title = format!(
                "S{}E{} - {}",
                season_num, episode.episode, episode.title
            );
            let opts = options.clone();

            async move {
                println!("[{}/{}] Starting: {}", i + 1, total, episode_title);

                let mut last_err = String::new();
                for attempt in 0..=retry_count {
                    if attempt > 0 {
                        let delay_secs = 5u64 * 2u64.pow((attempt - 1) as u32);
                        println!(
                            "[{}/{}] Retry {}/{}: {} (waiting {}s...)",
                            i + 1,
                            total,
                            attempt,
                            retry_count,
                            episode_title,
                            delay_secs
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                    }

                    match manager
                        .download_episode_with_options(&episode_id, opts.clone())
                        .await
                    {
                        Ok(result) => {
                            println!(
                                "[{}/{}] Completed: {} -> {}",
                                i + 1,
                                total,
                                episode_title,
                                result.output_uri
                            );
                            return Ok(result);
                        }
                        Err(e) => {
                            last_err = format!("{}", e);
                            error!(
                                "[{}/{}] Failed (attempt {}/{}): {} - {}",
                                i + 1,
                                total,
                                attempt + 1,
                                retry_count + 1,
                                episode_title,
                                e
                            );
                        }
                    }
                }

                Err(format!("{}: {}", episode_title, last_err))
            }
        })
        .buffer_unordered(simultaneous.max(1))
        .collect()
        .await;

    results
}

/// Print a summary of download results.
fn print_download_summary(results: &[Result<DownloadResult, String>]) {
    let total = results.len();
    let succeeded = results.iter().filter(|r| r.is_ok()).count();
    let failed = results.iter().filter(|r| r.is_err()).count();

    println!("\n--- Download Summary ---");
    println!("Total: {}", total);
    println!("Succeeded: {}", succeeded);
    println!("Failed: {}", failed);

    if failed > 0 {
        println!("\nFailed downloads:");
        for result in results {
            if let Err(e) = result {
                println!("  - {}", e);
            }
        }
    }
}

/// Convert VideoQuality enum to string for DownloadOptions.
fn quality_to_string(quality: &crunchy_cli::cli::VideoQuality) -> String {
    match quality.height() {
        Some(h) => format!("{}p", h),
        None => "best".to_string(),
    }
}

/// Handle queue commands.
async fn cmd_queue(config: Arc<RwLock<Config>>, args: crunchy_cli::cli::QueueArgs) -> Result<()> {
    let cfg = config.read().await;
    let manager = QueueManager::new(cfg.downloads.simultaneous as usize)?;
    drop(cfg);

    match args.command {
        QueueCommands::Add {
            url,
            output,
            audio_langs,
            sub_langs,
            quality,
            episodes: _,
            seasons: _,
        } => {
            use crunchy_cli::cli::CrunchyrollUrl;

            let parsed = CrunchyrollUrl::parse(&url)
                .ok_or_else(|| anyhow::anyhow!("Invalid Crunchyroll URL or ID: {}", url))?;

            let cfg = config.read().await;
            let options = QueueDownloadOptions {
                output_dir: output.unwrap_or_else(|| cfg.downloads.output_dir.clone()),
                audio_langs: if audio_langs.is_empty() {
                    cfg.languages.audio.clone()
                } else {
                    audio_langs
                },
                sub_langs: if sub_langs.is_empty() {
                    cfg.languages.subtitles.clone()
                } else {
                    sub_langs
                },
                video_quality: format!("{:?}", quality).to_lowercase(),
                skip_mux: false,
                no_subs: false,
            };

            // For now, create a placeholder item
            let item = QueueItem::new(
                &parsed.id,
                "Unknown Series",
                "Unknown Episode",
                0,
                "0",
                options,
            );

            let uuid = manager.add(item).await?;
            println!("Added to queue: {}", uuid);
        }

        QueueCommands::List { json } => {
            let items = manager.list().await;

            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else if items.is_empty() {
                println!("Queue is empty.");
            } else {
                println!("Queue ({} items):\n", items.len());
                for (i, item) in items.iter().enumerate() {
                    let status_icon = match item.status {
                        QueueStatus::Pending => "⏳",
                        QueueStatus::Active => "▶️",
                        QueueStatus::Paused => "⏸️",
                        QueueStatus::Completed => "✅",
                        QueueStatus::Failed => "❌",
                    };
                    println!(
                        "  {}. {} {} [{}]",
                        i,
                        status_icon,
                        item.display_name(),
                        item.status
                    );
                    if let Some(ref err) = item.error {
                        println!("      Error: {}", err);
                    }
                }
            }
        }

        QueueCommands::Remove { id } => {
            manager.remove(&id).await?;
            println!("Removed from queue.");
        }

        QueueCommands::Clear => {
            let count = manager.clear().await?;
            println!("Cleared {} items from queue.", count);
        }

        QueueCommands::Start => {
            println!("Queue processing not yet implemented.");
            // TODO: Implement queue runner
        }

        QueueCommands::Pause => {
            let count = manager.pause_all().await?;
            println!("Paused {} downloads.", count);
        }
    }

    Ok(())
}

/// Handle cache commands.
async fn cmd_cache(config: Arc<RwLock<Config>>, args: crunchy_cli::cli::CacheArgs) -> Result<()> {
    let cfg = config.read().await;
    let cache_dir = cfg.downloads.get_cache_dir();
    let retention_hours = cfg.downloads.cache_retention_hours;
    drop(cfg);

    match args.command {
        CacheCommands::List { json } => {
            let caches = list_caches(&cache_dir).await?;

            if json {
                // Serialize to JSON
                let json_caches: Vec<_> = caches
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "episode_id": c.episode_id,
                            "created_at": c.created_at.to_rfc3339(),
                            "size_bytes": c.size,
                            "phase": format!("{:?}", c.phase),
                            "path": c.path.display().to_string(),
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&json_caches)?);
            } else if caches.is_empty() {
                println!("No cached downloads found.");
            } else {
                println!("Cached downloads ({} entries):\n", caches.len());
                let total_size: u64 = caches.iter().map(|c| c.size).sum();

                for cache in &caches {
                    println!(
                        "  {} - {} ({}) - {:?}",
                        cache.episode_id,
                        format_bytes(cache.size),
                        cache.created_at.format("%Y-%m-%d %H:%M"),
                        cache.phase
                    );
                    println!("      {}", cache.summary);
                }

                println!("\nTotal cache size: {}", format_bytes(total_size));
            }
        }

        CacheCommands::Clean { all, force } => {
            if all && !force {
                println!("This will remove ALL cached downloads.");
                println!("Run with --force to confirm.");
                return Ok(());
            }

            let max_age = if all {
                chrono::Duration::zero()
            } else {
                chrono::Duration::hours(retention_hours as i64)
            };

            let stats = cleanup_stale_caches(&cache_dir, max_age).await?;

            if stats.removed == 0 {
                println!("No stale cache entries found.");
            } else {
                println!(
                    "Cleaned {} cache entries, freed {}",
                    stats.removed,
                    format_bytes(stats.bytes_freed)
                );
            }
        }

        CacheCommands::Path => {
            println!("{}", cache_dir.display());
        }
    }

    Ok(())
}

/// Handle config commands.
async fn cmd_config(config: Arc<RwLock<Config>>, args: crunchy_cli::cli::ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommands::Show { json, key } => {
            let cfg = config.read().await;
            if let Some(key) = key {
                let value = cfg.get_key(&key)?;
                if json {
                    println!("{}", serde_json::json!({ &key: &value }));
                } else {
                    println!("{}", value);
                }
            } else if json {
                println!("{}", serde_json::to_string_pretty(&*cfg)?);
            } else {
                crunchy_cli::config::display_pretty(&*cfg);
            }
        }

        ConfigCommands::Edit => {
            let path = Config::default_config_path()?;
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());

            std::process::Command::new(&editor).arg(&path).status()?;

            println!("Config edited. Reload with 'crunchy-cli config show' to verify.");
        }

        ConfigCommands::Set { key, value } => match (key, value) {
            (Some(key), Some(value)) => {
                let mut cfg = config.write().await;
                cfg.set_key(&key, &value)?;
                cfg.save()?;
                println!("Set {} = {}", key, value);
            }
            (Some(_), None) => {
                anyhow::bail!("Missing value. Usage: crunchy-cli config set <KEY> <VALUE>");
            }
            _ => {
                crunchy_cli::config::print_config_keys();
            }
        },

        ConfigCommands::Reset { force } => {
            if !force {
                println!("This will reset all settings to defaults.");
                println!("Run with --force to confirm.");
                return Ok(());
            }

            let path = Config::init(true)?;
            println!("Config reset to defaults at {:?}", path);
        }

        ConfigCommands::Path => {
            let path = Config::default_config_path()?;
            println!("{}", path.display());
        }

        ConfigCommands::Init { force } => {
            let path = Config::init(force)?;
            println!("Config initialized at {:?}", path);
        }
    }

    Ok(())
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
