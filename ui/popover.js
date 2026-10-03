import { ICONS, applyTheme, h, icon, invoke, listen, money, resetShortText, resetText, setLang, shortTime, t } from "./i18n.js";

const $ = (id) => document.getElementById(id);

let state = null;
let profileId = null;
let pinned = false;

const STATUS = {
  none: ["statusNone", "var(--green)"],
  minor: ["statusMinor", "var(--yellow)"],
  major: ["statusMajor", "var(--orange)"],
  critical: ["statusCritical", "var(--red)"],
  unknown: ["statusUnknown", "var(--muted)"],
};

function currentProfile() {
  if (!state) return null;
  return state.profiles.find((p) => p.id === profileId) ?? state.profiles[0] ?? null;
}

function renderHeader(profile) {
  // プロファイルが 1 つなら選ぶものがないので、矢印を出さずただの見出しにする
  const button = $("profile-button");
  const choosable = state.profiles.length > 1;
  button.replaceChildren(h("span", {}, profile?.name ?? "Mira"), ...(choosable ? [icon(ICONS.chevron, 13, 2)] : []));
  button.disabled = !choosable;
  if (choosable) button.setAttribute("aria-label", t("pickProfile"));
  else button.removeAttribute("aria-label");

  const [key, color] = state.status ? STATUS[state.status] : ["statusLoading", "var(--muted)"];
  $("status").title = t(key);
  $("status").setAttribute("aria-label", t(key));
  $("status-dot").style.background = color;

  const set = (id, paths, label, width = 1.7) => {
    const el = $(id);
    el.replaceChildren(icon(paths, 15, width));
    el.setAttribute("aria-label", label);
    el.title = label;
  };
  set("pin", ICONS.pin, pinned ? t("unpin") : t("pin"));
  $("pin").setAttribute("aria-pressed", String(pinned));
  // 最終更新の時刻は、更新ボタンのツールチップに出す
  const updated = profile?.lastSuccessMs ? t("lastUpdated", shortTime(profile.lastSuccessMs)) : t("neverUpdated");
  set("refresh", ICONS.refresh, t("refreshTip", updated));
  set("settings", ICONS.gear, t("settings"), 1.5);

  // 固定中はボタン以外のどこをつかんでも動かせる。"deep" は子要素も含め、ボタンなどは除く
  if (pinned) $("popover").setAttribute("data-tauri-drag-region", "deep");
  else $("popover").removeAttribute("data-tauri-drag-region");
}

function renderMenu() {
  const menu = $("profile-menu");
  menu.replaceChildren(
    ...state.profiles.map((p) =>
      h(
        "button",
        {
          type: "button",
          role: "menuitemradio",
          "aria-checked": String(p.id === currentProfile()?.id),
          onclick: () => selectProfile(p.id),
        },
        h("span", { class: "check" }, p.id === currentProfile()?.id ? icon(ICONS.check, 14, 2.2) : null),
        h("span", { class: "name" }, p.name),
      ),
    ),
  );
}

function toggleMenu(open) {
  const menu = $("profile-menu");
  const show = open ?? menu.classList.contains("hidden");
  if (show) renderMenu();
  menu.classList.toggle("hidden", !show);
  $("profile-button").setAttribute("aria-expanded", String(show));
  fit();
  if (show) menu.querySelector("button")?.focus();
}

async function selectProfile(id) {
  profileId = id;
  toggleMenu(false);
  await invoke("select_popover_profile", { id });
  render();
}

function banner(profile) {
  const b = profile?.banner;
  if (!b) return null;
  const text =
    b.kind === "expired" ? t("bannerExpired") : b.kind === "failed" ? t("bannerFailed", b.failures) : t("bannerStale", b.minutes);
  return h("div", { class: `banner ${b.kind}`, role: "status" }, icon(ICONS.warning, 14, 2), h("span", { class: "grow" }, text));
}

function card(row) {
  const s = state.settings;
  const remaining = s.showRemaining;
  const shown = remaining ? Math.max(0, 100 - row.used) : row.used;

  // コンパクトでは見出しの幅が狭いので、追加使用量は縮めて正式な名前をツールチップに入れる
  const compact = s.popoverLayout === "compact";
  let title;
  let titleTip = null;
  let sub = null;
  if (row.kind === "session") {
    title = t("sessionShort");
  } else if (row.kind === "weekly") {
    title = t("weekly");
  } else {
    title = compact ? t("extraShort") : t("extra");
    titleTip = compact ? t("extra") : null;
    sub = `${money(row.extraUsed).split(" ")[0]} / ${money(row.extraLimit)}`;
  }

  const fill = h("div", { class: `bar-fill fill-${row.level}` });
  fill.style.width = `${Math.min(100, shown)}%`;
  const bar = h("div", { class: "bar" }, h("div", { class: "bar-track" }), fill);
  if (row.elapsed != null) {
    const pos = remaining ? 1 - row.elapsed : row.elapsed;
    const marker = h("div", { class: "bar-marker" });
    marker.style.left = `calc(${(pos * 100).toFixed(2)}% - 1px)`;
    marker.style.background = s.markerMode === "pace" && row.pace != null ? `var(--pace-${row.pace})` : "var(--marker)";
    bar.append(marker);
  }

  return h(
    "div",
    { class: "card" },
    h(
      "div",
      { class: "card-head" },
      h("div", { class: "card-title", title: titleTip }, title),
      h("div", { class: "grow" }),
      h("div", { class: `percent level-${row.level}` }, `${Math.round(shown)}%`),
    ),
    sub && h("div", { class: "card-sub" }, sub),
    bar,
    row.kind !== "extra" &&
      (row.resetsAtMs != null
        ? h("div", { class: "card-reset", title: resetText(row.resetsAtMs, Date.now()) }, resetShortText(row.resetsAtMs, Date.now()))
        : h("div", { class: "card-reset" }, t("waitingStart"))),
  );
}

function render() {
  if (!state) return;
  setLang(state.lang);
  applyTheme(state.settings.theme);
  $("popover").classList.toggle("compact", state.settings.popoverLayout === "compact");
  const profile = currentProfile();
  renderHeader(profile);
  if (!$("profile-menu").classList.contains("hidden")) renderMenu();

  $("banner").replaceChildren(...[banner(profile)].filter(Boolean));

  const cards = [];
  if (profile && profile.rows.length > 0) {
    cards.push(...profile.rows.map(card));
    if (profile.balance) {
      cards.push(
        h("div", { class: "balance" }, h("span", { class: "label" }, t("balance")), h("span", { class: "value" }, money(profile.balance))),
      );
    }
  } else if (!profile?.banner) {
    cards.push(h("div", { class: "message" }, t("loading")));
  }
  $("cards").replaceChildren(...cards);
  fit();
}

/** 内容の高さをウィンドウに伝える。メニューが開いていればその下端まで含める。 */
let fitQueued = false;
function fit() {
  if (fitQueued) return;
  fitQueued = true;
  requestAnimationFrame(() => {
    fitQueued = false;
    const card = $("popover").getBoundingClientRect();
    const menu = $("profile-menu");
    const menuBottom = menu.classList.contains("hidden") ? 0 : menu.getBoundingClientRect().bottom;
    const height = Math.ceil(Math.max(card.bottom, menuBottom) + 10);
    invoke("fit_popover", { height });
  });
}

async function load() {
  state = await invoke("get_state");
  if (!profileId || !state.profiles.some((p) => p.id === profileId)) {
    const info = await invoke("popover_info");
    profileId = info.profileId;
    pinned = info.pinned;
  }
  render();
}

$("profile-button").addEventListener("click", () => toggleMenu());
$("status").addEventListener("click", () => invoke("open_link", { key: "status" }));
$("refresh").addEventListener("click", () => invoke("refresh_now"));
$("settings").addEventListener("click", () => invoke("open_settings"));
$("pin").addEventListener("click", () => invoke("set_pinned", { pinned: !pinned }));

document.addEventListener("keydown", (e) => {
  if (e.key !== "Escape") return;
  if (!$("profile-menu").classList.contains("hidden")) toggleMenu(false);
  else invoke("hide_popover");
});

document.addEventListener("click", (e) => {
  if (!e.target.closest(".profile-picker") && !$("profile-menu").classList.contains("hidden")) toggleMenu(false);
});

listen("popover-open", async (e) => {
  profileId = e.payload;
  $("profile-menu").classList.add("hidden");
  await load();
});
listen("popover-pinned", (e) => {
  pinned = e.payload;
  render();
});
listen("state-changed", load);

new ResizeObserver(fit).observe($("popover"));

// リセットまでの残り時間を進める
setInterval(render, 30_000);
load();
