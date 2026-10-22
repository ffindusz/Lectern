# Headings and anchors

Slugs follow GitHub's rules: lowercase, punctuation dropped, spaces become `-`, and
duplicates get `-1`, `-2`, … suffixes. Hover a TOC entry to see its title; click to jump.

Jump links: [first Setup](#setup), [second Setup](#setup-1), [third Setup](#setup-2),
[literal "Setup 1"](#setup-1-1), [C++ & Rust](#c--rust), [Über Straße](#über-straße),
[`code` heading](#code-in-a-heading), [emoji](#emoji--heading), and a
[missing anchor](#does-not-exist).

## Setup

First "Setup" → `#setup`.

## Setup

Second "Setup" → `#setup-1`.

## Setup 1

A heading literally called "Setup 1" → `#setup-1-1` (no collision).

## Setup

Third "Setup" → `#setup-2`.

## C++ & Rust

Punctuation is removed → `#c--rust`.

## Über Straße

Unicode letters are kept → `#über-straße`.

## `code` in a heading

Inline code is flattened to text → `#code-in-a-heading`.

## Emoji 🎉 heading

Emoji are dropped → `#emoji--heading`.

## A very long heading that keeps going and going so that it has to be truncated with an ellipsis in the sidebar

The TOC truncates long entries; the full text is in the tooltip.

### Nested under the long one

#### Fourth level is included

##### Fifth level is not in the TOC

But it still gets an anchor: [jump to it](#fifth-level-is-not-in-the-toc).

## Same name in different sections

### Example

`#example`

## Another section

### Example

`#example-1`
