//! Syntax highlighting with syntect, emitting CSS classes (not inline colours) so the
//! active theme decides the palette. Classes are prefixed with `hl-`, e.g. a Rust keyword
//! becomes `<span class="hl-keyword hl-control hl-rust">`.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

use comrak::adapters::SyntaxHighlighterAdapter;
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Don't try to highlight enormous blocks; escaping them is instant, highlighting is not.
const MAX_HIGHLIGHT_BYTES: usize = 256 * 1024;

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Load the syntax definitions ahead of time (call from a background thread at startup).
pub fn warm_up() {
    LazyLock::force(&SYNTAXES);
}

/// Common aliases that syntect's default set doesn't know by token.
fn canonical_lang(lang: &str) -> &str {
    match lang {
        "sh" | "shell" | "zsh" | "console" => "bash",
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "typescript" | "json5" => "js",
        "py" | "python3" => "python",
        "rs" => "rust",
        "yml" => "yaml",
        "c++" | "hpp" | "cc" => "cpp",
        "csharp" | "c#" => "cs",
        "golang" => "go",
        "md" => "markdown",
        "htm" | "xhtml" => "html",
        "ps1" | "powershell" | "toml" | "ini" | "dockerfile" => "plain",
        other => other,
    }
}

/// Highlight `code` written in `lang` to HTML (without the `<pre><code>` wrapper).
pub fn highlight(lang: &str, code: &str) -> String {
    let lang = canonical_lang(&lang.to_ascii_lowercase()).to_owned();
    if lang.is_empty() || lang == "plain" || lang == "text" || code.len() > MAX_HIGHLIGHT_BYTES {
        return escape_html(code);
    }
    let ss = &*SYNTAXES;
    let Some(syntax) = ss
        .find_syntax_by_token(&lang)
        .or_else(|| ss.find_syntax_by_name(&lang))
    else {
        return escape_html(code);
    };
    let mut generator = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        ss,
        ClassStyle::SpacedPrefixed { prefix: "hl-" },
    );
    for line in LinesWithEndings::from(code) {
        if generator
            .parse_html_for_line_which_includes_newline(line)
            .is_err()
        {
            return escape_html(code);
        }
    }
    generator.finalize()
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// comrak plugin adapter.
pub struct SyntectAdapter;

impl SyntaxHighlighterAdapter for SyntectAdapter {
    fn write_highlighted(
        &self,
        output: &mut dyn fmt::Write,
        lang: Option<&str>,
        code: &str,
    ) -> fmt::Result {
        output.write_str(&highlight(lang.unwrap_or(""), code))
    }

    fn write_pre_tag(
        &self,
        output: &mut dyn fmt::Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        comrak::html::write_opening_tag(output, "pre", attributes)
    }

    fn write_code_tag(
        &self,
        output: &mut dyn fmt::Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        comrak::html::write_opening_tag(output, "code", attributes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_gets_classes() {
        let html = highlight("rust", "fn main() {}\n");
        assert!(
            html.contains("hl-keyword") || html.contains("hl-storage"),
            "{html}"
        );
        assert!(html.contains("hl-entity"), "{html}");
        assert!(
            !html.contains("style="),
            "must not emit inline colours: {html}"
        );
    }

    #[test]
    fn aliases_resolve() {
        assert!(highlight("py", "def f(): pass\n").contains("hl-"));
        assert!(highlight("sh", "echo hi\n").contains("hl-"));
    }

    #[test]
    fn unknown_language_is_escaped_plain() {
        assert_eq!(
            highlight("nonexistent-lang", "<b>&</b>"),
            "&lt;b&gt;&amp;&lt;/b&gt;"
        );
        assert_eq!(highlight("", "a < b"), "a &lt; b");
    }
}
