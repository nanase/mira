// 画面の文言と、時刻・金額の書式。Rust 側の文言は src-tauri/src/i18n.rs にある。

const ja = {
  statusNone: "すべてのシステムは正常です",
  statusMinor: "一部のシステムで軽微な障害が起きています",
  statusMajor: "一部のシステムで障害が起きています",
  statusCritical: "重大な障害が起きています",
  statusUnknown: "稼働状況を取得できません",
  statusLoading: "稼働状況を確認しています…",
  pickProfile: "表示するプロファイルを選ぶ",
  pin: "ウィンドウとして固定",
  unpin: "固定を解除",
  refresh: "今すぐ更新",
  settings: "設定",
  session: "セッション",
  sessionSub: "5 時間ウィンドウ",
  allModels: "すべてのモデル",
  weekly: "週間",
  extra: "追加使用量",
  balance: "クレジット残高",
  resets: (rel, abs) => `${rel}後にリセット（${abs}）`,
  days: (d, h) => `${d}日${h}時間`,
  hours: (h, m) => `${h}時間${m}分`,
  minutes: (m) => `${m}分`,
  bannerExpired: "認証が切れています。Claude Code で再ログインしてください",
  bannerFailed: (n) => `更新に ${n} 回続けて失敗しました`,
  bannerStale: (n) => `${n} 分前のデータを表示しています`,
  footer: (time, n) => `最終更新 ${time} · ${n} 分ごとに自動更新`,
  footerNever: (n) => `まだ取得していません · ${n} 分ごとに自動更新`,
  loading: "使用量を取得しています…",

  settingsTitle: "Mira の設定",
  navGeneral: "一般",
  navAppearance: "表示",
  navProfiles: "プロファイル",
  navHistory: "履歴",
  navAbout: "情報",
  refreshInterval: "更新間隔",
  refreshIntervalDesc: "使用量を取得し直す間隔。短くすると API に拒否されやすくなります",
  everyMinutes: (n) => `${n} 分`,
  autostart: "Windows へのサインイン時に起動",
  autostartDesc: "スタートアップに Mira を登録します",
  theme: "テーマ",
  themeSystem: "Windows の設定に合わせる",
  themeDark: "ダーク",
  themeLight: "ライト",
  iconStyle: "トレイアイコンのスタイル",
  trayIcon: "トレイアイコン",
  trayMarker: "時間マーカーを表示",
  trayMarkerDesc: "経過時間の位置に目盛りを重ねます。数字のスタイルでは表示されません",
  styleRing: "リング",
  styleRingDesc: "5 時間の使用率を弧で表す",
  styleDouble: "二重リング",
  styleDoubleDesc: "外側が 5 時間、内側が週間",
  styleNumber: "数字",
  styleNumberDesc: "5 時間の使用率を数字で出す",
  styleBars: "縦バー 2 本",
  styleBarsDesc: "左が 5 時間、右が週間",
  iconColor: "アイコンの色",
  colorColor: "カラー",
  colorMono: "モノクロ",
  markerColor: "時間マーカーの色",
  markerWhite: "白",
  markerPace: "ペース色（6 段階）",
  percentMode: "パーセントの表示",
  percentUsed: "使用済み",
  percentRemaining: "残り",
  profilesDesc: "Claude Code の設定フォルダごとにプロファイルを作ります。フォルダの中身は許可したトークンの更新以外では変更しません。",
  addFolder: "フォルダを追加",
  rescan: "もう一度検出",
  rescanNone: "新しい設定フォルダは見つかりませんでした",
  addNoCredentials: "選んだフォルダに .credentials.json がありません",
  addExists: "このフォルダは登録済みです",
  noProfiles: "プロファイルがありません。Claude Code にログインしてから「もう一度検出」を押してください。",
  profileName: "表示名",
  folder: "フォルダ",
  tokenRefresh: "トークンの自動更新",
  tokenRefreshDesc: "期限切れのとき .credentials.json の 3 項目を書き換えます",
  tokenRefreshUnasked: "未回答",
  remove: "外す",
  removeConfirm: "外しますか？",
  yes: "外す",
  no: "やめる",
  historyProfile: "プロファイル",
  range: "期間",
  range1: "1 日",
  range7: "7 日",
  range30: "30 日",
  range90: "90 日",
  historyEmpty: "この期間の記録はまだありません",
  historyNote: "記録は 90 日間保存します",
  showTable: "表で見る",
  aboutDesc: "Claude の使用量制限をタスクトレイに表示するアプリです。",
  destinations: "通信先",
  destUsage: "使用量の取得",
  destToken: "トークンの更新（許可したプロファイルだけ）",
  destStatus: "稼働状況の取得",
  credits: "クレジット",
  creditBefore: "このプロジェクトは ",
  creditAfter: " を参考に制作されました。",
  openRepo: "リポジトリを開く",
  quit: "Mira を終了",

  step1: "手順 1 ・ 読み取りのみ",
  step1Title: "Claude Code の設定フォルダが見つかりました",
  step1Body: "次のフォルダをプロファイルとして登録します。登録しても、このフォルダの中身は変更しません。",
  readFiles: "読み取るファイル",
  readFilesValue: "（使用量の取得に使うトークン）",
  register: "登録する",
  dontRegister: "登録しない",
  step2: "手順 2 ・ ファイルへの書き込み",
  step2Title: "トークンの自動更新を許可しますか？",
  step2Body: "アクセストークンの期限が切れたとき、Mira が新しいトークンを受け取り、次のファイルに書き込みます。Claude Code も同じファイルを使うため、書き込まないと Claude Code 側のログインが切れます。",
  targetFile: "対象ファイル",
  fields: "書き換える項目",
  fieldsValue: "accessToken、refreshToken、expiresAt の 3 つだけ。ほかの項目は変更しません",
  timing: "タイミング",
  timingValue: "期限切れを検出したときだけ",
  destination: "通信先",
  step2Note: "許可しない場合は読み取りのみで動きます。期限が切れたら、Claude Code を起動するよう案内します。この設定はあとから変更できます。",
  allow: "許可する",
  deny: "許可しない",
};

const en = {
  statusNone: "All systems operational",
  statusMinor: "Minor service disruption",
  statusMajor: "Partial service outage",
  statusCritical: "Major service outage",
  statusUnknown: "Status unavailable",
  statusLoading: "Checking status…",
  pickProfile: "Choose a profile to show",
  pin: "Pin as a window",
  unpin: "Unpin",
  refresh: "Refresh now",
  settings: "Settings",
  session: "Session",
  sessionSub: "5-hour window",
  allModels: "All models",
  weekly: "Weekly",
  extra: "Extra usage",
  balance: "Credit balance",
  resets: (rel, abs) => `Resets in ${rel} (${abs})`,
  days: (d, h) => `${d}d ${h}h`,
  hours: (h, m) => `${h}h ${m}m`,
  minutes: (m) => `${m}m`,
  bannerExpired: "Sign-in expired. Sign in again in Claude Code",
  bannerFailed: (n) => `Update failed ${n} times in a row`,
  bannerStale: (n) => `Showing data from ${n} min ago`,
  footer: (time, n) => `Updated ${time} · refreshes every ${n} min`,
  footerNever: (n) => `Not fetched yet · refreshes every ${n} min`,
  loading: "Fetching usage…",

  settingsTitle: "Mira Settings",
  navGeneral: "General",
  navAppearance: "Appearance",
  navProfiles: "Profiles",
  navHistory: "History",
  navAbout: "About",
  refreshInterval: "Refresh interval",
  refreshIntervalDesc: "How often usage is fetched. Short intervals are more likely to be rate-limited",
  everyMinutes: (n) => `${n} min`,
  autostart: "Start when you sign in to Windows",
  autostartDesc: "Adds Mira to your startup apps",
  theme: "Theme",
  themeSystem: "Match Windows",
  themeDark: "Dark",
  themeLight: "Light",
  iconStyle: "Tray icon style",
  trayIcon: "Tray icon",
  trayMarker: "Show time marker",
  trayMarkerDesc: "Adds a tick at the elapsed time. Not shown with the Number style",
  styleRing: "Ring",
  styleRingDesc: "An arc for 5-hour usage",
  styleDouble: "Double ring",
  styleDoubleDesc: "Outer: 5-hour, inner: weekly",
  styleNumber: "Number",
  styleNumberDesc: "5-hour usage as a number",
  styleBars: "Two bars",
  styleBarsDesc: "Left: 5-hour, right: weekly",
  iconColor: "Icon color",
  colorColor: "Color",
  colorMono: "Monochrome",
  markerColor: "Time marker color",
  markerWhite: "White",
  markerPace: "Pace colors (6 levels)",
  percentMode: "Show percentage as",
  percentUsed: "Used",
  percentRemaining: "Remaining",
  profilesDesc: "Each Claude Code config folder becomes a profile. Mira never changes the folder, except for token refresh you allow.",
  addFolder: "Add folder",
  rescan: "Detect again",
  rescanNone: "No new config folders were found",
  addNoCredentials: "The folder has no .credentials.json",
  addExists: "This folder is already registered",
  noProfiles: "No profiles yet. Sign in to Claude Code, then press “Detect again”.",
  profileName: "Display name",
  folder: "Folder",
  tokenRefresh: "Refresh tokens automatically",
  tokenRefreshDesc: "Rewrites 3 fields of .credentials.json when the token expires",
  tokenRefreshUnasked: "Not answered",
  remove: "Remove",
  removeConfirm: "Remove this profile?",
  yes: "Remove",
  no: "Cancel",
  historyProfile: "Profile",
  range: "Range",
  range1: "1 day",
  range7: "7 days",
  range30: "30 days",
  range90: "90 days",
  historyEmpty: "No records in this range yet",
  historyNote: "Records are kept for 90 days",
  showTable: "Show as a table",
  aboutDesc: "Shows your Claude usage limits in the system tray.",
  destinations: "Network destinations",
  destUsage: "Fetching usage",
  destToken: "Refreshing tokens (allowed profiles only)",
  destStatus: "Fetching service status",
  credits: "Credits",
  creditBefore: "This project was inspired by ",
  creditAfter: ".",
  openRepo: "Open the repository",
  quit: "Quit Mira",

  step1: "Step 1 · Read only",
  step1Title: "Found a Claude Code config folder",
  step1Body: "Mira will register this folder as a profile. Registering does not change anything inside the folder.",
  readFiles: "Files read",
  readFilesValue: " (the token used to fetch usage)",
  register: "Register",
  dontRegister: "Don't register",
  step2: "Step 2 · Writing a file",
  step2Title: "Allow automatic token refresh?",
  step2Body: "When the access token expires, Mira gets a new one and writes it to the file below. Claude Code uses the same file, so without writing it back, Claude Code would be signed out.",
  targetFile: "File",
  fields: "Fields changed",
  fieldsValue: "Only accessToken, refreshToken and expiresAt. Nothing else is changed",
  timing: "When",
  timingValue: "Only when the token is found to be expired",
  destination: "Destination",
  step2Note: "If you don't allow it, Mira works read-only and asks you to start Claude Code when the token expires. You can change this later.",
  allow: "Allow",
  deny: "Don't allow",
};

let dict = ja;
let lang = "ja";

export function setLang(value) {
  lang = value === "en" ? "en" : "ja";
  dict = lang === "en" ? en : ja;
  document.documentElement.lang = lang;
}

export function t(key, ...args) {
  const v = dict[key] ?? ja[key] ?? key;
  return typeof v === "function" ? v(...args) : v;
}

const pad = (n) => String(n).padStart(2, "0");

/** 今日なら「16:40」、それ以外は「10/2 02:00」。 */
export function shortTime(ms) {
  const d = new Date(ms);
  const now = new Date();
  const hm = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  return d.toDateString() === now.toDateString() ? hm : `${d.getMonth() + 1}/${d.getDate()} ${hm}`;
}

/** 「2時間10分後にリセット（16:40）」 */
export function resetText(resetMs, nowMs) {
  const total = Math.max(1, Math.round((resetMs - nowMs) / 60000));
  const d = Math.floor(total / 1440);
  const h = Math.floor((total % 1440) / 60);
  const m = total % 60;
  const rel = d > 0 ? t("days", d, h) : h > 0 ? t("hours", h, m) : t("minutes", m);
  return t("resets", rel, shortTime(resetMs));
}

export function money(m) {
  return `${m.amount.toFixed(2)} ${m.currency}`;
}

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

export const tauri = window.__TAURI__;
export const invoke = (cmd, args) => tauri.core.invoke(cmd, args);
export const listen = (event, fn) => tauri.event.listen(event, fn);
