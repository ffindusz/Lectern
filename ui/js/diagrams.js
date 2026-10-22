// Mermaid and Graphviz rendering. Libraries are vendored and loaded on first use.
// A failing diagram always becomes an inline error box showing the message and source.

import { $$, el, loadScript } from './util.js';

const svgCache = new Map(); // `${appearanceKey}\0${source}` → mermaid SVG string
const MAX_CACHE = 200;
let seq = 0;
let mermaidQueue = Promise.resolve();
let graphvizPromise = null;

/** Replace `pre.diagram-source` blocks (emitted by Rust) with rendered diagrams. */
export function renderDiagrams(root) {
  const boxes = $$('pre.diagram-source', root).map((pre) => {
    const kind = pre.dataset.diagram === 'mermaid' ? 'mermaid' : 'dot';
    const box = el('div', { class: `diagram diagram-${kind} loading`, 'data-kind': kind, text: 'Rendering diagram…' });
    box.dataset.source = pre.textContent;
    pre.replaceWith(box);
    return box;
  });
  return renderBoxes(boxes);
}

/** Re-render Mermaid diagrams after a theme or font change (Graphviz follows CSS). */
export function rerenderMermaid(root) {
  return renderBoxes($$('[data-kind="mermaid"]', root));
}

function renderBoxes(boxes) {
  const mermaid = boxes.filter((b) => b.dataset.kind === 'mermaid');
  const dot = boxes.filter((b) => b.dataset.kind === 'dot');
  return Promise.allSettled([
    mermaid.length ? renderMermaid(mermaid) : null,
    dot.length ? renderDot(dot) : null,
  ]);
}

function showSvg(box, svgMarkup) {
  box.className = `diagram diagram-${box.dataset.kind}`;
  box.setAttribute('role', 'img');
  box.innerHTML = svgMarkup;
}

function showError(box, title, err) {
  let message = err?.message ?? err?.str ?? String(err ?? 'Unknown error');
  message = message.replace(/^Error:\s*/, '').trim() || 'Unknown error';
  box.className = 'diagram-error';
  box.removeAttribute('role');
  box.replaceChildren(
    el('div', { class: 'diagram-error-title', text: `⚠ ${title} could not be rendered` }),
    el('div', { class: 'diagram-error-message', text: message }),
    el('pre', {}, el('code', { text: box.dataset.source })),
  );
}

// ------------------------------------------------------------------ Mermaid

function cssVar(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function mermaidConfig() {
  const v = cssVar;
  const fg = v('--fg');
  const bg = v('--bg');
  const fill = v('--diagram-fill');
  const fill2 = v('--diagram-fill-2');
  const fill3 = v('--diagram-fill-3');
  const stroke = v('--diagram-stroke');
  const line = v('--diagram-line');
  const note = v('--diagram-note');
  const border = v('--border');
  const font = v('--body-font');
  const dark = getComputedStyle(document.documentElement).colorScheme.includes('dark');
  return {
    startOnLoad: false,
    securityLevel: 'strict',
    suppressErrorRendering: true,
    theme: 'base',
    fontFamily: font,
    themeVariables: {
      darkMode: dark,
      background: bg,
      fontFamily: font,
      fontSize: '15px',
      textColor: fg,
      lineColor: line,
      primaryColor: fill,
      primaryTextColor: fg,
      primaryBorderColor: stroke,
      secondaryColor: fill2,
      secondaryTextColor: fg,
      secondaryBorderColor: stroke,
      tertiaryColor: fill3,
      tertiaryTextColor: fg,
      tertiaryBorderColor: stroke,
      mainBkg: fill,
      nodeBorder: stroke,
      nodeTextColor: fg,
      clusterBkg: fill3,
      clusterBorder: border,
      titleColor: fg,
      edgeLabelBackground: bg,
      noteBkgColor: note,
      noteTextColor: fg,
      noteBorderColor: stroke,
      actorBkg: fill,
      actorBorder: stroke,
      actorTextColor: fg,
      actorLineColor: line,
      signalColor: fg,
      signalTextColor: fg,
      labelBoxBkgColor: fill,
      labelBoxBorderColor: stroke,
      labelTextColor: fg,
      loopTextColor: fg,
      activationBkgColor: fill2,
      activationBorderColor: stroke,
      sequenceNumberColor: bg,
      sectionBkgColor: fill3,
      altSectionBkgColor: bg,
      sectionBkgColor2: fill2,
      taskBkgColor: fill,
      taskBorderColor: stroke,
      taskTextColor: fg,
      taskTextLightColor: fg,
      taskTextDarkColor: fg,
      taskTextOutsideColor: fg,
      taskTextClickableColor: v('--link'),
      activeTaskBkgColor: fill2,
      activeTaskBorderColor: stroke,
      doneTaskBkgColor: border,
      doneTaskBorderColor: line,
      critBkgColor: fill2,
      critBorderColor: v('--syn-deleted'),
      gridColor: border,
      todayLineColor: v('--syn-deleted'),
      pie1: fill, pie2: fill2, pie3: fill3, pie4: note,
      pieStrokeColor: stroke,
      pieTitleTextColor: fg,
      pieSectionTextColor: fg,
      pieLegendTextColor: fg,
    },
  };
}

function renderMermaid(boxes) {
  // mermaid.render is not re-entrant; serialize all work.
  mermaidQueue = mermaidQueue.then(() => renderMermaidNow(boxes)).catch((e) => console.error(e));
  return mermaidQueue;
}

async function renderMermaidNow(boxes) {
  try {
    await loadScript('vendor/mermaid/mermaid.min.js');
  } catch (err) {
    boxes.forEach((b) => showError(b, 'Mermaid diagram', err));
    return;
  }
  const mermaid = window.mermaid;
  const config = mermaidConfig();
  const key = JSON.stringify(config.themeVariables);
  mermaid.initialize(config);

  for (const box of boxes) {
    if (!box.isConnected) continue;
    const source = box.dataset.source;
    const cacheKey = `${key}\0${source}`;
    const cached = svgCache.get(cacheKey);
    if (cached) {
      showSvg(box, cached);
      continue;
    }
    const id = `lec-mermaid-${++seq}`;
    try {
      await mermaid.parse(source);
      const { svg } = await mermaid.render(id, source);
      if (svgCache.size >= MAX_CACHE) svgCache.delete(svgCache.keys().next().value);
      svgCache.set(cacheKey, svg);
      showSvg(box, svg);
    } catch (err) {
      showError(box, 'Mermaid diagram', err);
    } finally {
      // Mermaid renders into temporary nodes in <body>; make sure none are left behind.
      document.getElementById(`d${id}`)?.remove();
      for (const n of document.querySelectorAll(`body > #${id}, body > [id^="d${id}"]`)) n.remove();
    }
  }
}

// ------------------------------------------------------------------ Graphviz

function graphviz() {
  graphvizPromise ??= import('../vendor/graphviz/index.js')
    .then((m) => m.Graphviz.load())
    .catch((e) => {
      graphvizPromise = null;
      throw e;
    });
  return graphvizPromise;
}

const SAFE_HREF = /^(https?:|mailto:|#)/i;

/** Graphviz output is not sanitized by the library; strip anything active. */
function sanitizeSvg(markup) {
  const doc = new DOMParser().parseFromString(markup, 'image/svg+xml');
  const svg = doc.documentElement;
  if (svg.nodeName !== 'svg' || doc.querySelector('parsererror')) {
    throw new Error('Graphviz produced invalid SVG');
  }
  for (const n of svg.querySelectorAll('script, foreignObject, iframe, object, embed, use, animate, set')) n.remove();
  for (const n of svg.querySelectorAll('*')) {
    for (const attr of Array.from(n.attributes)) {
      const name = attr.name.toLowerCase();
      if (name.startsWith('on')) n.removeAttribute(attr.name);
      else if ((name === 'href' || name === 'xlink:href') && !SAFE_HREF.test(attr.value.trim())) {
        n.removeAttribute(attr.name);
      }
    }
  }
  return document.importNode(svg, true);
}

async function renderDot(boxes) {
  let gv;
  try {
    gv = await graphviz();
  } catch (err) {
    boxes.forEach((b) => showError(b, 'Graphviz diagram', err));
    return;
  }
  for (const box of boxes) {
    if (!box.isConnected) continue;
    try {
      const svg = gv.dot(box.dataset.source);
      if (!svg?.includes('<svg')) throw new Error('Graphviz produced no output (syntax error?)');
      box.className = 'diagram diagram-dot';
      box.setAttribute('role', 'img');
      box.replaceChildren(sanitizeSvg(svg));
    } catch (err) {
      showError(box, 'Graphviz diagram', err);
    }
  }
}
