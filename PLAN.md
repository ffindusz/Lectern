# Lectern — implementation plan

## Layout

```
lectern/
├── README.md               build deps, usage, shortcuts, adding a theme
├── PLAN.md                 this file
├── samples/                Markdown exercising every feature
├── scripts/vendor.sh       re-downloads vendored JS/fonts (documentation; not part of the build)
├── ui/                     frontendDist: plain HTML/CSS/JS, no bundler
│   ├── index.html
│   ├── app.js              ES module: wiring, shortcuts, toolbar, settings
│   ├── js/                 small modules: toc.js, diagrams.js, math.js, find.js, …
│   ├── css/                base.css, themes.css, syntax.css, fonts.css
│   ├── fonts/              vendored woff2 + OFL licences
│   └── vendor/             mermaid, katex, @hpcc-js/wasm-graphviz (+ licences)
└── src-tauri/
    ├── Cargo.toml, build.rs, tauri.conf.json, capabilities/, icons/
    └── src/
        ├── main.rs         thin: calls lectern_lib::run()
        ├── lib.rs          Tauri builder, state, commands, events, window geometry
        ├── render.rs       Markdown → sanitized HTML + TOC (NO tauri dependency)
        ├── slug.rs         GitHub-style slugger (dedup: foo, foo-1, …)
        ├── highlight.rs    syntect → class-based spans (comrak adapter)
        ├── config.rs       Config struct, load (corrupt → defaults), atomic save
        └── watcher.rs      notify watcher on parent dir, debounced
    (lib.rs also hosts the `mdfile:` image protocol, which serves only images the current doc references)
    windows/                app manifest (Common Controls v6), embedded into every target incl. tests
```

## Tauri commands (Rust ⇄ JS)

| command            | purpose |
|--------------------|---------|
| `startup`          | → `{ settings, doc? }` (doc from CLI arg) |
| `open_path(path)`  | read + render + watch + add to recent → `Doc` |
| `open_dialog`      | native file dialog → `Doc?` |
| `open_link(href)`  | resolve relative `.md` link against current doc → `Doc` |
| `reload`           | re-render current file → `Doc` |
| `save_settings(s)` | persist UI settings (theme, fonts, size, width, toc) |
| `clear_recent`     | |
| `open_external(url)` | http/https/mailto only, via opener plugin |
| `recent_files`     | recent list (existing files only) |
| `print`, `close_window`, `ready` | webview print, close, show window after first paint |

`Doc = { path, fileName, html, toc: [{level, text, id}], title }`.
Events Rust → JS: `file-changed` (debounced watcher), `open-file` and `drag-state` (drag & drop).

## Rendering pipeline (render.rs)

comrak parse (GFM + footnotes + front matter + `$math$`) → walk AST: collect headings,
assign slugs, inject anchors, pull front matter → format HTML with a syntect adapter
(class-based spans; `mermaid`/`dot`/`math` blocks left as plain escaped code for JS) →
ammonia with an explicit allowlist (local `img src` rewritten to `mdfile://`) → `Rendered { html, toc, local_images }`.

## Security
- CSP: `script-src 'self' 'wasm-unsafe-eval'`, no inline scripts, `object-src 'none'`.
- Document HTML is always ammonia-sanitized; `<script>`, event handlers, and `javascript:` URLs are stripped.
- Images are served by a custom protocol that only returns files the current document references (allowlist), with image MIME types only.

## Milestones
1. **M1**: skeleton, open (CLI/dialog/DnD), GFM render, path header, title.
2. **M2**: TOC in Rust, sidebar, Ctrl+\, scroll-spy, auto-hide (<2 headings).
3. **M3**: themes (CSS vars), syntax colours via vars, fonts, size/width, config.toml.
4. **M4**: Mermaid, KaTeX, images (protocol + lightbox), live reload, md links, recent files.
5. **M5**: Graphviz, find (Ctrl+F), copy buttons, print.

Each milestone: `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, run the app.

## Decisions
- **Menu**: an in-app toolbar with a "File" dropdown instead of a native GTK menubar. It follows
  the active theme (a native menubar stays light in dark themes), and all shortcuts are handled
  in one place in JS.
- **Lazy-loading**: mermaid, KaTeX, and graphviz load only when a document needs them, to keep startup fast.
- **Window geometry**: tracked in Rust and written to config.toml on close.
