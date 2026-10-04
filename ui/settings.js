import { applyTheme, h, invoke, listen } from "./dom.js";
import { setLang, shortTime, t } from "./i18n.js";

const $ = (id) => document.getElementById(id);
const SVG = "http://www.w3.org/2000/svg";

let state = null;
let section = "general";
let pendingRender = false;
let notice = "";
let confirmRemove = null;
let confirmClear = false;
let historyProfile = null;
let historyDays = 7;

const SECTIONS = [
  ["general", "navGeneral"],
  ["appearance", "navAppearance"],
  ["profiles", "navProfiles"],
  ["history", "navHistory"],
  ["about", "navAbout"],
];

async function save(patch) {
  const settings = { ...state.settings, ...patch };
  state.settings = settings;
  try {
    await invoke("update_settings", { settings });
  } catch (err) {
    notice = String(err);
    await load();
  }
}

// ---- 共通の部品 ----

function row(label, sub, control) {
  return h(
    "div",
    { class: "row" },
    h("div", { class: "text" }, h("div", { class: "label" }, label), sub && h("div", { class: "sub" }, sub)),
    control,
  );
}

function toggle(checked, onChange, label) {
  return h("input", {
    type: "checkbox",
    role: "switch",
    class: "switch",
    checked,
    "aria-label": label,
    onchange: (e) => onChange(e.target.checked),
  });
}

function segmented(options, value, onChange, label) {
  return h(
    "div",
    { class: "segmented", role: "radiogroup", "aria-label": label },
    options.map(([v, text]) =>
      h("button", { type: "button", role: "radio", "aria-checked": String(v === value), onclick: () => onChange(v) }, text),
    ),
  );
}

/** 取り消せない操作のボタン。押すと、その場で「はい / いいえ」を尋ねる。 */
function confirmable({ asking, label, question, yes, setAsking, onYes }) {
  const ask = (v) => {
    setAsking(v);
    render();
  };
  if (!asking) return h("button", { type: "button", class: "button small", onclick: () => ask(true) }, label);
  return h(
    "div",
    { class: "confirm" },
    h("span", {}, question),
    h(
      "button",
      {
        type: "button",
        class: "button small danger",
        onclick: () => {
          setAsking(false);
          onYes();
        },
      },
      yes,
    ),
    h("button", { type: "button", class: "button small", onclick: () => ask(false) }, t("no")),
  );
}

function svg(tag, attrs = {}) {
  const el = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
  return el;
}

// ---- トレイアイコンの見本（src-tauri/src/icon.rs と同じ図形） ----

const TRAY = {
  dark: { bg: "#1f1f1f", levels: ["#3cc75f", "#ff9f0a", "#ff5a4f"], track: "rgba(255,255,255,0.22)", tick: "#ffffff" },
  light: { bg: "#eeeeee", levels: ["#1b7a3a", "#b25000", "#c4001a"], track: "rgba(0,0,0,0.16)", tick: "#1a1a1a" },
};

function arc(r, width, color, frac) {
  const c = 2 * Math.PI * r;
  return svg("circle", {
    cx: 16,
    cy: 16,
    r,
    fill: "none",
    stroke: color,
    "stroke-width": width,
    "stroke-dasharray": `${(c * frac).toFixed(2)} ${c.toFixed(2)}`,
    transform: "rotate(-90 16 16)",
  });
}

/** 見本の値。5 時間 39%（経過 57%）、週間 62%（経過 72%） */
const SAMPLE = { s: 0.39, st: 0.567, sPace: 1, w: 0.62, wt: 0.72, wPace: 2 };
const PACE = {
  dark: ["#3cc75f", "#40c8e0", "#ffd60a", "#ff9f0a", "#ff5a4f", "#c77dff"],
  light: ["#1b7a3a", "#007c89", "#8a6d00", "#b25000", "#c4001a", "#8a3fc2"],
};

function tickLine(x1, y1, x2, y2, stroke, width) {
  return svg("line", { x1, y1, x2, y2, stroke, "stroke-width": width, "stroke-linecap": "round" });
}

function radialTick(t, rIn, rOut, stroke, width) {
  const a = t * 2 * Math.PI - Math.PI / 2;
  return tickLine(16 + rIn * Math.cos(a), 16 + rIn * Math.sin(a), 16 + rOut * Math.cos(a), 16 + rOut * Math.sin(a), stroke, width);
}

function trayPreview(style, dark, settings) {
  const mode = dark ? "dark" : "light";
  const p = TRAY[mode];
  const mono = settings.colorMode === "mono";
  const fg = dark ? "#ffffff" : "#1a1a1a";
  const color = (level) => (mono ? fg : p.levels[level]);
  const marker = settings.trayMarker && style !== "number";
  const tickColor = (pace) =>
    mono ? (dark ? "#8c8c8c" : "#767676") : settings.markerMode === "pace" ? PACE[mode][pace] : p.tick;
  const { s, st, sPace, w, wt, wPace } = SAMPLE;
  const el = svg("svg", { width: 32, height: 32, viewBox: "0 0 32 32", "aria-hidden": "true" });
  if (style === "ring") {
    el.append(arc(12, 5, p.track, 1), arc(12, 5, color(0), s));
    if (marker) el.append(radialTick(st, 7.5, 15.5, tickColor(sPace), 2.4));
  } else if (style === "double") {
    el.append(arc(13.5, 3.5, p.track, 1), arc(13.5, 3.5, color(0), s), arc(7.5, 3.5, p.track, 1), arc(7.5, 3.5, color(1), w));
    if (marker) el.append(radialTick(st, 11, 16, tickColor(sPace), 2), radialTick(wt, 5, 10, tickColor(wPace), 2));
  } else if (style === "number") {
    const text = svg("text", {
      x: 16,
      y: 16.5,
      "text-anchor": "middle",
      "dominant-baseline": "central",
      "font-size": 19,
      "font-weight": 700,
      "font-family": "Segoe UI, sans-serif",
      fill: color(0),
    });
    text.textContent = "39";
    el.append(text);
  } else {
    for (const [x, v, t, level, pace] of [
      [5, s, st, 0, sPace],
      [18, w, wt, 1, wPace],
    ]) {
      el.append(svg("rect", { x, y: 3, width: 9, height: 26, rx: 2, fill: p.track }));
      el.append(svg("rect", { x, y: 3 + 26 * (1 - v), width: 9, height: 26 * v, rx: 2, fill: color(level) }));
      if (marker) {
        const y = 3 + 26 * (1 - t);
        el.append(tickLine(x - 1, y, x + 10, y, tickColor(pace), 2));
      }
    }
  }
  return el;
}

// ---- 各ページ ----

function general() {
  const s = state.settings;
  const minutes = [1, 2, 3, 5, 10, 15, 30];
  return [
    h("h1", {}, t("navGeneral")),
    h(
      "div",
      { class: "group" },
      row(
        t("refreshInterval"),
        t("refreshIntervalDesc"),
        h(
          "select",
          { "aria-label": t("refreshInterval"), onchange: (e) => save({ refreshMinutes: Number(e.target.value) }) },
          minutes.map((m) => h("option", { value: m, selected: m === s.refreshMinutes }, t("everyMinutes", m))),
        ),
      ),
      row(t("autostart"), t("autostartDesc"), toggle(s.autostart, (v) => save({ autostart: v }), t("autostart"))),
      row(t("pinnedOnTop"), null, toggle(s.pinnedOnTop, (v) => save({ pinnedOnTop: v }), t("pinnedOnTop"))),
    ),
  ];
}

function appearance() {
  const s = state.settings;
  const styles = [
    ["ring", "styleRing", "styleRingDesc"],
    ["double", "styleDouble", "styleDoubleDesc"],
    ["number", "styleNumber", "styleNumberDesc"],
    ["bars", "styleBars", "styleBarsDesc"],
  ];
  const choice = (label, options, value, key) =>
    row(label, null, segmented(options, value, (v) => save({ [key]: v }), label));
  return [
    h("h1", {}, t("navAppearance")),
    h(
      "div",
      { class: "group" },
      choice(
        t("theme"),
        [
          ["system", t("themeSystem")],
          ["dark", t("themeDark")],
          ["light", t("themeLight")],
        ],
        s.theme,
        "theme",
      ),
      choice(
        t("popoverLayout"),
        [
          ["cards", t("layoutCards")],
          ["compact", t("layoutCompact")],
        ],
        s.popoverLayout,
        "popoverLayout",
      ),
      choice(
        t("percentMode"),
        [
          [false, t("percentUsed")],
          [true, t("percentRemaining")],
        ],
        s.showRemaining,
        "showRemaining",
      ),
      choice(
        t("markerColor"),
        [
          ["white", t("markerWhite")],
          ["pace", t("markerPace")],
        ],
        s.markerMode,
        "markerMode",
      ),
    ),
    h("h2", {}, t("trayIcon")),
    h(
      "div",
      { class: "styles", role: "radiogroup", "aria-label": t("iconStyle") },
      styles.map(([key, label, desc]) =>
        h(
          "button",
          { type: "button", class: "style-card", role: "radio", "aria-checked": String(s.iconStyle === key), onclick: () => save({ iconStyle: key }) },
          h(
            "div",
            { class: "style-preview" },
            h("span", { class: "swatch", style: `background:${TRAY.dark.bg}` }, trayPreview(key, true, s)),
            h("span", { class: "swatch", style: `background:${TRAY.light.bg}` }, trayPreview(key, false, s)),
          ),
          h("div", { class: "label" }, t(label)),
          h("div", { class: "sub" }, t(desc)),
        ),
      ),
    ),
    h(
      "div",
      { class: "group spaced" },
      row(t("trayMarker"), t("trayMarkerDesc"), toggle(s.trayMarker, (v) => save({ trayMarker: v }), t("trayMarker"))),
      choice(
        t("iconColor"),
        [
          ["color", t("colorColor")],
          ["mono", t("colorMono")],
        ],
        s.colorMode,
        "colorMode",
      ),
    ),
  ];
}

function profileCard(p) {
  const input = h("input", {
    type: "text",
    value: p.name,
    maxlength: 40,
    "aria-label": t("profileName"),
    onchange: (e) => invoke("rename_profile", { id: p.id, name: e.target.value }),
    onkeydown: (e) => {
      if (e.key === "Enter") e.target.blur();
    },
  });
  const removeControl = confirmable({
    asking: confirmRemove === p.id,
    label: t("remove"),
    question: t("removeConfirm"),
    yes: t("yes"),
    setAsking: (v) => (confirmRemove = v ? p.id : null),
    onYes: () => invoke("remove_profile", { id: p.id }),
  });

  return h(
    "div",
    { class: "profile" },
    h("div", { class: "profile-top" }, h("label", { class: "field" }, t("profileName"), input), removeControl),
    h("div", { class: "field" }, t("folder"), h("div", { class: "path mono" }, p.dirDisplay)),
    row(
      t("tokenRefresh"),
      p.allowTokenRefresh == null ? `${t("tokenRefreshDesc")}（${t("tokenRefreshUnasked")}）` : t("tokenRefreshDesc"),
      toggle(p.allowTokenRefresh === true, (v) => invoke("set_token_refresh", { id: p.id, allow: v }), t("tokenRefresh")),
    ),
  );
}

function profiles() {
  return [
    h("h1", {}, t("navProfiles")),
    h("p", { class: "desc" }, t("profilesDesc")),
    h(
      "div",
      { class: "toolbar" },
      h(
        "button",
        {
          type: "button",
          class: "button primary small",
          onclick: async () => {
            const result = await invoke("add_profile_folder");
            notice = result === "noCredentials" ? t("addNoCredentials") : result === "alreadyRegistered" ? t("addExists") : "";
            render();
          },
        },
        t("addFolder"),
      ),
      h(
        "button",
        {
          type: "button",
          class: "button small",
          onclick: async () => {
            const found = await invoke("rescan_profiles");
            notice = found ? "" : t("rescanNone");
            render();
          },
        },
        t("rescan"),
      ),
      notice && h("span", { class: "info", role: "status" }, notice),
    ),
    state.profiles.length === 0
      ? h("p", { class: "desc" }, t("noProfiles"))
      : h("div", { class: "group" }, state.profiles.map(profileCard)),
  ];
}

// ---- 履歴のグラフ ----

const SERIES = [
  ["s", "session", "var(--series-session)"],
  ["w", "weekly", "var(--series-weekly)"],
];

function seriesLabel(key) {
  return key === "s" ? `${t("session")}（5h）` : t("weekly");
}

function chart(points, sinceMs, nowMs, width) {
  // 表示幅に合わせて座標系を取り、文字が縮まないようにする
  const W = Math.max(360, Math.round(width));
  const H = 240;
  const pad = { l: 40, r: 12, t: 10, b: 26 };
  const x = (ms) => pad.l + ((ms - sinceMs) / (nowMs - sinceMs)) * (W - pad.l - pad.r);
  const y = (v) => pad.t + (1 - v / 100) * (H - pad.t - pad.b);
  const root = svg("svg", { class: "chart", viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": t("navHistory") });

  for (const v of [0, 25, 50, 75, 100]) {
    root.append(svg("line", { class: "grid", x1: pad.l, x2: W - pad.r, y1: y(v), y2: y(v) }));
    const label = svg("text", { class: "axis", x: pad.l - 8, y: y(v), "text-anchor": "end", "dominant-baseline": "central" });
    label.textContent = `${v}%`;
    root.append(label);
  }
  for (let i = 0; i <= 4; i++) {
    const ms = sinceMs + ((nowMs - sinceMs) * i) / 4;
    const d = new Date(ms);
    const label = svg("text", {
      class: "axis",
      x: x(ms),
      y: H - 6,
      "text-anchor": i === 0 ? "start" : i === 4 ? "end" : "middle",
    });
    label.textContent = historyDays <= 1 ? `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}` : `${d.getMonth() + 1}/${d.getDate()}`;
    root.append(label);
  }

  // アプリが止まっていた間は線をつながない
  const gap = Math.max(3 * state.settings.refreshMinutes, 30) * 60_000;
  for (const [key, , color] of SERIES) {
    // 途切れた区間ごとに分け、1 点だけの区間は点で描く
    const runs = [];
    for (const p of points) {
      if (p[key] == null) continue;
      const run = runs.at(-1);
      if (run && p.t - run.at(-1).t <= gap) run.push(p);
      else runs.push([p]);
    }
    for (const run of runs) {
      if (run.length === 1) {
        root.append(svg("circle", { cx: x(run[0].t), cy: y(run[0][key]), r: 3, fill: color }));
        continue;
      }
      const d = run.map((p, i) => `${i ? "L" : "M"}${x(p.t).toFixed(1)},${y(p[key]).toFixed(1)}`).join("");
      root.append(svg("path", { class: "line", d, stroke: color }));
    }
  }

  // なぞると、その時刻の値を出す
  const cross = svg("line", { class: "crosshair hidden", y1: pad.t, y2: H - pad.b });
  const dots = SERIES.map(([, , color]) => svg("circle", { class: "dot hidden", r: 4, fill: color }));
  const hit = svg("rect", { x: pad.l, y: 0, width: W - pad.l - pad.r, height: H, fill: "transparent" });
  root.append(cross, ...dots, hit);
  const tip = h("div", { class: "tooltip hidden" });

  const hide = () => {
    cross.classList.add("hidden");
    dots.forEach((dot) => dot.classList.add("hidden"));
    tip.classList.add("hidden");
  };
  hit.addEventListener("mouseleave", hide);
  hit.addEventListener("mousemove", (e) => {
    const box = root.getBoundingClientRect();
    const ms = sinceMs + ((((e.clientX - box.left) / box.width) * W - pad.l) / (W - pad.l - pad.r)) * (nowMs - sinceMs);
    let best = null;
    for (const p of points) if (!best || Math.abs(p.t - ms) < Math.abs(best.t - ms)) best = p;
    if (!best) return hide();
    const px = x(best.t);
    cross.setAttribute("x1", px);
    cross.setAttribute("x2", px);
    cross.classList.remove("hidden");
    SERIES.forEach(([key], i) => {
      if (best[key] == null) return dots[i].classList.add("hidden");
      dots[i].setAttribute("cx", px);
      dots[i].setAttribute("cy", y(best[key]));
      dots[i].classList.remove("hidden");
    });
    tip.replaceChildren(
      h("div", {}, shortTime(best.t)),
      ...SERIES.filter(([key]) => best[key] != null).map(([key, , color]) =>
        h("div", {}, h("span", { class: "swatch", style: `background:${color}` }), `${seriesLabel(key)} ${Math.round(best[key])}%`),
      ),
    );
    tip.classList.remove("hidden");
    const wrap = root.parentElement.getBoundingClientRect();
    const left = e.clientX - wrap.left + 12;
    tip.style.left = `${Math.min(left, wrap.width - 180)}px`;
    tip.style.top = `${e.clientY - wrap.top + 12}px`;
  });
  return [root, tip];
}

/** 表で見る。1 日なら 1 時間ごと、それ以外は 1 日ごとの最大値。 */
function table(points) {
  const buckets = new Map();
  for (const p of points) {
    const d = new Date(p.t);
    const key = historyDays <= 1 ? `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:00` : `${d.getMonth() + 1}/${d.getDate()}`;
    const b = buckets.get(key) ?? { s: null, w: null };
    if (p.s != null) b.s = Math.max(b.s ?? 0, p.s);
    if (p.w != null) b.w = Math.max(b.w ?? 0, p.w);
    buckets.set(key, b);
  }
  const fmt = (v) => (v == null ? "–" : `${Math.round(v)}%`);
  return h(
    "table",
    {},
    h("thead", {}, h("tr", {}, h("th", {}, ""), h("th", {}, seriesLabel("s")), h("th", {}, seriesLabel("w")))),
    h(
      "tbody",
      {},
      [...buckets].map(([k, b]) => h("tr", {}, h("td", {}, k), h("td", {}, fmt(b.s)), h("td", {}, fmt(b.w)))),
    ),
  );
}

function history() {
  const nodes = [h("h1", {}, t("navHistory"))];
  if (state.profiles.length === 0) {
    nodes.push(h("p", { class: "desc" }, t("noProfiles")));
    return nodes;
  }
  if (!state.profiles.some((p) => p.id === historyProfile)) historyProfile = state.profiles[0].id;
  const profile = state.profiles.find((p) => p.id === historyProfile);
  const clearControl = confirmable({
    asking: confirmClear,
    label: t("historyClear"),
    question: t("historyClearConfirm"),
    yes: t("historyClearYes"),
    setAsking: (v) => (confirmClear = v),
    onYes: async () => {
      await invoke("clear_history", { id: historyProfile });
      render();
    },
  });
  const body = h("div", {}, h("div", { class: "empty" }, "…"));
  nodes.push(
    h(
      "div",
      { class: "group" },
      row(
        t("historyRecord"),
        t("historyNote"),
        toggle(state.settings.recordHistory, (v) => save({ recordHistory: v }), t("historyRecord")),
      ),
      row(t("historyClear"), t("historyClearDesc", profile.name), clearControl),
    ),
    h(
      "div",
      { class: "toolbar" },
      state.profiles.length > 1 &&
        h(
          "select",
          {
            "aria-label": t("historyProfile"),
            onchange: (e) => {
              historyProfile = e.target.value;
              confirmClear = false;
              render();
            },
          },
          state.profiles.map((p) => h("option", { value: p.id, selected: p.id === historyProfile }, p.name)),
        ),
      segmented(
        [
          [1, t("range1")],
          [7, t("range7")],
          [30, t("range30")],
          [90, t("range90")],
        ],
        historyDays,
        (v) => {
          historyDays = v;
          render();
        },
        t("range"),
      ),
    ),
    body,
  );

  invoke("get_history", { id: historyProfile, days: historyDays }).then((points) => {
    const nowMs = Date.now();
    const sinceMs = nowMs - historyDays * 86_400_000;
    if (points.length === 0) {
      body.replaceChildren(h("div", { class: "empty" }, t("historyEmpty")));
      return;
    }
    body.replaceChildren(
      h(
        "div",
        { class: "chart-wrap" },
        h(
          "div",
          { class: "legend" },
          SERIES.map(([key, , color]) => h("span", {}, h("i", { style: `background:${color}` }), seriesLabel(key))),
        ),
        ...chart(points, sinceMs, nowMs, body.getBoundingClientRect().width - 34),
      ),
      h("details", {}, h("summary", {}, t("showTable")), table(points)),
    );
  });
  return nodes;
}

function about() {
  const original = h(
    "a",
    { class: "link", href: "#", onclick: (e) => (e.preventDefault(), invoke("open_link", { key: "original" })) },
    "Claude-Usage-Tracker",
  );
  return [
    h("h1", {}, t("navAbout")),
    h(
      "div",
      { class: "about" },
      h("p", {}, t("aboutDesc")),
      h(
        "div",
        { class: "kv" },
        h("span", { class: "key" }, t("destinations")),
        h(
          "span",
          {},
          h("div", {}, h("span", { class: "mono" }, "api.anthropic.com"), ` — ${t("destUsage")}`),
          h("div", {}, h("span", { class: "mono" }, "platform.claude.com"), ` — ${t("destToken")}`),
          h("div", {}, h("span", { class: "mono" }, "status.claude.com"), ` — ${t("destStatus")}`),
        ),
      ),
      h("h2", {}, t("credits")),
      h("p", {}, t("creditBefore"), original, t("creditAfter")),
    ),
  ];
}

const PAGES = { general, appearance, profiles, history, about };

function render() {
  pendingRender = false;
  setLang(state.lang);
  applyTheme(state.settings.theme);
  document.title = t("settingsTitle");
  $("nav").replaceChildren(
    h(
      "div",
      { class: "brand" },
      h("img", { src: "icon.png", alt: "", width: 40, height: 40 }),
      h(
        "div",
        {},
        h(
          "a",
          {
            class: "brand-name",
            href: "#",
            title: t("openRepo"),
            onclick: (e) => (e.preventDefault(), invoke("open_link", { key: "repo" })),
          },
          "Mira",
        ),
        h("div", { class: "brand-version" }, `v${state.version} · ${state.releaseDate}`),
      ),
    ),
    h(
      "div",
      { class: "nav-items" },
      SECTIONS.map(([key, label]) =>
        h(
          "button",
          {
            type: "button",
            "aria-current": key === section ? "page" : null,
            onclick: () => {
              section = key;
              notice = "";
              confirmRemove = null;
              confirmClear = false;
              render();
            },
          },
          t(label),
        ),
      ),
    ),
    h("button", { type: "button", class: "button quit", onclick: () => invoke("quit") }, t("quit")),
  );
  $("content").replaceChildren(...PAGES[section]().filter(Boolean));
}

async function load() {
  state = await invoke("get_state");
  // 入力中に書き換えると入力が消えるので、フォーカスが外れてから描き直す
  const active = document.activeElement;
  if (active && active.tagName === "INPUT" && active.type === "text" && $("content").contains(active)) {
    if (!pendingRender) {
      pendingRender = true;
      active.addEventListener("blur", () => setTimeout(render, 0), { once: true });
    }
    return;
  }
  render();
}

listen("state-changed", () => {
  // 履歴のグラフは取得のたびに作り直さなくてよい
  if (section === "history") {
    invoke("get_state").then((s) => (state = s));
    return;
  }
  load();
});
load();
