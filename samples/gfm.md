---
title: GFM element showcase
author: Lectern
tags: [markdown, gfm, test]
---

# GFM element showcase

The front matter above renders as a collapsed **Front matter** box instead of a stray
horizontal rule. Back to the [index](README.md).

## Inline formatting

Plain, *emphasis*, _also emphasis_, **strong**, __also strong__, ***both***,
~~strikethrough~~, `inline code`, <kbd>Ctrl</kbd>+<kbd>C</kbd>, <mark>marked</mark>,
H<sub>2</sub>O, x<sup>2</sup>, and a hard line break after this sentence  
(two trailing spaces) followed by a backslash break\
done.

Autolinks: https://github.com and www.example.com and <mailto:someone@example.com>.
Inline [link with title](https://example.org "Example title"), and a
[reference link][ref] plus a [collapsed reference][].

[ref]: https://commonmark.org/help/
[collapsed reference]: https://spec.commonmark.org/

Escapes: \*not emphasis\*, \`not code\`, &copy; &amp; &lt;tag&gt; &#x1F600;

## Headings

### Level three
#### Level four
##### Level five (not in the TOC)
###### Level six (not in the TOC)

Setext heading level 1
======================

Setext heading level 2
----------------------

## Lists

- Unordered item
- Another item
  - Nested item
    - Deeper item
- Item with a paragraph

  that continues here.

1. First
2. Second
   1. Nested ordered
   2. Nested ordered two
3. Third

7. A list starting at seven
8. Eight

### Task lists

- [x] Write the renderer
- [x] Sanitize the output
- [ ] Take over the world
  - [x] Nested task done
  - [ ] Nested task open

## Blockquotes

> A blockquote with **formatting** and a [link](https://example.com).
>
> > A nested quote.
>
> - A list inside a quote

## Alerts

> [!NOTE]
> Useful information that users should know.

> [!TIP]
> Helpful advice for doing things better.

> [!IMPORTANT]
> Key information users need to know.

> [!WARNING]
> Urgent info that needs immediate attention.

> [!CAUTION]
> Advises about risks or negative outcomes.

## Tables

| Left aligned | Centered | Right aligned | Default |
|:-------------|:--------:|--------------:|---------|
| apples       | 1        | 1.00          | `code`  |
| bananas      | 22       | 22.50         | **bold** |
| cherries     | 333      | 333.75        | [link](#tables) |
| a much longer cell that wraps maybe | ✓ | 0.01 | ~~old~~ |

A wide table scrolls horizontally rather than breaking the layout:

| a | b | c | d | e | f | g | h | i | j | k | l | m | n |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| alpha | bravo | charlie | delta | echo | foxtrot | golf | hotel | india | juliett | kilo | lima | mike | november |

## Code

Indented code block:

    $ lectern README.md
    opens the file

```rust
use std::collections::HashMap;

/// Count words in a string.
fn count_words(text: &str) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts // returned
}

fn main() {
    let n: u32 = 42;
    println!("{:?} {}", count_words("a b a"), n);
}
```

```python
from dataclasses import dataclass

@dataclass
class Point:
    x: float
    y: float = 0.0

    def norm(self) -> float:
        """Euclidean norm."""
        return (self.x ** 2 + self.y ** 2) ** 0.5

print(Point(3, 4).norm())  # 5.0
```

```js
// JavaScript
export async function load(url, { retries = 3 } = {}) {
  for (let i = 0; i < retries; i++) {
    const res = await fetch(url);
    if (res.ok) return res.json();
  }
  throw new Error(`failed: ${url}`);
}
```

```bash
#!/usr/bin/env bash
set -euo pipefail
for f in *.md; do
  echo "rendering $f" && lectern "$f" &
done
```

```json
{ "name": "lectern", "version": "0.1.0", "tags": ["viewer", "markdown"], "fast": true, "deps": null }
```

```yaml
theme: nord
fonts:
  body: literata   # serif
  code: fira-code
```

```diff
- removed line
+ added line
  unchanged line
@@ -1,3 +1,3 @@
```

```html
<!doctype html>
<p class="greeting" id="hi">Hello <b>world</b></p>
```

```css
:root[data-theme="nord"] { --bg: #2e3440; }
.markdown-body > p:first-child { margin: 0 !important; }
```

```go
package main

import "fmt"

func main() {
	fmt.Println("hello, world")
}
```

```c
#include <stdio.h>
int main(void) { printf("%d\n", 42); return 0; }
```

```sql
SELECT id, title FROM docs WHERE title LIKE '%md%' ORDER BY id DESC LIMIT 10;
```

```
A fenced block without a language: no highlighting, still copyable.
```

```unknownlang
A language syntect doesn't know: shown as plain text.
```

## Horizontal rules

---
***
___

## Footnotes

Here is a footnote reference[^1], another one[^note], and a repeat[^1].

[^1]: The first footnote. Click ↩ to jump back.
[^note]: A named footnote with **formatting** and `code`.

## Raw HTML (sanitized)

<details>
<summary>Click to expand</summary>

Hidden content with *Markdown* inside, and a table:

| x | y |
|---|---|
| 1 | 2 |

</details>

<div align="center">
  <p>Centered paragraph via raw HTML <code>align</code>.</p>
</div>

<dl>
  <dt>Definition term</dt>
  <dd>Definition description.</dd>
</dl>

Line one<br>line two via `<br>`.
