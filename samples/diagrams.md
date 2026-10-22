# Diagrams

Mermaid and Graphviz render from fenced code blocks. Switch themes with
<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd>: diagram colours follow. Broken diagrams live in
[broken.md](broken.md).

## Mermaid flowchart

```mermaid
flowchart LR
    A[Markdown file] -->|read| B(Rust: comrak)
    B --> C{Sanitize}
    C -->|ammonia| D[Safe HTML]
    D --> E[WebView]
    E --> F((Mermaid))
    E --> G((KaTeX))
    E --> H((Graphviz))
    subgraph Frontend
      E
      F
      G
      H
    end
```

## Mermaid sequence diagram

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant UI as WebView (JS)
    participant Core as Rust core
    participant FS as File system
    User->>UI: Ctrl+O
    UI->>Core: invoke("open_dialog")
    Core->>FS: read file
    FS-->>Core: bytes
    Core->>Core: render + sanitize
    Core-->>UI: Doc { html, toc }
    Note over UI,Core: The watcher emits "file-changed" on save
    loop Every save
        FS-->>Core: notify event
        Core-->>UI: file-changed
        UI->>Core: reload()
    end
```

## Mermaid gantt chart

```mermaid
gantt
    title Lectern milestones
    dateFormat  YYYY-MM-DD
    axisFormat  %b %d
    section Core
    Open + render GFM      :done,    m1, 2024-01-01, 3d
    TOC + sidebar          :done,    m2, after m1, 2d
    section Look
    Themes + fonts         :active,  m3, after m2, 3d
    section Extras
    Mermaid, KaTeX, images :         m4, after m3, 4d
    Graphviz + find        :crit,    m5, after m4, 2d
```

## Mermaid class diagram

```mermaid
classDiagram
    class Config {
        +UiSettings ui
        +WindowGeometry window
        +Vec~PathBuf~ recent_files
        +load(path) Config
        +save(path)
    }
    class UiSettings {
        +String theme
        +u32 font_size
    }
    Config *-- UiSettings
```

## Mermaid state diagram

```mermaid
stateDiagram-v2
    [*] --> Empty
    Empty --> Showing: open file
    Showing --> Showing: file changed / reload
    Showing --> Error: file deleted
    Error --> Showing: file restored
    Showing --> [*]: close
```

## Mermaid pie chart

```mermaid
pie title Time spent
    "Rendering" : 40
    "Theming" : 25
    "Diagrams" : 20
    "Bikeshedding" : 15
```

## Graphviz (dot)

```dot
digraph pipeline {
    rankdir=LR;
    node [shape=box, style=rounded];
    md [label="file.md"];
    comrak; ammonia; webview [label="WebView"];
    md -> comrak -> ammonia -> webview;
    webview -> mermaid [style=dashed];
    webview -> katex [style=dashed];
}
```

Graphviz with explicit colours (these are kept as written):

```graphviz
graph G {
    layout=neato;
    node [style=filled];
    a [fillcolor="#ffcc66"]; b [fillcolor="#99ccff"]; c [fillcolor="#cc99ff"];
    a -- b -- c -- a;
}
```
