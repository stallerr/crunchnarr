//! Language code mapping for FFmpeg metadata.
//!
//! Maps Crunchyroll locale codes (e.g., "ja-JP") to ISO 639-2/3 codes
//! used by FFmpeg for track metadata.

/// Language information for FFmpeg metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageItem {
    /// Crunchyroll locale code (e.g., "ja-JP", "en-US").
    pub cr: &'static str,
    /// Standard locale code (e.g., "ja", "en").
    pub locale: &'static str,
    /// ISO 639-2/3 code for FFmpeg (e.g., "jpn", "eng").
    pub code: &'static str,
    /// Human-readable language name (e.g., "Japanese", "English").
    pub name: &'static str,
}

/// All supported languages matching Crunchyroll's available locales.
pub static LANGUAGES: &[LanguageItem] = &[
    LanguageItem {
        cr: "ja-JP",
        locale: "ja",
        code: "jpn",
        name: "Japanese",
    },
    LanguageItem {
        cr: "de-DE",
        locale: "de",
        code: "deu",
        name: "German",
    },
    LanguageItem {
        cr: "en-US",
        locale: "en",
        code: "eng",
        name: "English",
    },
    LanguageItem {
        cr: "en-IN",
        locale: "en-IN",
        code: "eng",
        name: "English (India)",
    },
    LanguageItem {
        cr: "es-419",
        locale: "es-419",
        code: "spa-419",
        name: "Spanish",
    },
    LanguageItem {
        cr: "es-ES",
        locale: "es-ES",
        code: "spa-ES",
        name: "Castilian",
    },
    LanguageItem {
        cr: "pt-BR",
        locale: "pt-BR",
        code: "por",
        name: "Portuguese",
    },
    LanguageItem {
        cr: "pt-PT",
        locale: "pt-PT",
        code: "por",
        name: "Portuguese (Portugal)",
    },
    LanguageItem {
        cr: "fr-FR",
        locale: "fr",
        code: "fra",
        name: "French",
    },
    LanguageItem {
        cr: "it-IT",
        locale: "it",
        code: "ita",
        name: "Italian",
    },
    LanguageItem {
        cr: "pl-PL",
        locale: "pl-PL",
        code: "pol",
        name: "Polish",
    },
    LanguageItem {
        cr: "id-ID",
        locale: "id-ID",
        code: "ind",
        name: "Indonesian",
    },
    LanguageItem {
        cr: "ms-MY",
        locale: "ms-MY",
        code: "may",
        name: "Malay (Malaysia)",
    },
    LanguageItem {
        cr: "ca-ES",
        locale: "ca-ES",
        code: "cat",
        name: "Catalan",
    },
    LanguageItem {
        cr: "vi-VN",
        locale: "vi-VN",
        code: "vie",
        name: "Vietnamese",
    },
    LanguageItem {
        cr: "tr-TR",
        locale: "tr",
        code: "tur",
        name: "Turkish",
    },
    LanguageItem {
        cr: "ru-RU",
        locale: "ru",
        code: "rus",
        name: "Russian",
    },
    LanguageItem {
        cr: "ar-SA",
        locale: "ar-SA",
        code: "ara",
        name: "Arabic",
    },
    LanguageItem {
        cr: "hi-IN",
        locale: "hi",
        code: "hin",
        name: "Hindi",
    },
    LanguageItem {
        cr: "ta-IN",
        locale: "ta-IN",
        code: "tam",
        name: "Tamil (India)",
    },
    LanguageItem {
        cr: "te-IN",
        locale: "te-IN",
        code: "tel",
        name: "Telugu (India)",
    },
    LanguageItem {
        cr: "zh-CN",
        locale: "zh-CN",
        code: "zho",
        name: "Chinese (Mainland China)",
    },
    LanguageItem {
        cr: "zh-HK",
        locale: "zh-HK",
        code: "zho-HK",
        name: "Chinese (Hong Kong)",
    },
    LanguageItem {
        cr: "zh-TW",
        locale: "zh-TW",
        code: "chi",
        name: "Chinese (Taiwan)",
    },
    LanguageItem {
        cr: "ko-KR",
        locale: "ko",
        code: "kor",
        name: "Korean",
    },
    LanguageItem {
        cr: "th-TH",
        locale: "th-TH",
        code: "tha",
        name: "Thai",
    },
];

/// Look up language information by Crunchyroll locale code.
///
/// Supports exact matches (e.g., "ja-JP") and prefix matches (e.g., "ja" matches "ja-JP").
///
/// # Examples
///
/// ```
/// use crunchy_cli::utils::languages::get_language;
///
/// let lang = get_language("ja-JP").unwrap();
/// assert_eq!(lang.code, "jpn");
/// assert_eq!(lang.name, "Japanese");
///
/// // Prefix match also works
/// let lang = get_language("ja").unwrap();
/// assert_eq!(lang.code, "jpn");
/// ```
pub fn get_language(cr_locale: &str) -> Option<&'static LanguageItem> {
    // Try exact match first (case-insensitive)
    if let Some(lang) = LANGUAGES
        .iter()
        .find(|l| l.cr.eq_ignore_ascii_case(cr_locale))
    {
        return Some(lang);
    }

    // Try locale match
    if let Some(lang) = LANGUAGES
        .iter()
        .find(|l| l.locale.eq_ignore_ascii_case(cr_locale))
    {
        return Some(lang);
    }

    // Try prefix match (e.g., "ja" matches "ja-JP")
    let locale_lower = cr_locale.to_lowercase();
    LANGUAGES.iter().find(|l| {
        l.cr.to_lowercase().starts_with(&locale_lower)
            || l.locale.to_lowercase().starts_with(&locale_lower)
            || locale_lower.starts_with(&l.locale.to_lowercase())
    })
}

/// Check if two locale strings refer to the same language.
///
/// This is used for Signs & Songs detection - when the subtitle language
/// matches the audio language, the subtitles are Signs & Songs.
///
/// # Examples
///
/// ```
/// use crunchy_cli::utils::languages::locales_match;
///
/// assert!(locales_match("ja-JP", "ja-JP"));
/// assert!(locales_match("ja-JP", "ja"));
/// assert!(locales_match("en-US", "en"));
/// assert!(!locales_match("ja-JP", "en-US"));
/// ```
pub fn locales_match(locale1: &str, locale2: &str) -> bool {
    // Exact match
    if locale1.eq_ignore_ascii_case(locale2) {
        return true;
    }

    // Extract base language (before the hyphen)
    let base1 = locale1.split('-').next().unwrap_or(locale1);
    let base2 = locale2.split('-').next().unwrap_or(locale2);

    base1.eq_ignore_ascii_case(base2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_exact_match() {
        let lang = get_language("ja-JP").unwrap();
        assert_eq!(lang.code, "jpn");
        assert_eq!(lang.name, "Japanese");

        let lang = get_language("en-US").unwrap();
        assert_eq!(lang.code, "eng");
        assert_eq!(lang.name, "English");
    }

    #[test]
    fn test_get_language_case_insensitive() {
        let lang = get_language("JA-JP").unwrap();
        assert_eq!(lang.code, "jpn");

        let lang = get_language("ja-jp").unwrap();
        assert_eq!(lang.code, "jpn");
    }

    #[test]
    fn test_get_language_prefix_match() {
        let lang = get_language("ja").unwrap();
        assert_eq!(lang.code, "jpn");

        let lang = get_language("en").unwrap();
        assert_eq!(lang.code, "eng");

        let lang = get_language("de").unwrap();
        assert_eq!(lang.code, "deu");
    }

    #[test]
    fn test_get_language_locale_match() {
        let lang = get_language("fr").unwrap();
        assert_eq!(lang.code, "fra");
        assert_eq!(lang.cr, "fr-FR");
    }

    #[test]
    fn test_get_language_not_found() {
        assert!(get_language("xx-XX").is_none());
        assert!(get_language("invalid").is_none());
    }

    #[test]
    fn test_locales_match_exact() {
        assert!(locales_match("ja-JP", "ja-JP"));
        assert!(locales_match("en-US", "en-US"));
    }

    #[test]
    fn test_locales_match_prefix() {
        assert!(locales_match("ja-JP", "ja"));
        assert!(locales_match("ja", "ja-JP"));
        assert!(locales_match("en-US", "en"));
        assert!(locales_match("en", "en-US"));
    }

    #[test]
    fn test_locales_match_different() {
        assert!(!locales_match("ja-JP", "en-US"));
        assert!(!locales_match("ja", "en"));
        assert!(!locales_match("de-DE", "fr-FR"));
    }

    #[test]
    fn test_locales_match_case_insensitive() {
        assert!(locales_match("JA-JP", "ja-jp"));
        assert!(locales_match("EN-US", "en"));
    }

    #[test]
    fn test_all_languages_have_required_fields() {
        for lang in LANGUAGES {
            assert!(!lang.cr.is_empty(), "cr should not be empty");
            assert!(!lang.locale.is_empty(), "locale should not be empty");
            assert!(!lang.code.is_empty(), "code should not be empty");
            assert!(!lang.name.is_empty(), "name should not be empty");
        }
    }
}
