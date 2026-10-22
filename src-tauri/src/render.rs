//! Markdown → sanitized HTML + table of contents.
//!
//! Pure library code with no Tauri dependency so it can be unit-tested in isolation.
//!
//! Pipeline: comrak (GFM) with plugins for heading ids/TOC, syntect highlighting and
//! diagram blocks → front matter pulled out into a `<details>` block → ammonia sanitizer
//! (which also rewrites local image paths to the app's image protocol).

use std::borrow::Cow;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use comrak::adapters::{CodefenceRendererAdapter, HeadingAdapter, HeadingMeta};
use comrak::nodes::{NodeValue, Sourcepos};
use comrak::options::Plugins;
use comrak::{Arena, Options, format_html_with_plugins, parse_document};
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use serde::Serialize;

use crate::highlight::{SyntectAdapter, escape_html};
use crate::slug::Slugger;

/// Headings up to this level appear in the table of contents (all headings get ids).
pub const TOC_MAX_LEVEL: u8 = 4;

#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Directory of the Markdown file. Relative image paths resolve against it.
    pub base_dir: Option<PathBuf>,
    /// URL prefix for local images, e.g. `mdfile://localhost/`. The absolute file path is
    /// percent-encoded (as a single path segment) and appended. `None` leaves `src` untouched.
    pub local_image_prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TocEntry {
    pub level: u8,
    pub text: String,
    pub id: String,
}

#[derive(Debug, Clone, Default)]
pub struct Rendered {
    /// Sanitized HTML, safe to inject into the page.
    pub html: String,
    /// Headings h1–h4 in document order.
    pub toc: Vec<TocEntry>,
    /// Text of the first h1, if any.
    pub title: Option<String>,
    /// Absolute paths of local images referenced by the document (the image protocol's allowlist).
    pub local_images: Vec<PathBuf>,
}

/// Render Markdown to sanitized HTML.
pub fn render(markdown: &str, opts: &RenderOptions) -> Rendered {
    let options = comrak_options();

    let headings = HeadingIds::default();
    let diagrams = DiagramBlock;
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&SyntectAdapter);
    plugins.render.heading_adapter = Some(&headings);
    for lang in ["mermaid", "dot", "graphviz"] {
        plugins
            .render
            .codefence_renderers
            .insert(lang.to_owned(), &diagrams);
    }

    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options);

    let mut html = String::with_capacity(markdown.len() * 2);
    for node in root.children() {
        if let NodeValue::FrontMatter(fm) = &node.data().value {
            html.push_str(&front_matter_html(fm));
        }
    }
    if format_html_with_plugins(root, &options, &mut html, &plugins).is_err() {
        // Only possible if a writer fails; ours write to a String. Keep whatever we have.
        html.push_str("<p><em>Rendering failed.</em></p>");
    }

    let toc = std::mem::take(&mut lock(&headings.state).toc);
    let title = toc.iter().find(|e| e.level == 1).map(|e| e.text.clone());

    let local_images = Arc::new(Mutex::new(Vec::new()));
    let html = sanitizer(opts, Arc::clone(&local_images))
        .clean(&html)
        .to_string();
    let local_images = std::mem::take(&mut *lock(&local_images));

    Rendered {
        html,
        toc,
        title,
        local_images,
    }
}

fn comrak_options() -> Options<'static> {
    let mut o = Options::default();
    let e = &mut o.extension;
    e.strikethrough = true;
    e.table = true;
    e.autolink = true;
    e.tasklist = true;
    e.footnotes = true;
    e.front_matter_delimiter = Some("---".to_owned());
    e.math_dollars = true;
    e.math_code = true;
    e.alerts = true;
    e.superscript = false;
    o.render.r#unsafe = true; // raw HTML is allowed; ammonia sanitizes it afterwards
    o.render.tasklist_classes = true;
    o
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------------------
// Headings: stable GitHub-style ids, collected into the TOC as they are rendered.

#[derive(Default)]
struct HeadingIds {
    state: Mutex<HeadingState>,
}

#[derive(Default)]
struct HeadingState {
    slugger: Slugger,
    toc: Vec<TocEntry>,
}

impl HeadingAdapter for HeadingIds {
    fn enter(
        &self,
        output: &mut dyn fmt::Write,
        heading: &HeadingMeta,
        _sourcepos: Option<Sourcepos>,
    ) -> fmt::Result {
        let mut st = lock(&self.state);
        let text = heading
            .content
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let id = st.slugger.slug(&text);
        write!(output, "<h{} id=\"{}\">", heading.level, escape_html(&id))?;
        if heading.level <= TOC_MAX_LEVEL {
            st.toc.push(TocEntry {
                level: heading.level,
                text,
                id,
            });
        }
        Ok(())
    }

    fn exit(&self, output: &mut dyn fmt::Write, heading: &HeadingMeta) -> fmt::Result {
        writeln!(output, "</h{}>", heading.level)
    }
}

// ---------------------------------------------------------------------------------------
// Diagram blocks are emitted as plain escaped source; the frontend renders them.

struct DiagramBlock;

impl CodefenceRendererAdapter for DiagramBlock {
    fn write(
        &self,
        output: &mut dyn fmt::Write,
        lang: &str,
        _meta: &str,
        code: &str,
        _sourcepos: Option<Sourcepos>,
    ) -> fmt::Result {
        let kind = if lang == "mermaid" { "mermaid" } else { "dot" };
        writeln!(
            output,
            "<pre class=\"diagram-source\" data-diagram=\"{kind}\"><code>{}</code></pre>",
            escape_html(code)
        )
    }
}

// ---------------------------------------------------------------------------------------
// Front matter

fn front_matter_html(raw: &str) -> String {
    let body: Vec<&str> = raw
        .trim()
        .lines()
        .filter(|l| l.trim_end() != "---")
        .collect();
    let body = body.join("\n");
    if body.trim().is_empty() {
        return String::new();
    }
    format!(
        "<details class=\"front-matter\"><summary>Front matter</summary>         <pre><code class=\"language-yaml\">{}</code></pre></details>
",
        crate::highlight::highlight("yaml", &body)
    )
}

// ---------------------------------------------------------------------------------------
// Sanitizer

fn sanitizer(
    opts: &RenderOptions,
    local_images: Arc<Mutex<Vec<PathBuf>>>,
) -> ammonia::Builder<'static> {
    let base_dir = opts.base_dir.clone();
    let prefix = opts.local_image_prefix.clone();

    let mut b = ammonia::Builder::default();
    b.add_tags(&[
        "section",
        "input",
        "details",
        "summary",
        "mark",
        "figure",
        "figcaption",
    ])
    .add_generic_attributes(&["id", "class", "align", "aria-hidden", "aria-label", "role"])
    .add_generic_attribute_prefixes(&["data-"])
    .add_tag_attributes("input", &["checked"])
    .add_tag_attributes("img", &["loading"])
    .add_tag_attributes("details", &["open"])
    .add_tag_attributes("ol", &["type"])
    // The only inputs Markdown produces are task-list checkboxes. Force every <input>
    // (including raw-HTML text fields) to be a disabled checkbox.
    .set_tag_attribute_value("input", "type", "checkbox")
    .set_tag_attribute_value("input", "disabled", "")
    .add_url_schemes(&["data", "mdfile"])
    .url_relative(ammonia::UrlRelative::PassThrough)
    .link_rel(Some("noopener noreferrer"))
    .strip_comments(true)
    .attribute_filter(move |element, attribute, value| {
        filter_attribute(
            element,
            attribute,
            value,
            base_dir.as_deref(),
            prefix.as_deref(),
            &local_images,
        )
    });
    b
}

fn filter_attribute<'u>(
    element: &str,
    attribute: &str,
    value: &'u str,
    base_dir: Option<&Path>,
    prefix: Option<&str>,
    local_images: &Mutex<Vec<PathBuf>>,
) -> Option<Cow<'u, str>> {
    let is_data_url = value
        .trim_start()
        .get(..5)
        .is_some_and(|s| s.eq_ignore_ascii_case("data:"));
    match (element, attribute) {
        ("img", "src") => {
            if is_data_url {
                let image = value.trim_start()[5..]
                    .to_ascii_lowercase()
                    .starts_with("image/");
                return image.then_some(Cow::Borrowed(value));
            }
            let (Some(prefix), Some(path)) = (prefix, local_image_path(value, base_dir)) else {
                return Some(Cow::Borrowed(value));
            };
            let url = format!(
                "{prefix}{}",
                utf8_percent_encode(&path.to_string_lossy(), NON_ALPHANUMERIC)
            );
            lock(local_images).push(path);
            Some(Cow::Owned(url))
        }
        // data: URLs are only allowed for images.
        (_, "href" | "src" | "cite") if is_data_url => None,
        _ => Some(Cow::Borrowed(value)),
    }
}

/// If `src` points to a local file, return its absolute, normalized path.
pub fn local_image_path(src: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    let src = src.trim();
    if src.is_empty() || src.starts_with("//") || src.starts_with('#') {
        return None;
    }
    let path_part = if let Some(rest) = strip_prefix_ci(src, "file://") {
        // file:///home/x.png → /home/x.png ; file:///C:/x.png → C:/x.png
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        if cfg!(windows) {
            rest.trim_start_matches('/')
        } else {
            rest
        }
    } else if has_url_scheme(src) {
        return None;
    } else {
        src
    };
    let path_part = path_part.split(['?', '#']).next().unwrap_or_default();
    let decoded = percent_decode_str(path_part).decode_utf8().ok()?;
    let path = PathBuf::from(decoded.as_ref());
    let abs = if path.is_absolute() {
        path
    } else {
        base_dir?.join(path)
    };
    Some(normalize(&abs))
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

/// `scheme:` per RFC 3986. A single letter followed by `:` is treated as a Windows drive.
fn has_url_scheme(s: &str) -> bool {
    let Some(colon) = s.find(':') else {
        return false;
    };
    let scheme = &s[..colon];
    if scheme.len() == 1 && cfg!(windows) {
        return false;
    }
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// Lexically resolve `.` and `..` (the file may not exist; no symlink resolution).
pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(md: &str) -> Rendered {
        render(md, &RenderOptions::default())
    }

    #[test]
    fn gfm_elements() {
        let html = r("| a | b |\n|:--|--:|\n| 1 | 2 |\n\n\
                      - [x] done\n- [ ] todo\n\n\
                      ~~gone~~ and https://example.com\n\n\
                      Note[^1].\n\n[^1]: The footnote.\n")
        .html;
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("<th align=\"left\">a</th>"), "{html}");
        assert!(html.contains("type=\"checkbox\""), "{html}");
        assert!(html.contains("checked"), "{html}");
        assert!(html.contains("disabled"), "{html}");
        assert!(html.contains("<del>gone</del>"), "{html}");
        assert!(html.contains("<a href=\"https://example.com\""), "{html}");
        assert!(html.contains("class=\"footnotes\""), "{html}");
        assert!(html.contains("id=\"fn-1\""), "{html}");
    }

    #[test]
    fn headings_get_ids_and_toc() {
        let out = r("# Title\n\n## Intro\n\n## Intro\n\n### Deep `code`\n\n##### Five\n");
        assert!(
            out.html.contains("<h1 id=\"title\">Title</h1>"),
            "{}",
            out.html
        );
        assert!(out.html.contains("<h2 id=\"intro\">"), "{}", out.html);
        assert!(out.html.contains("<h2 id=\"intro-1\">"), "{}", out.html);
        assert!(out.html.contains("<h5 id=\"five\">"), "{}", out.html);
        let ids: Vec<_> = out.toc.iter().map(|e| (e.level, e.id.as_str())).collect();
        assert_eq!(
            ids,
            [(1, "title"), (2, "intro"), (2, "intro-1"), (3, "deep-code")],
            "h5 is excluded from the TOC"
        );
        assert_eq!(out.toc[3].text, "Deep code");
        assert_eq!(out.title.as_deref(), Some("Title"));
    }

    #[test]
    fn script_and_handlers_are_stripped() {
        let html = r("<script>alert(1)</script>\n\n\
                      <p onclick=\"alert(2)\">x</p>\n\n\
                      <a href=\"javascript:alert(3)\">y</a>\n\n\
                      <iframe src=\"https://evil.example\"></iframe>\n\n\
                      <style>body{display:none}</style>\n\n\
                      <img src=\"x.png\" onerror=\"alert(4)\">\n\n\
                      [z](javascript:alert(5))\n")
        .html;
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("alert(1)"), "{html}");
        assert!(!html.contains("onclick"), "{html}");
        assert!(!html.contains("javascript:"), "{html}");
        assert!(!html.contains("<iframe"), "{html}");
        assert!(!html.contains("<style"), "{html}");
        assert!(!html.contains("onerror"), "{html}");
    }

    #[test]
    fn raw_html_allowed_subset_survives() {
        let html =
            r("<details><summary>More</summary>\n\nHidden <kbd>Ctrl</kbd>\n\n</details>\n").html;
        assert!(html.contains("<details>"), "{html}");
        assert!(html.contains("<summary>More</summary>"), "{html}");
        assert!(html.contains("<kbd>Ctrl</kbd>"), "{html}");
    }

    #[test]
    fn inputs_become_disabled_checkboxes() {
        let html = r("<input type=\"text\" value=\"x\"> <input type=\"password\"> <input>\n").html;
        assert!(!html.contains("text"), "{html}");
        assert!(!html.contains("password"), "{html}");
        assert!(!html.contains("value="), "{html}");
        assert_eq!(html.matches("type=\"checkbox\"").count(), 3, "{html}");
        assert_eq!(html.matches("disabled").count(), 3, "{html}");
    }

    #[test]
    fn code_is_highlighted_with_classes() {
        let html = r("```rust\nfn main() {}\n```\n").html;
        assert!(html.contains("class=\"language-rust\""), "{html}");
        assert!(html.contains("hl-"), "{html}");
        assert!(!html.contains("style="), "{html}");
    }

    #[test]
    fn diagrams_are_left_for_the_frontend() {
        let html = r("```mermaid\ngraph TD; A-->B;\n```\n\n```dot\ndigraph { a -> b }\n```\n").html;
        assert!(html.contains("data-diagram=\"mermaid\""), "{html}");
        assert!(html.contains("graph TD; A--&gt;B;"), "{html}");
        assert!(html.contains("data-diagram=\"dot\""), "{html}");
        assert!(html.contains("a -&gt; b"), "{html}");
    }

    #[test]
    fn math_spans() {
        let html = r("Inline $a^2$ and\n\n$$\nb^2\n$$\n").html;
        assert!(html.contains("data-math-style=\"inline\""), "{html}");
        assert!(html.contains("data-math-style=\"display\""), "{html}");
    }

    #[test]
    fn front_matter_is_collapsed_not_rendered() {
        let html = r("---\ntitle: Hello\ntags: [a]\n---\n\n# Body\n").html;
        assert!(html.contains("class=\"front-matter\""), "{html}");
        assert!(html.contains("title"), "{html}");
        assert!(!html.contains("<hr"), "{html}");
        assert!(html.contains("<h1 id=\"body\">"), "{html}");
    }

    #[test]
    fn local_images_are_rewritten_and_recorded() {
        let base = std::env::temp_dir().join("docs");
        let opts = RenderOptions {
            base_dir: Some(base.clone()),
            local_image_prefix: Some("mdfile://localhost/".into()),
        };
        let out = render(
            "![a](img/a%20b.png) ![b](../up.svg) ![c](https://example.com/r.png) ![d](data:image/png;base64,AAAA)\n\n<img src=\"raw.gif\">\n",
            &opts,
        );
        assert_eq!(
            out.local_images,
            vec![
                base.join("img").join("a b.png"),
                normalize(&base.join("..").join("up.svg")),
                base.join("raw.gif"),
            ]
        );
        let encoded = utf8_percent_encode(
            &base.join("img").join("a b.png").to_string_lossy(),
            NON_ALPHANUMERIC,
        )
        .to_string();
        assert!(
            out.html
                .contains(&format!("src=\"mdfile://localhost/{encoded}\"")),
            "{}",
            out.html
        );
        assert!(
            out.html.contains("src=\"https://example.com/r.png\""),
            "{}",
            out.html
        );
        assert!(
            out.html.contains("src=\"data:image/png;base64,AAAA\""),
            "{}",
            out.html
        );
    }

    #[test]
    fn data_urls_only_for_images() {
        let html =
            r("[x](data:text/html,<script>alert(1)</script>)\n\n<img src=\"data:text/html,hi\">\n")
                .html;
        assert!(!html.contains("data:text"), "{html}");
    }

    #[test]
    fn local_image_path_rules() {
        let base = Path::new(if cfg!(windows) { r"C:\docs" } else { "/docs" });
        assert_eq!(local_image_path("https://x/y.png", Some(base)), None);
        assert_eq!(local_image_path("//cdn/y.png", Some(base)), None);
        assert_eq!(local_image_path("rel.png", None), None);
        assert_eq!(
            local_image_path("./a/../b.png?x=1#f", Some(base)),
            Some(base.join("b.png"))
        );
    }
}
