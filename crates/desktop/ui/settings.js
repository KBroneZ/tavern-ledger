// The Settings page (T-324): the language (T-325) and Windows start-up. The
// overlay switch is overlay-switch.js and the upload panel upload.js; they
// sit on the same page. Changes apply at once; each is kept where the app
// keeps it (settings.json, Windows' start-up entry). A language change
// reloads both windows (i18n.js), and this page opens again where it was.
"use strict";

(() => {
  const { invoke: call } = window.__TAURI__.core;
  const { listen: onEvent } = window.__TAURI__.event;
  const I18n = window.TLI18n;
  const { t } = I18n;
  const $ = (id) => document.getElementById(id);
  const REOPEN = "tl-reopen-settings";
  let saved = null; // the saved choice: "en", "es" or null for the system language

  // The problem lines are live regions: they stay in the page, empty when
  // there is nothing to say, so a screen reader hears the text when it comes.
  function problem(id, text) {
    $(id).textContent = text || "";
  }

  // "Windows language (Español)", then each language by its own name.
  function languageOptions() {
    const system = I18n.pick(null, navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language]);
    const select = $("setting-language");
    const option = (value, text) => {
      const node = document.createElement("option");
      node.value = value;
      node.textContent = text;
      node.selected = value === (saved || "");
      return node;
    };
    select.replaceChildren(
      option("", t("settings.language.system", { name: t(`lang.${system}`) })),
      ...I18n.LANGS.map((l) => {
        const node = option(l, t(`lang.${l}`));
        node.lang = l;
        return node;
      }),
    );
  }

  $("setting-language").addEventListener("change", (e) => {
    const language = e.target.value || null;
    problem("language-problem", "");
    // Only a change of the shown language reloads the page; then it opens here again.
    const system = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language];
    if (I18n.pick(language, system) !== I18n.lang()) {
      try {
        sessionStorage.setItem(REOPEN, "1");
      } catch (_) {
        // Without session storage the page opens on the first tab; nothing is lost.
      }
    }
    // The answer reloads the page (i18n.js); an error leaves it as it is.
    call("app_set_language", { language }).then((v) => {
      saved = v ? v.language : language;
    }, (error) => {
      problem("language-problem", t("settings.language.failed", { error: I18n.backend(String(error)) }));
      // Kept for this run only when saving failed: the event still reloads the
      // page in that language; if nothing changed, the list shows the old choice.
      languageOptions();
    });
  });

  function showAutostart(v) {
    const box = $("setting-autostart");
    box.checked = Boolean(v.enabled);
    box.disabled = !v.available;
    const note = $("autostart-note");
    note.textContent = v.available ? "" : t("settings.autostart.unavailable");
    note.hidden = v.available;
    problem("autostart-problem", "");
  }

  const autostartFailed = (error) => problem("autostart-problem", t("settings.autostart.failed", { error: I18n.backend(String(error)) }));
  const refreshAutostart = () => call("app_autostart").then(showAutostart).catch(autostartFailed);

  $("setting-autostart").addEventListener("change", (e) => {
    e.target.disabled = true;
    call("app_set_autostart", { enabled: e.target.checked })
      .then(showAutostart)
      .catch((error) => {
        autostartFailed(error);
        refreshAutostart();
      });
  });

  // Back on Settings after a language change, with focus on the language.
  function reopen() {
    let asked = false;
    try {
      asked = sessionStorage.getItem(REOPEN) === "1";
      sessionStorage.removeItem(REOPEN);
    } catch (_) {
      asked = false;
    }
    if (!asked) return;
    $("tab-settings").click();
    $("setting-language").focus();
  }

  I18n.ready().then(async () => {
    saved = await call("app_settings").then((s) => (s && s.language) || null, () => null);
    languageOptions();
    reopen();
    await onEvent("autostart-changed", refreshAutostart);
    await refreshAutostart();
  });
})();
