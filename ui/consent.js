import { applyTheme, h, invoke, listen } from "./dom.js";
import { setLang, t } from "./i18n.js";

const $ = (id) => document.getElementById(id);
let view = null;

function fact(key, value, mono = false) {
  return h("div", { class: "fact" }, h("span", { class: "key" }, key), h("span", { class: mono ? "mono" : null }, value));
}

function render() {
  const step1 = view.step === 1;
  $("step").textContent = step1 ? t("step1") : t("step2");
  $("step").classList.toggle("write", !step1);
  $("title").textContent = step1 ? t("step1Title") : t("step2Title");
  $("lead").textContent = step1 ? t("step1Body") : t("step2Body");

  if (step1) {
    $("facts").replaceChildren(
      fact(t("folder"), view.dirDisplay, true),
      h(
        "div",
        { class: "fact" },
        h("span", { class: "key" }, t("readFiles")),
        h("span", {}, h("span", { class: "mono" }, ".credentials.json"), t("readFilesValue")),
      ),
    );
  } else {
    $("facts").replaceChildren(
      fact(t("targetFile"), view.credentialsDisplay, true),
      fact(t("fields"), t("fieldsValue")),
      fact(t("timing"), t("timingValue")),
      fact(t("destination"), "platform.claude.com", true),
    );
  }

  $("name-label").classList.toggle("hidden", !step1);
  $("name-caption").textContent = t("profileName");
  if (step1) $("name").value = view.defaultName;
  $("note").classList.toggle("hidden", step1);
  $("note").textContent = t("step2Note");

  $("yes").textContent = step1 ? t("register") : t("allow");
  $("no").textContent = step1 ? t("dontRegister") : t("deny");

  requestAnimationFrame(() => (step1 ? $("name") : $("no")).focus());
}

// 幅が決まると折り返しが変わるので、大きさが変わるたびに測り直す
new ResizeObserver(() => {
  if (view) invoke("fit_consent", { height: Math.ceil($("dialog").getBoundingClientRect().height) });
}).observe($("dialog"));

async function load() {
  const state = await invoke("get_state");
  setLang(state.lang);
  applyTheme(state.settings.theme);
  view = await invoke("get_consent");
  if (view) render();
}

async function answer(yes) {
  if (!view) return;
  const step1 = view.step === 1;
  view = null;
  if (step1 && yes) await invoke("consent_register", { name: $("name").value });
  else if (step1) await invoke("consent_decline");
  else await invoke("consent_refresh", { allow: yes });
}

$("yes").addEventListener("click", () => answer(true));
$("no").addEventListener("click", () => answer(false));
$("name").addEventListener("keydown", (e) => {
  if (e.key === "Enter") answer(true);
});
document.addEventListener("keydown", (e) => {
  // Esc は断る側のボタンと同じ扱いにする
  if (e.key === "Escape") answer(false);
});

listen("consent-changed", load);
load();
