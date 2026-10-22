#!/usr/bin/env bash
# Re-download the third-party JS libraries and fonts vendored into ui/.
#
# This is NOT part of the build: the files are committed so the app works fully
# offline with no npm/bundler step. Run this only to update a vendored version.
#
#   scripts/vendor.sh
#
set -euo pipefail

MERMAID=12.2.0
KATEX=0.19.0
GRAPHVIZ=1.29.2
FONTSOURCE=5.3.0

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
V="$ROOT/ui/vendor"
F="$ROOT/ui/fonts"
CDN="https://cdn.jsdelivr.net/npm"

get() { # url dest
  mkdir -p "$(dirname "$2")"
  echo "  $2"
  curl -fsSL --retry 5 -o "$2" "$1"
}

echo "mermaid $MERMAID"
get "$CDN/mermaid@$MERMAID/dist/mermaid.min.js" "$V/mermaid/mermaid.min.js"
get "$CDN/mermaid@$MERMAID/LICENSE" "$V/mermaid/LICENSE"

echo "katex $KATEX"
get "$CDN/katex@$KATEX/dist/katex.min.js" "$V/katex/katex.min.js"
get "$CDN/katex@$KATEX/dist/katex.min.css" "$V/katex/katex.min.css"
get "$CDN/katex@$KATEX/LICENSE" "$V/katex/LICENSE"
for f in AMS-Regular Caligraphic-Bold Caligraphic-Regular Fraktur-Bold Fraktur-Regular \
         Main-Bold Main-BoldItalic Main-Italic Main-Regular Math-BoldItalic Math-Italic \
         SansSerif-Bold SansSerif-Italic SansSerif-Regular Script-Regular \
         Size1-Regular Size2-Regular Size3-Regular Size4-Regular Typewriter-Regular; do
  get "$CDN/katex@$KATEX/dist/fonts/KaTeX_$f.woff2" "$V/katex/fonts/KaTeX_$f.woff2"
done

echo "@hpcc-js/wasm-graphviz $GRAPHVIZ"
get "$CDN/@hpcc-js/wasm-graphviz@$GRAPHVIZ/dist/index.js" "$V/graphviz/index.js"
get "$CDN/@hpcc-js/wasm-graphviz@$GRAPHVIZ/LICENSE" "$V/graphviz/LICENSE"

# Fonts (all SIL Open Font License 1.1), latin + latin-ext subsets.
# font <package> <file-prefix> <dir> <subsets> <styles...>
font() {
  local pkg="$1" prefix="$2" dir="$3" subsets="$4"; shift 4
  echo "font $pkg"
  for style in "$@"; do
    for subset in $subsets; do
      get "$CDN/$pkg@$FONTSOURCE/files/$prefix-$subset-$style.woff2" "$F/$dir/$subset-$style.woff2"
    done
  done
  get "$CDN/$pkg@$FONTSOURCE/LICENSE" "$F/$dir/OFL.txt"
}

font @fontsource-variable/inter           inter                  inter                  "latin latin-ext"  wght-normal wght-italic
font @fontsource-variable/source-serif-4  source-serif-4         source-serif-4         "latin latin-ext"  wght-normal wght-italic
font @fontsource-variable/literata        literata               literata               "latin latin-ext"  wght-normal wght-italic
font @fontsource/atkinson-hyperlegible    atkinson-hyperlegible  atkinson-hyperlegible  "latin latin-ext"  400-normal 700-normal 400-italic 700-italic
font @fontsource/ia-writer-quattro        ia-writer-quattro      ia-writer-quattro      latin              400-normal 700-normal 400-italic 700-italic
font @fontsource/opendyslexic             opendyslexic           opendyslexic           latin              400-normal 700-normal 400-italic 700-italic
font @fontsource-variable/jetbrains-mono  jetbrains-mono         jetbrains-mono         "latin latin-ext"  wght-normal wght-italic
font @fontsource-variable/fira-code       fira-code              fira-code              "latin latin-ext"  wght-normal
font @fontsource/ibm-plex-mono            ibm-plex-mono          ibm-plex-mono          "latin latin-ext"  400-normal 700-normal 400-italic 700-italic

echo "done"
