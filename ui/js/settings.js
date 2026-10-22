// Appearance settings: theme, fonts, size, width, TOC visibility.
// Persisted by Rust (config.toml) via the `save_settings` command.

import { invoke } from './tauri.js';
import { $, $$, el, debounce, toast } from './util.js';

export const BODY_FONTS = [
  { id: 'inter', name: 'Inter', stack: '"Inter", system-ui, sans-serif' },
  { id: 'source-serif-4', name: 'Source Serif 4', stack: '"Source Serif 4", Georgia, serif' },
  { id: 'literata', name: 'Literata', stack: '"Literata", Georgia, serif' },
  { id: 'atkinson-hyperlegible', name: 'Atkinson Hyperlegible', stack: '"Atkinson Hyperlegible", system-ui, sans-serif' },
  { id: 'ia-writer-quattro', name: 'iA Writer Quattro', stack: '"iA Writer Quattro", system-ui, sans-serif' },
  { id: 'opendyslexic', name: 'OpenDyslexic', stack: '"OpenDyslexic", system-ui, sans-serif' },
];

export const CODE_FONTS = [
  { id: 'jetbrains-mono', name: 'JetBrains Mono', stack: '"JetBrains Mono", ui-monospace, monospace' },
  { id: 'fira-code', name: 'Fira Code', stack: '"Fira Code", ui-monospace, monospace' },
  { id: 'ibm-plex-mono', name: 'IBM Plex Mono', stack: '"IBM Plex Mono", ui-monospace, monospace' },
];

const WIDTHS = { narrow: '680px', medium: '820px', wide: '1080px', full: 'none' };
export const MIN_SIZE = 10;
export const MAX_SIZE = 32;
export const DEFAULT_SIZE = 16;

/** Mirrors `UiSettings` in config.rs (snake_case on purpose). */
export const settings = {
  theme: 'system',
  body_font: 'inter',
  code_font: 'jetbrains-mono',
  font_size: DEFAULT_SIZE,
  content_width: 'medium',
  toc_open: true,
};

const systemDark = window.matchMedia('(prefers-color-scheme: dark)');
let themes = [];

/** Find every `:root[data-theme="…"]` block in the loaded stylesheets. */
export function discoverThemes() {
  const found = [];
  for (const sheet of document.styleSheets) {
    let rules;
    try {
      rules = sheet.cssRules;
    } catch {
      continue;
    }
    for (const rule of rules) {
      const m = rule.selectorText?.match(/^:root\[data-theme=["']?([\w-]+)["']?\]$/);
      if (!m || found.some((t) => t.id === m[1])) continue;
      const label = rule.style.getPropertyValue('--theme-name').trim().replace(/^["']|["']$/g, '');
      found.push({ id: m[1], name: label || m[1] });
    }
  }
  return found;
}

export function resolvedTheme() {
  if (settings.theme !== 'system' && themes.some((t) => t.id === settings.theme)) return settings.theme;
  return systemDark.matches ? 'dark' : 'light';
}

const persist = debounce(() => {
  invoke('save_settings', { settings: { ...settings } }).catch((e) => console.warn('save_settings', e));
}, 300);

let lastAppearance = '';

/** Apply `settings` to the document. Fires `appearance-change` when diagrams must re-render. */
export function apply() {
  const root = document.documentElement;
  if (!BODY_FONTS.some((f) => f.id === settings.body_font)) settings.body_font = BODY_FONTS[0].id;
  if (!CODE_FONTS.some((f) => f.id === settings.code_font)) settings.code_font = CODE_FONTS[0].id;
  if (!(settings.content_width in WIDTHS)) settings.content_width = 'medium';
  settings.font_size = Math.min(MAX_SIZE, Math.max(MIN_SIZE, Math.round(settings.font_size) || DEFAULT_SIZE));

  root.dataset.theme = resolvedTheme();
  root.style.setProperty('--body-font', BODY_FONTS.find((f) => f.id === settings.body_font).stack);
  root.style.setProperty('--code-font', CODE_FONTS.find((f) => f.id === settings.code_font).stack);
  root.style.setProperty('--font-size', `${settings.font_size}px`);
  root.style.setProperty('--content-width', WIDTHS[settings.content_width]);
  document.body.classList.toggle('toc-open', settings.toc_open);

  syncControls();

  // Mermaid bakes colours and font metrics into its SVG, so it needs to know.
  const appearance = `${root.dataset.theme}|${settings.body_font}`;
  if (appearance !== lastAppearance) {
    const first = lastAppearance === '';
    lastAppearance = appearance;
    if (!first) document.dispatchEvent(new CustomEvent('appearance-change'));
  }
}

/** Change settings, apply and persist them. */
export function update(patch) {
  Object.assign(settings, patch);
  apply();
  persist();
}

export function cycleTheme() {
  const ids = themes.map((t) => t.id);
  const next = ids[(ids.indexOf(resolvedTheme()) + 1) % ids.length];
  update({ theme: next });
  toast(`Theme: ${themes.find((t) => t.id === next).name}`, { ms: 1200 });
}

export function changeFontSize(delta) {
  update({ font_size: delta === 0 ? DEFAULT_SIZE : settings.font_size + delta });
}

export function toggleToc(force) {
  update({ toc_open: force ?? !settings.toc_open });
}

function syncControls() {
  $('#lec-set-theme').value = themes.some((t) => t.id === settings.theme) ? settings.theme : 'system';
  $('#lec-set-body-font').value = settings.body_font;
  $('#lec-set-code-font').value = settings.code_font;
  $('#lec-size-value').textContent = `${settings.font_size}px`;
  for (const b of $$('#lec-set-width button')) {
    b.setAttribute('aria-pressed', String(b.dataset.width === settings.content_width));
  }
  $('#lec-set-toc').checked = settings.toc_open;
  $('#lec-toc-btn').setAttribute('aria-pressed', String(settings.toc_open));
}

/** Build the settings popover controls. Call once before `apply()`. */
export function initSettingsUI() {
  themes = discoverThemes();

  const themeSel = $('#lec-set-theme');
  themeSel.append(el('option', { value: 'system', text: 'Follow system' }));
  for (const t of themes) themeSel.append(el('option', { value: t.id, text: t.name }));
  themeSel.addEventListener('change', () => update({ theme: themeSel.value }));

  const fontSelect = (sel, fonts, key) => {
    for (const f of fonts) {
      const o = el('option', { value: f.id, text: f.name });
      o.style.fontFamily = f.stack;
      sel.append(o);
    }
    sel.addEventListener('change', () => update({ [key]: sel.value }));
  };
  fontSelect($('#lec-set-body-font'), BODY_FONTS, 'body_font');
  fontSelect($('#lec-set-code-font'), CODE_FONTS, 'code_font');

  $('#lec-size-down').addEventListener('click', () => changeFontSize(-1));
  $('#lec-size-up').addEventListener('click', () => changeFontSize(1));
  $('#lec-size-reset').addEventListener('click', () => changeFontSize(0));
  for (const b of $$('#lec-set-width button')) {
    b.addEventListener('click', () => update({ content_width: b.dataset.width }));
  }
  $('#lec-set-toc').addEventListener('change', (e) => toggleToc(e.target.checked));

  systemDark.addEventListener('change', () => {
    if (settings.theme === 'system') apply();
  });
}
