//! Range parsing utilities for episode/season selection.

use std::collections::BTreeSet;

/// A set of numbers parsed from a range expression.
#[derive(Debug, Clone, Default)]
pub struct RangeSet {
    values: BTreeSet<u32>,
}

impl RangeSet {
    /// Create an empty range set.
    pub fn new() -> Self {
        Self {
            values: BTreeSet::new(),
        }
    }

    /// Check if a value is in the set.
    pub fn contains(&self, value: u32) -> bool {
        self.values.contains(&value)
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get all values as a sorted vector.
    pub fn values(&self) -> Vec<u32> {
        self.values.iter().copied().collect()
    }

    /// Get the number of values in the set.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Add a single value.
    pub fn add(&mut self, value: u32) {
        self.values.insert(value);
    }

    /// Add a range of values (inclusive).
    pub fn add_range(&mut self, start: u32, end: u32) {
        for v in start..=end {
            self.values.insert(v);
        }
    }
}

/// Parse a range expression into a RangeSet.
///
/// Supports:
/// - Single numbers: "5"
/// - Ranges: "1-5"
/// - Comma-separated: "1,3,5"
/// - Combined: "1-3,5,7-9"
///
/// # Examples
///
/// ```
/// use crunchy_cli::utils::parse_range;
///
/// let range = parse_range("1-3,5,7-9").unwrap();
/// assert!(range.contains(1));
/// assert!(range.contains(5));
/// assert!(range.contains(8));
/// assert!(!range.contains(6));
/// ```
pub fn parse_range(input: &str) -> Result<RangeSet, String> {
    let mut set = RangeSet::new();

    if input.trim().is_empty() {
        return Ok(set);
    }

    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((start, end)) = part.split_once('-') {
            let start: u32 = start
                .trim()
                .parse()
                .map_err(|_| format!("Invalid number: {}", start))?;
            let end: u32 = end
                .trim()
                .parse()
                .map_err(|_| format!("Invalid number: {}", end))?;

            if start > end {
                return Err(format!("Invalid range: {} > {}", start, end));
            }

            set.add_range(start, end);
        } else {
            let value: u32 = part
                .parse()
                .map_err(|_| format!("Invalid number: {}", part))?;
            set.add(value);
        }
    }

    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_number() {
        let set = parse_range("5").unwrap();
        assert!(set.contains(5));
        assert!(!set.contains(4));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_range() {
        let set = parse_range("1-5").unwrap();
        assert!(set.contains(1));
        assert!(set.contains(3));
        assert!(set.contains(5));
        assert!(!set.contains(0));
        assert!(!set.contains(6));
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_comma_separated() {
        let set = parse_range("1,3,5").unwrap();
        assert!(set.contains(1));
        assert!(!set.contains(2));
        assert!(set.contains(3));
        assert!(!set.contains(4));
        assert!(set.contains(5));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_combined() {
        let set = parse_range("1-3,5,7-9").unwrap();
        let values = set.values();
        assert_eq!(values, vec![1, 2, 3, 5, 7, 8, 9]);
    }

    #[test]
    fn test_empty() {
        let set = parse_range("").unwrap();
        assert!(set.is_empty());
    }

    #[test]
    fn test_invalid_range() {
        assert!(parse_range("5-3").is_err());
    }

    #[test]
    fn test_invalid_number() {
        assert!(parse_range("abc").is_err());
        assert!(parse_range("1-abc").is_err());
    }

    #[test]
    fn test_with_spaces() {
        let set = parse_range(" 1 - 3 , 5 , 7 - 9 ").unwrap();
        let values = set.values();
        assert_eq!(values, vec![1, 2, 3, 5, 7, 8, 9]);
    }
}
