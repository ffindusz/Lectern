//! GitHub-style heading slugs.
//!
//! Matches the behaviour of `github-slugger` (what github.com uses for heading anchors):
//! lowercase, drop everything that is not a letter, number, `_`, `-` or space, then turn
//! spaces into `-`. Repeated slugs get `-1`, `-2`, … suffixes.

use std::collections::HashMap;

/// Slugify a single heading text without de-duplication.
pub fn slugify(text: &str) -> String {
    text.trim()
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            ' ' => Some('-'),
            '-' | '_' => Some(c),
            c if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

/// Produces unique slugs for a sequence of headings within one document.
#[derive(Debug, Default)]
pub struct Slugger {
    occurrences: HashMap<String, usize>,
}

impl Slugger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Slug for `text`, unique among all slugs previously returned by this slugger.
    pub fn slug(&mut self, text: &str) -> String {
        let mut base = slugify(text);
        if base.is_empty() {
            base = "section".to_owned();
        }
        let mut result = base.clone();
        while self.occurrences.contains_key(&result) {
            let n = self
                .occurrences
                .get_mut(&base)
                .expect("base slug is always registered before suffixing");
            *n += 1;
            result = format!("{base}-{n}");
        }
        self.occurrences.insert(result.clone(), 0);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("  Trim me  "), "trim-me");
        assert_eq!(slugify("API v2.0: What's new?"), "api-v20-whats-new");
        assert_eq!(
            slugify("snake_case and kebab-case"),
            "snake_case-and-kebab-case"
        );
        assert_eq!(slugify("C++ & Rust"), "c--rust");
    }

    #[test]
    fn unicode_is_kept() {
        assert_eq!(slugify("Über Straße"), "über-straße");
        assert_eq!(slugify("日本語 見出し"), "日本語-見出し");
    }

    #[test]
    fn duplicates_get_suffixes() {
        let mut s = Slugger::new();
        assert_eq!(s.slug("Foo"), "foo");
        assert_eq!(s.slug("Foo"), "foo-1");
        assert_eq!(s.slug("foo"), "foo-2");
        assert_eq!(s.slug("Bar"), "bar");
    }

    #[test]
    fn suffix_does_not_collide_with_literal_heading() {
        // A heading literally named "foo-1" must not be duplicated by the suffixing scheme.
        let mut s = Slugger::new();
        assert_eq!(s.slug("foo"), "foo");
        assert_eq!(s.slug("foo-1"), "foo-1");
        assert_eq!(s.slug("foo"), "foo-2");
    }

    #[test]
    fn empty_heading_gets_fallback() {
        let mut s = Slugger::new();
        assert_eq!(s.slug("!!!"), "section");
        assert_eq!(s.slug(""), "section-1");
    }
}
