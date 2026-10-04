// 画面で共通に使う、DOM の部品と Tauri への橋渡し。

/** テーマ設定を <html data-theme> に反映する。system は Windows の設定に従う。 */
export function applyTheme(theme) {
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  const resolve = () => (theme === "system" ? (media.matches ? "dark" : "light") : theme);
  document.documentElement.dataset.theme = resolve();
  media.onchange = () => {
    document.documentElement.dataset.theme = resolve();
  };
}

/** 子要素と属性を持つ要素を作る。文字列は textContent として入れる。 */
export function h(tag, attrs = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === false || v == null) continue;
    if (k === "class") el.className = v;
    else if (k === "style") el.setAttribute("style", v);
    else if (k.startsWith("on")) el.addEventListener(k.slice(2).toLowerCase(), v);
    else el.setAttribute(k, v === true ? "" : v);
  }
  for (const c of children.flat()) {
    if (c == null || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  return el;
}

/** 線で描くアイコン。path は 24×24 の座標。 */
export function icon(paths, size = 16, width = 1.7) {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("width", size);
  svg.setAttribute("height", size);
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  for (const d of [].concat(paths)) {
    const p = document.createElementNS(ns, "path");
    p.setAttribute("d", d);
    p.setAttribute("stroke-width", width);
    svg.append(p);
  }
  return svg;
}

export const ICONS = {
  chevron: "M6 9l6 6 6-6",
  pin: "M9 3h6l-1 6 3 3H7l3-3-1-6z M12 12v9",
  refresh: "M20 12a8 8 0 1 1-2.34-5.66 M20 4v5h-5",
  gear: [
    "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
    "M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z",
  ],
  warning: "M12 3l10 18H2z M12 10v5 M12 18h.01",
  check: "M5 12l5 5L20 7",
};

// WebView2 の右クリックメニュー（再読み込み、印刷など）を出さない。文字の入力欄では編集用のメニューを残す
document.addEventListener("contextmenu", (e) => {
  if (!e.target.closest?.('input[type="text"], textarea')) e.preventDefault();
});

export const tauri = window.__TAURI__;
export const invoke = (cmd, args) => tauri.core.invoke(cmd, args);
export const listen = (event, fn) => tauri.event.listen(event, fn);
