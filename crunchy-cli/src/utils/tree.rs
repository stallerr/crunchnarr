//! Tree rendering utilities for CLI output.
//!
//! Provides helpers to display hierarchical data using Unicode box-drawing characters.

/// Tree branch characters for rendering.
pub mod chars {
    /// Branch connector for non-last items: `├── `
    pub const BRANCH: &str = "\u{251C}\u{2500}\u{2500} ";
    /// Branch connector for last item: `└── `
    pub const LAST_BRANCH: &str = "\u{2514}\u{2500}\u{2500} ";
    /// Vertical line for continuing branches: `│   `
    pub const VERTICAL: &str = "\u{2502}   ";
    /// Empty space for completed branches: `    `
    pub const EMPTY: &str = "    ";
}

/// Print a tree item with the appropriate connector.
///
/// # Arguments
/// * `prefix` - The accumulated prefix from parent levels
/// * `is_last` - Whether this is the last sibling at this level
/// * `content` - The content to display
pub fn print_tree_item(prefix: &str, is_last: bool, content: &str) {
    let connector = if is_last {
        chars::LAST_BRANCH
    } else {
        chars::BRANCH
    };
    println!("{}{}{}", prefix, connector, content);
}

/// Get the prefix to use for children of the current item.
///
/// # Arguments
/// * `parent_prefix` - The prefix used for the parent item
/// * `is_last` - Whether the parent was the last sibling at its level
pub fn child_prefix(parent_prefix: &str, is_last: bool) -> String {
    let extension = if is_last {
        chars::EMPTY
    } else {
        chars::VERTICAL
    };
    format!("{}{}", parent_prefix, extension)
}

/// Format a list of locales for display.
///
/// # Example
/// ```
/// use crunchy_cli::utils::tree::format_locales;
/// let locales = vec!["ja-JP".to_string(), "en-US".to_string()];
/// assert_eq!(format_locales(&locales), "ja-JP, en-US");
/// ```
pub fn format_locales(locales: &[String]) -> String {
    locales.join(", ")
}

/// Format subtitle locales for display.
///
/// Returns a formatted string like "subs: en-US, es-LA" or empty string if no subs.
pub fn format_subs(locales: &[String]) -> String {
    if locales.is_empty() {
        String::new()
    } else {
        format!("subs: {}", locales.join(", "))
    }
}

/// Truncate a string to a maximum length, adding ellipsis if needed.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_locales() {
        let locales = vec!["ja-JP".to_string(), "en-US".to_string()];
        assert_eq!(format_locales(&locales), "ja-JP, en-US");

        let empty: Vec<String> = vec![];
        assert_eq!(format_locales(&empty), "");
    }

    #[test]
    fn test_format_subs() {
        let locales = vec!["en-US".to_string(), "es-LA".to_string()];
        assert_eq!(format_subs(&locales), "subs: en-US, es-LA");

        let empty: Vec<String> = vec![];
        assert_eq!(format_subs(&empty), "");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a long string", 10), "this is...");
        assert_eq!(truncate("abc", 3), "abc");
        assert_eq!(truncate("abcd", 3), "...");
    }

    #[test]
    fn test_child_prefix() {
        assert_eq!(child_prefix("", false), chars::VERTICAL);
        assert_eq!(child_prefix("", true), chars::EMPTY);
        assert_eq!(
            child_prefix(chars::VERTICAL, false),
            format!("{}{}", chars::VERTICAL, chars::VERTICAL)
        );
    }
}
