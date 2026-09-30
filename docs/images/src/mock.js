// README の画像を作るための見本データ。Tauri の代わりに window.__TAURI__ を用意し、
// 実際の画面（ui/）に見本の値を渡す。実在のアカウントの値は使わない。

(() => {
  const now = Date.now();
  const min = 60_000;
  const hour = 60 * min;
  const day = 24 * hour;
  const weekReset = now + day + 23 * hour;

  // URL の ?theme=light&profile=work で、テーマと表示するプロファイルを選ぶ
  const params = new URLSearchParams(location.search);

  const personalRows = [
    { kind: "session", used: 39, resetsAtMs: now + 2 * hour + 10 * min, elapsed: 0.567, level: 0, pace: 1 },
    { kind: "weekly", used: 62, resetsAtMs: weekReset, elapsed: 0.72, level: 1, pace: 2 },
    {
      kind: "extra",
      used: 37.2,
      resetsAtMs: null,
      elapsed: null,
      level: 0,
      pace: null,
      extraUsed: { amount: 1.86, currency: "USD" },
      extraLimit: { amount: 5, currency: "USD" },
    },
  ];

  const workRows = [
    { kind: "session", used: 74, resetsAtMs: now + 36 * min, elapsed: 0.88, level: 1, pace: 2 },
    { kind: "weekly", used: 41, resetsAtMs: weekReset, elapsed: 0.72, level: 0, pace: 1 },
  ];

  const profile = (id, name, dir, rows, balance) => ({
    id,
    name,
    dirDisplay: dir,
    allowTokenRefresh: true,
    rows,
    balance,
    banner: null,
    lastSuccessMs: now - min,
  });

  const state = {
    lang: "ja",
    version: "0.1.0",
    releaseDate: "2026-09-30",
    settings: {
      refreshMinutes: 5,
      iconStyle: "ring",
      trayMarker: true,
      colorMode: "color",
      markerMode: "white",
      showRemaining: false,
      theme: params.get("theme") ?? "dark",
      autostart: false,
    },
    profiles: [
      profile("personal", "personal", "%USERPROFILE%\\.claude", personalRows, { amount: 12.4, currency: "USD" }),
      profile("work", "work", "%USERPROFILE%\\.claude-work", workRows, null),
    ],
    status: "none",
    nowMs: now,
  };

  const history = [];
  for (let t = now - 7 * day; t <= now; t += 30 * min) {
    const h = ((t - (now - 7 * day)) / hour) % 5;
    history.push({ t, s: Math.min(100, h * 14 + 5 * Math.sin(t / hour)), w: ((t - (now - 7 * day)) / (7 * day)) * 62 });
  }

  const results = {
    get_state: () => structuredClone(state),
    popover_info: () => ({ profileId: params.get("profile") ?? "personal", pinned: false }),
    get_history: () => history,
  };

  window.__TAURI__ = {
    core: { invoke: async (cmd) => (results[cmd] ? results[cmd]() : null) },
    event: { listen: async () => () => {} },
  };
})();
