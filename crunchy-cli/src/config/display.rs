//! Display formatting for configuration.

use crate::config::Config;
use console::{style, Style};

/// Display configuration in a standard key = value format grouped by section.
pub fn display_pretty(config: &Config) {
    println!();
    display_auth_section(config);
    display_downloads_section(config);
    display_languages_section(config);
    display_muxing_section(config);
    display_tools_section(config);
    display_proxy_section(config);
}

/// Display a section header.
fn section_header(title: &str) {
    println!("{}", style(format!("[{}]", title)).bold());
}

/// Display a key = value pair.
fn field(key: &str, value: &str) {
    println!("  {} = {}", style(key).dim(), value);
}

/// Display a key with no value set.
fn field_unset(key: &str) {
    println!("  {} = {}", style(key).dim(), style("(not set)").dim());
}

/// Mask a token showing only first 4 and last 4 characters.
fn mask_token(token: &str) -> String {
    if token.len() <= 12 {
        return "***".to_string();
    }
    format!("{}...{}", &token[..4], &token[token.len() - 4..])
}

/// Display authentication section.
fn display_auth_section(config: &Config) {
    section_header("auth");

    match &config.auth.device_id {
        Some(id) => field("device_id", id),
        None => field_unset("device_id"),
    }

    match &config.auth.access_token {
        Some(token) => field("access_token", &mask_token(token)),
        None => field_unset("access_token"),
    }

    match &config.auth.refresh_token {
        Some(token) => field("refresh_token", &mask_token(token)),
        None => field_unset("refresh_token"),
    }

    match config.auth.expires_at {
        Some(expires_at) => {
            let text = chrono::DateTime::from_timestamp(expires_at as i64, 0)
                .map(|dt| {
                    let formatted = dt.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    if expires_at > now {
                        let rem = expires_at - now;
                        format!("{} (expires in {}h {}m)", formatted, rem / 3600, (rem % 3600) / 60)
                    } else {
                        format!("{} (expired)", formatted)
                    }
                })
                .unwrap_or_else(|| expires_at.to_string());
            field("expires_at", &text);
        }
        None => field_unset("expires_at"),
    }

    match &config.auth.account_id {
        Some(id) => field("account_id", id),
        None => field_unset("account_id"),
    }

    match &config.auth.profile_id {
        Some(id) => field("profile_id", id),
        None => field_unset("profile_id"),
    }

    let status = if config.is_authenticated() { "authenticated" } else { "not authenticated" };
    field("status", status);
    println!();
}

/// Display downloads section.
fn display_downloads_section(config: &Config) {
    section_header("downloads");

    field("output_dir", &config.downloads.output_dir.display().to_string());
    field("temp_dir", &config.downloads.temp_dir.display().to_string());

    match &config.downloads.cache_dir {
        Some(p) => field("cache_dir", &p.display().to_string()),
        None => field_unset("cache_dir"),
    }

    field("cache_retention_hours", &config.downloads.cache_retention_hours.to_string());

    let speed = if config.downloads.max_speed_kbps == 0 {
        "0 (unlimited)".to_string()
    } else {
        format!("{}", config.downloads.max_speed_kbps)
    };
    field("max_speed_kbps", &speed);

    field("simultaneous", &config.downloads.simultaneous.to_string());
    field("parts", &config.downloads.parts.to_string());
    field("video_quality", &config.downloads.video_quality);
    field("audio_quality", &config.downloads.audio_quality);
    field("retry_count", &config.downloads.retry_count.to_string());
    field("max_concurrent_keys", &config.downloads.max_concurrent_keys.to_string());
    println!();
}

/// Display languages section.
fn display_languages_section(config: &Config) {
    section_header("languages");

    let audio = if config.languages.audio.is_empty() {
        "(none)".to_string()
    } else {
        config.languages.audio.join(", ")
    };
    field("audio", &audio);

    let subs = if config.languages.subtitles.is_empty() {
        "(none)".to_string()
    } else {
        config.languages.subtitles.join(", ")
    };
    field("subtitles", &subs);
    field("include_cc", &config.languages.include_cc.to_string());
    println!();
}

/// Display muxing section.
fn display_muxing_section(config: &Config) {
    section_header("muxing");

    field("format", &config.muxing.format);
    field("embed_subs", &config.muxing.embed_subs.to_string());
    field("default_audio", &config.muxing.default_audio);
    field("default_sub", &config.muxing.default_sub);
    field("prefer_signs_songs", &config.muxing.prefer_signs_songs.to_string());
    field("filename_template", &config.muxing.filename_template);
    println!();
}

/// Display tools section.
fn display_tools_section(config: &Config) {
    section_header("tools");

    let ffmpeg_path = ffmpeg_sidecar::paths::ffmpeg_path();
    field("ffmpeg", &ffmpeg_path.display().to_string());
    field("mp4decrypt", &config.tools.mp4decrypt);

    match &config.tools.widevine_client {
        Some(p) => field("widevine_client", &p.display().to_string()),
        None => field_unset("widevine_client"),
    }

    match &config.tools.widevine_private_key {
        Some(p) => field("widevine_private_key", &p.display().to_string()),
        None => field_unset("widevine_private_key"),
    }

    println!();
}

/// Display proxy section.
fn display_proxy_section(config: &Config) {
    section_header("proxy");

    field("enabled", &config.proxy.enabled.to_string());

    match &config.proxy.url {
        Some(url) => field("url", url),
        None => field_unset("url"),
    }

    println!();
}

/// Print all available config keys with descriptions and examples.
pub fn print_config_keys() {
    let key_style = Style::new().yellow();
    let desc_style = Style::new().white().dim();
    let example_style = Style::new().cyan();

    println!();
    println!(
        "  {}",
        style("Available configuration keys:").bold()
    );
    println!();

    let sections: &[(&str, &[(&str, &str, &str)])] = &[
        (
            "Downloads",
            &[
                ("downloads.output_dir", "Output directory for downloaded files", "~/Videos/Crunchyroll"),
                ("downloads.temp_dir", "Temporary directory for in-progress downloads", "~/Videos/.tmp"),
                ("downloads.cache_dir", "Directory for segment cache files", "~/.cache/crunchy-cli"),
                ("downloads.cache_retention_hours", "Hours to keep cached segments", "72"),
                ("downloads.video_quality", "Preferred video quality", "best | 1080p | 720p | 480p | 360p | 240p"),
                ("downloads.audio_quality", "Preferred audio quality", "best"),
                ("downloads.max_speed_kbps", "Max download speed in KB/s (0 = unlimited)", "0"),
                ("downloads.simultaneous", "Number of simultaneous downloads", "2"),
                ("downloads.parts", "Number of parallel segment downloads", "4"),
                ("downloads.retry_count", "Number of retries on failure", "3"),
                ("downloads.max_concurrent_keys", "Max concurrent DRM key requests", "2"),
            ],
        ),
        (
            "Languages",
            &[
                ("languages.audio", "Audio languages (comma-separated)", "ja-JP,en-US"),
                ("languages.subtitles", "Subtitle languages (comma-separated)", "en-US,es-ES"),
                ("languages.include_cc", "Include closed captions", "true | false"),
            ],
        ),
        (
            "Muxing",
            &[
                ("muxing.format", "Output container format", "mkv | mp4"),
                ("muxing.embed_subs", "Embed subtitles in output file", "true | false"),
                ("muxing.default_audio", "Default audio track language", "ja-JP"),
                ("muxing.default_sub", "Default subtitle track language", "en-US"),
                ("muxing.prefer_signs_songs", "Prefer Signs & Songs sub when audio and sub match", "true | false"),
                ("muxing.filename_template", "Output filename template. Vars: {series}, {season}, {season:02}, {season_title}, {episode}, {episode:02}, {title}, {quality}, {audio}, {year}", "{series} - S{season:02}E{episode:02} - {title}"),
            ],
        ),
        (
            "Tools",
            &[
                ("tools.mp4decrypt", "Path to mp4decrypt binary", "mp4decrypt"),
                ("tools.widevine_client", "Path to Widevine client ID file", "/path/to/client_id.bin"),
                ("tools.widevine_private_key", "Path to Widevine private key file", "/path/to/private_key.pem"),
            ],
        ),
        (
            "Proxy",
            &[
                ("proxy.enabled", "Enable proxy for requests", "true | false"),
                ("proxy.url", "Proxy URL", "http://127.0.0.1:8080"),
            ],
        ),
    ];

    for (section_name, keys) in sections {
        println!("  {}:", style(section_name).cyan().bold());
        for (key, description, example) in *keys {
            println!(
                "    {:<40} {}",
                key_style.apply_to(key),
                desc_style.apply_to(description),
            );
            println!(
                "    {:<40} {}",
                "",
                example_style.apply_to(format!("e.g. crunchy-cli config set {} {}", key, example)),
            );
        }
        println!();
    }
}
