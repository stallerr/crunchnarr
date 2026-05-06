//! Subtitle conversion (VTT to ASS).

use crate::error::{MediaError, Result};
use std::panic;
use tracing::{debug, trace, warn};

/// Converts subtitles to ASS format.
///
/// Handles both VTT and ASS input formats:
/// - VTT files are converted to ASS using rsubs-lib
/// - ASS files are passed through as-is (Crunchyroll often serves ASS directly)
pub struct SubtitleConverter;

impl SubtitleConverter {
    /// Convert subtitle content to ASS format.
    ///
    /// Accepts both VTT and ASS input:
    /// - If content is already ASS format, returns it as-is
    /// - If content is VTT format, converts to ASS
    pub fn to_ass(content: &str) -> Result<String> {
        // Preprocess: trim BOM and whitespace
        let content = content
            .trim_start_matches('\u{FEFF}') // UTF-8 BOM
            .trim();

        // Check if it's already ASS format (Crunchyroll often serves ASS directly)
        if content.starts_with("[Script Info]") || content.contains("[V4+ Styles]") {
            debug!("Content is already ASS format, passing through");
            trace!("ASS content length: {} bytes", content.len());
            return Ok(content.to_string());
        }

        // Check if it's VTT format
        if content.starts_with("WEBVTT") {
            debug!("Converting VTT to ASS");
            trace!("VTT content length: {} bytes", content.len());
            return Self::convert_vtt_to_ass(content);
        }

        // Unknown format
        let preview: String = content.chars().take(100).collect();
        warn!("Unknown subtitle format. First 100 chars: {:?}", preview);
        Err(
            MediaError::SubtitleError("Unknown subtitle format (expected VTT or ASS)".to_string())
                .into(),
        )
    }

    /// Convert VTT content to ASS format using rsubs-lib.
    fn convert_vtt_to_ass(vtt_content: &str) -> Result<String> {
        // Use catch_unwind because rsubs-lib panics on invalid input
        let content_owned = vtt_content.to_string();
        let result = panic::catch_unwind(|| {
            let vtt = rsubs_lib::vtt::parse(content_owned);
            vtt.to_ass().to_string()
        });

        match result {
            Ok(ass) => Ok(ass),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic during VTT parsing".to_string()
                };
                warn!("VTT parsing failed: {}", msg);
                Err(MediaError::SubtitleError(format!("VTT parsing failed: {}", msg)).into())
            }
        }
    }

    /// Legacy alias for backwards compatibility.
    #[deprecated(since = "0.1.0", note = "Use `to_ass` instead")]
    pub fn vtt_to_ass(content: &str) -> Result<String> {
        Self::to_ass(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vtt_to_ass() {
        let vtt = r#"WEBVTT

00:00:01.000 --> 00:00:05.000
Hello, world!

00:00:06.000 --> 00:00:10.000
This is <i>italic</i> text.
"#;

        let ass = SubtitleConverter::to_ass(vtt).unwrap();
        assert!(ass.contains("[Script Info]"));
        assert!(ass.contains("[Events]"));
        assert!(ass.contains("Hello, world!"));
        // rsubs-lib converts <i> to {\i1} style tags
        assert!(ass.contains("italic"));
    }

    #[test]
    fn test_vtt_to_ass_multiline() {
        let vtt = r#"WEBVTT

00:00:01.000 --> 00:00:05.000
Line one
Line two
"#;

        let ass = SubtitleConverter::to_ass(vtt).unwrap();
        assert!(ass.contains("[Script Info]"));
        // rsubs-lib should handle multiline correctly
        assert!(ass.contains("Line one"));
        assert!(ass.contains("Line two"));
    }

    #[test]
    fn test_vtt_with_bom() {
        let vtt = "\u{FEFF}WEBVTT

00:00:01.000 --> 00:00:05.000
Hello with BOM
";

        let ass = SubtitleConverter::to_ass(vtt).unwrap();
        assert!(ass.contains("[Script Info]"));
        assert!(ass.contains("Hello with BOM"));
    }

    #[test]
    fn test_invalid_content_returns_error() {
        let invalid = "This is not a subtitle file";
        let result = SubtitleConverter::to_ass(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_ass_passthrough() {
        let ass = r#"[Script Info]
Title: Test
ScriptType: v4.00+

[V4+ Styles]
Format: Name,Fontname
Style: Default,Arial

[Events]
Format: Layer,Start,End,Text
Dialogue: 0,0:00:01.00,0:00:05.00,Hello
"#;

        let result = SubtitleConverter::to_ass(ass).unwrap();
        assert!(result.contains("[Script Info]"));
        assert!(result.contains("Title: Test"));
    }

    #[test]
    fn test_ass_with_bom() {
        let ass = "\u{FEFF}[Script Info]
Title: Test with BOM
ScriptType: v4.00+
";

        let result = SubtitleConverter::to_ass(ass).unwrap();
        assert!(result.contains("[Script Info]"));
        assert!(result.contains("Title: Test with BOM"));
    }
}
