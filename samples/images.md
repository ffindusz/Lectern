# Images

Local images resolve against this file's directory and are served through the app's
`mdfile:` protocol, which only serves the images this document references. Click any
image to see it full size; click again to toggle fit / 100 %, and press Esc to close.

## Local formats

PNG: ![A PNG test card](images/diagram.png)

JPEG: ![A JPEG test card](images/photo.jpg "JPEG with a title")

GIF (animated): ![Animated GIF](images/bounce.gif)

WebP: ![WebP test card](images/leaf.webp)

SVG (contains a `<script>` that must not run): ![SVG logo](images/logo.svg)

## Paths

- `./` prefix: ![dot-slash](./images/logo.svg)
- Parent directory, from the nested sample: see [nested/other.md](nested/other.md)
- Percent-encoded and with a query string: ![encoded](images%2Fdiagram.png?v=1)

## Large image

Scaled down to fit; click to view at full size.

![Large image](images/large.png)

## Raw HTML image with a size

<img src="images/diagram.png" width="240" alt="Raw HTML img, width=240">

## Image inside a link

[![Clickable SVG (opens the GFM sample)](images/logo.svg)](gfm.md)

## Remote images

These need a network connection; they are allowed by the CSP (`img-src https:`).

![Markdown mark from Wikimedia](https://upload.wikimedia.org/wikipedia/commons/4/48/Markdown-mark.svg)

![Random photo from picsum.photos](https://picsum.photos/seed/lectern/640/220)

## Missing image

![This file does not exist](images/nope.png)
