//! CLI command definitions using clap derive macros.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Crunchy-CLI: Download content from Crunchyroll
#[derive(Parser, Debug)]
#[command(name = "crunchy-cli")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Use alternate config file
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Authenticate with Crunchyroll
    Login(LoginArgs),

    /// Clear stored credentials
    Logout,

    /// Show current user profile
    Whoami {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Search for series
    Search(SearchArgs),

    /// Show series/season/episode details
    Info(InfoArgs),

    /// Download episodes
    Download(DownloadArgs),

    /// Manage download queue
    Queue(QueueArgs),

    /// Manage download cache
    Cache(CacheArgs),

    /// View/edit configuration
    Config(ConfigArgs),
}

/// Arguments for the login command
#[derive(Parser, Debug)]
pub struct LoginArgs {
    /// Crunchyroll account email
    #[arg(short, long)]
    pub username: Option<String>,

    /// Crunchyroll account password
    #[arg(short, long)]
    pub password: Option<String>,

    /// Login with existing refresh token
    #[arg(long, conflicts_with_all = ["username", "password"])]
    pub token: Option<String>,
}

/// Arguments for the search command
#[derive(Parser, Debug)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    pub limit: u32,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Expand results to show seasons
    #[arg(long, short = 'e')]
    pub expand: bool,

    /// Show episodes under each season (implies --expand)
    #[arg(long, short = 'E')]
    pub episodes: bool,

    /// Show subtitle languages for episodes (requires --episodes)
    #[arg(long, short = 's')]
    pub subs: bool,

    /// Maximum number of series to expand (default: 3)
    #[arg(long, default_value = "3")]
    pub expand_limit: u32,
}

/// Arguments for the info command
#[derive(Parser, Debug)]
pub struct InfoArgs {
    /// Crunchyroll URL or content ID
    pub url: String,

    /// List seasons
    #[arg(long)]
    pub seasons: bool,

    /// List episodes
    #[arg(long)]
    pub episodes: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for the download command
#[derive(Parser, Debug)]
pub struct DownloadArgs {
    /// Crunchyroll URL or content ID
    pub url: String,

    /// Output directory
    #[arg(short, long, value_name = "DIR")]
    pub output: Option<PathBuf>,

    /// Audio languages (can be specified multiple times)
    #[arg(short, long = "audio", value_name = "LANG", conflicts_with = "all_audio")]
    pub audio_langs: Vec<String>,

    /// Download all available audio dubs
    #[arg(long, conflicts_with = "audio_langs")]
    pub all_audio: bool,

    /// Subtitle languages (can be specified multiple times)
    #[arg(short, long = "subs", value_name = "LANG", conflicts_with = "all_subs")]
    pub sub_langs: Vec<String>,

    /// Download all available subtitles
    #[arg(long, conflicts_with = "sub_langs")]
    pub all_subs: bool,

    /// Video quality
    #[arg(long, value_enum, default_value = "best")]
    pub quality: VideoQuality,

    /// Don't mux, keep separate files
    #[arg(long, conflicts_with_all = ["only_subs", "only_audio", "only_video"])]
    pub skip_mux: bool,

    /// Disable progress bars
    #[arg(long)]
    pub no_progress: bool,

    /// Download only subtitles (outputs {name}.subtitles.mkv)
    #[arg(long, conflicts_with_all = ["only_audio", "only_video", "skip_mux"])]
    pub only_subs: bool,

    /// Download only audio tracks (outputs {name}.audios.mkv)
    #[arg(long, conflicts_with_all = ["only_subs", "only_video", "skip_mux"])]
    pub only_audio: bool,

    /// Download only video track (outputs {name}.video.mkv)
    #[arg(long, conflicts_with_all = ["only_subs", "only_audio", "skip_mux"])]
    pub only_video: bool,

    /// [Experimental] Include closed caption (CC) subtitles in downloads.
    /// By default, CC subtitles are excluded. Enable this to include them.
    #[arg(long)]
    pub experimental_cc: bool,

    /// [Experimental] Enable resumable downloads via segment caching.
    /// When enabled, interrupted downloads can be resumed from where they left off.
    /// This feature is experimental and may produce corrupted output in some cases.
    #[arg(long)]
    pub experimental_resume: bool,

    /// Number of retries for failed episode downloads (overrides config)
    #[arg(long, value_name = "N")]
    pub retries: Option<u8>,
}

/// Video quality options
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum VideoQuality {
    #[default]
    Best,
    #[value(name = "1080p")]
    P1080,
    #[value(name = "720p")]
    P720,
    #[value(name = "480p")]
    P480,
    #[value(name = "360p")]
    P360,
    #[value(name = "240p")]
    P240,
}

impl VideoQuality {
    /// Get the height in pixels for this quality level.
    pub fn height(&self) -> Option<u32> {
        match self {
            VideoQuality::Best => None,
            VideoQuality::P1080 => Some(1080),
            VideoQuality::P720 => Some(720),
            VideoQuality::P480 => Some(480),
            VideoQuality::P360 => Some(360),
            VideoQuality::P240 => Some(240),
        }
    }
}

/// Arguments for the queue command
#[derive(Parser, Debug)]
pub struct QueueArgs {
    #[command(subcommand)]
    pub command: QueueCommands,
}

#[derive(Subcommand, Debug)]
pub enum QueueCommands {
    /// Add item to queue
    Add {
        /// Crunchyroll URL or content ID
        url: String,

        /// Output directory
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// Audio languages
        #[arg(short, long = "audio", value_name = "LANG")]
        audio_langs: Vec<String>,

        /// Subtitle languages
        #[arg(short, long = "subs", value_name = "LANG")]
        sub_langs: Vec<String>,

        /// Video quality
        #[arg(long, value_enum, default_value = "best")]
        quality: VideoQuality,

        /// Episode range
        #[arg(long, value_name = "RANGE")]
        episodes: Option<String>,

        /// Season range
        #[arg(long, value_name = "RANGE")]
        seasons: Option<String>,
    },

    /// Show queue status
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Remove item from queue
    Remove {
        /// UUID or index of item to remove
        id: String,
    },

    /// Clear entire queue
    Clear,

    /// Start processing queue
    Start,

    /// Pause queue processing
    Pause,
}

/// Arguments for the cache command
#[derive(Parser, Debug)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommands,
}

#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// List cached downloads
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Clean up stale cache entries
    Clean {
        /// Remove all cache entries (not just stale ones)
        #[arg(long)]
        all: bool,

        /// Don't ask for confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show cache directory path
    Path,
}

/// Arguments for the config command
#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Display current config (or a single key's value)
    Show {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Optional config key to show (e.g., "downloads.output_dir")
        key: Option<String>,
    },

    /// Open config in $EDITOR
    Edit,

    /// Set a config value (run without arguments to list all keys)
    Set {
        /// Config key (e.g., "downloads.output_dir")
        key: Option<String>,
        /// Value to set
        value: Option<String>,
    },

    /// Reset config to defaults
    Reset {
        /// Don't ask for confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show config file path
    Path,

    /// Initialize config with default values
    Init {
        /// Overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::parse_from(["crunchy-cli", "search", "one piece"]);
        assert!(matches!(cli.command, Commands::Search(_)));
    }

    #[test]
    fn test_download_args() {
        let cli = Cli::parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--quality",
            "1080p",
            "--audio",
            "ja-JP",
            "--audio",
            "en-US",
        ]);
        if let Commands::Download(args) = cli.command {
            assert_eq!(args.quality.height(), Some(1080));
            assert_eq!(args.audio_langs, vec!["ja-JP", "en-US"]);
        } else {
            panic!("Expected Download command");
        }
    }

    #[test]
    fn test_verbose_flag() {
        let cli = Cli::parse_from(["crunchy-cli", "-vvv", "whoami"]);
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn test_only_subs_flag() {
        let cli = Cli::parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-subs",
        ]);
        if let Commands::Download(args) = cli.command {
            assert!(args.only_subs);
            assert!(!args.only_audio);
            assert!(!args.only_video);
        } else {
            panic!("Expected Download command");
        }
    }

    #[test]
    fn test_only_audio_flag() {
        let cli = Cli::parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-audio",
        ]);
        if let Commands::Download(args) = cli.command {
            assert!(!args.only_subs);
            assert!(args.only_audio);
            assert!(!args.only_video);
        } else {
            panic!("Expected Download command");
        }
    }

    #[test]
    fn test_only_video_flag() {
        let cli = Cli::parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-video",
        ]);
        if let Commands::Download(args) = cli.command {
            assert!(!args.only_subs);
            assert!(!args.only_audio);
            assert!(args.only_video);
        } else {
            panic!("Expected Download command");
        }
    }

    #[test]
    fn test_only_flags_mutually_exclusive() {
        // --only-subs and --only-audio should conflict
        let result = Cli::try_parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-subs",
            "--only-audio",
        ]);
        assert!(result.is_err());

        // --only-subs and --only-video should conflict
        let result = Cli::try_parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-subs",
            "--only-video",
        ]);
        assert!(result.is_err());

        // --only-audio and --only-video should conflict
        let result = Cli::try_parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-audio",
            "--only-video",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_only_flags_incompatible_with_skip_mux() {
        let result = Cli::try_parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-subs",
            "--skip-mux",
        ]);
        assert!(result.is_err());

        let result = Cli::try_parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-audio",
            "--skip-mux",
        ]);
        assert!(result.is_err());

        let result = Cli::try_parse_from([
            "crunchy-cli",
            "download",
            "https://crunchyroll.com/series/GXYZ123/test",
            "--only-video",
            "--skip-mux",
        ]);
        assert!(result.is_err());
    }
}
