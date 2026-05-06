//! Output filename generation.

use regex::Regex;
use std::path::PathBuf;
use std::sync::LazyLock;

/// Invalid filename characters regex.
static INVALID_CHARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#).unwrap());

/// Multiple spaces/underscores regex.
static MULTI_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\s_]+").unwrap());

/// Generates output filenames from templates.
pub struct FilenameGenerator {
    template: String,
    base_dir: PathBuf,
}

/// Variables available for filename templates.
#[derive(Debug, Clone, Default)]
pub struct FilenameVars {
    pub series: String,
    pub season: u32,
    pub season_title: String,
    pub episode: String,
    pub episode_number: Option<f32>,
    pub title: String,
    pub quality: String,
    pub audio: String,
    pub year: String,
}

impl FilenameGenerator {
    /// Create a new filename generator.
    pub fn new(template: &str, base_dir: PathBuf) -> Self {
        Self {
            template: template.to_string(),
            base_dir,
        }
    }

    /// Generate a filename from the template and variables.
    pub fn generate(&self, vars: &FilenameVars, extension: &str) -> PathBuf {
        self.generate_with_suffix(vars, None, extension)
    }

    /// Generate a filename from the template and variables with an optional suffix.
    ///
    /// The suffix is inserted before the extension, e.g.:
    /// - `suffix: None` -> "S01E05 - Title.mkv"
    /// - `suffix: Some("subtitles")` -> "S01E05 - Title.subtitles.mkv"
    pub fn generate_with_suffix(
        &self,
        vars: &FilenameVars,
        suffix: Option<&str>,
        extension: &str,
    ) -> PathBuf {
        let mut result = self.template.clone();

        // Replace variables (longer patterns first to avoid partial matches)
        result = result.replace("{series}", &Self::sanitize(&vars.series));
        result = result.replace("{season_title}", &Self::sanitize(&vars.season_title));
        result = result.replace("{season:02}", &format!("{:02}", vars.season));
        result = result.replace("{season}", &vars.season.to_string());

        if let Some(ep_num) = vars.episode_number {
            if ep_num.fract() == 0.0 {
                result = result.replace("{episode:02}", &format!("{:02}", ep_num as u32));
            } else {
                result = result.replace("{episode:02}", &format!("{:04.1}", ep_num));
            }
        } else {
            result = result.replace("{episode:02}", &Self::sanitize(&vars.episode));
        }
        result = result.replace("{episode}", &Self::sanitize(&vars.episode));

        result = result.replace("{title}", &Self::sanitize(&vars.title));
        result = result.replace("{quality}", &vars.quality);
        result = result.replace("{audio}", &vars.audio);
        result = result.replace("{year}", &vars.year);

        // Build full path with optional suffix
        let filename = match suffix {
            Some(s) => format!("{}.{}.{}", result, s, extension),
            None => format!("{}.{}", result, extension),
        };
        self.base_dir.join(filename)
    }

    /// Sanitize a string for use in filenames.
    pub fn sanitize(input: &str) -> String {
        // Remove invalid characters
        let result = INVALID_CHARS.replace_all(input, "");

        // Replace multiple spaces/underscores with single space
        let result = MULTI_SPACE.replace_all(&result, " ");

        // Trim whitespace
        let result = result.trim();

        // Truncate if too long
        if result.len() > 200 {
            result[..200].to_string()
        } else {
            result.to_string()
        }
    }

    /// Generate just the series folder path.
    pub fn series_folder(&self, series_name: &str) -> PathBuf {
        self.base_dir.join(Self::sanitize(series_name))
    }

    /// Generate the season folder path.
    pub fn season_folder(&self, series_name: &str, season: u32) -> PathBuf {
        self.series_folder(series_name)
            .join(format!("Season {:02}", season))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize() {
        assert_eq!(FilenameGenerator::sanitize("Hello: World"), "Hello World");
        assert_eq!(
            FilenameGenerator::sanitize("Test/File\\Name"),
            "TestFileName"
        );
        assert_eq!(
            FilenameGenerator::sanitize("Multiple   Spaces"),
            "Multiple Spaces"
        );
    }

    #[test]
    fn test_generate_filename() {
        let gen = FilenameGenerator::new(
            "{series}/S{season:02}E{episode:02} - {title}",
            PathBuf::from("/downloads"),
        );

        let vars = FilenameVars {
            series: "One Piece".to_string(),
            season: 1,
            season_title: "East Blue Saga".to_string(),
            episode: "5".to_string(),
            episode_number: Some(5.0),
            title: "The King of the Pirates".to_string(),
            quality: "1080p".to_string(),
            audio: "ja-JP".to_string(),
            year: "1999".to_string(),
        };

        let path = gen.generate(&vars, "mkv");
        assert_eq!(
            path,
            PathBuf::from("/downloads/One Piece/S01E05 - The King of the Pirates.mkv")
        );
    }

    #[test]
    fn test_generate_with_new_vars() {
        let gen = FilenameGenerator::new(
            "{series} ({year})/{season_title}/E{episode:02} - {title} [{quality}] [{audio}]",
            PathBuf::from("/downloads"),
        );

        let vars = FilenameVars {
            series: "One Piece".to_string(),
            season: 1,
            season_title: "East Blue Saga".to_string(),
            episode: "5".to_string(),
            episode_number: Some(5.0),
            title: "The King of the Pirates".to_string(),
            quality: "1080p".to_string(),
            audio: "ja-JP".to_string(),
            year: "1999".to_string(),
        };

        let path = gen.generate(&vars, "mkv");
        assert_eq!(
            path,
            PathBuf::from("/downloads/One Piece (1999)/East Blue Saga/E05 - The King of the Pirates [1080p] [ja-JP].mkv")
        );
    }

    #[test]
    fn test_decimal_episode_number() {
        let gen = FilenameGenerator::new("E{episode:02} - {title}", PathBuf::from("/downloads"));

        let vars = FilenameVars {
            episode_number: Some(5.5),
            title: "Special".to_string(),
            ..Default::default()
        };

        let path = gen.generate(&vars, "mkv");
        assert!(path.to_string_lossy().contains("E05.5"));
    }

    #[test]
    fn test_generate_with_suffix() {
        let gen = FilenameGenerator::new(
            "{series}/S{season:02}E{episode:02} - {title}",
            PathBuf::from("/downloads"),
        );

        let vars = FilenameVars {
            series: "One Piece".to_string(),
            season: 1,
            season_title: "East Blue Saga".to_string(),
            episode: "5".to_string(),
            episode_number: Some(5.0),
            title: "The King of the Pirates".to_string(),
            quality: "1080p".to_string(),
            audio: "ja-JP".to_string(),
            year: "1999".to_string(),
        };

        // Test with subtitles suffix
        let path = gen.generate_with_suffix(&vars, Some("subtitles"), "mkv");
        assert_eq!(
            path,
            PathBuf::from("/downloads/One Piece/S01E05 - The King of the Pirates.subtitles.mkv")
        );

        // Test with audios suffix
        let path = gen.generate_with_suffix(&vars, Some("audios"), "mkv");
        assert_eq!(
            path,
            PathBuf::from("/downloads/One Piece/S01E05 - The King of the Pirates.audios.mkv")
        );

        // Test with video suffix
        let path = gen.generate_with_suffix(&vars, Some("video"), "mkv");
        assert_eq!(
            path,
            PathBuf::from("/downloads/One Piece/S01E05 - The King of the Pirates.video.mkv")
        );

        // Test without suffix (same as generate)
        let path = gen.generate_with_suffix(&vars, None, "mkv");
        assert_eq!(
            path,
            PathBuf::from("/downloads/One Piece/S01E05 - The King of the Pirates.mkv")
        );
    }
}
