// The site's language choice (T-508, D-086). Loaded in <head> as a plain,
// blocking script from our own origin (no inline script, the CSP is
// unchanged), so a redirect happens before anything is drawn. Only in the
// accounts-open build; the closed build ships no script and every page keeps
// its plain links.
//
// English pages are at /…, Spanish ones at /es/…. The language is:
//   1. the viewer's own choice, saved when they use the switch in the menu;
//   2. else, on a first visit, the browser's language (Spanish or English,
//      whichever comes first in its list), only where a choice can be
//      remembered: with storage blocked a guess could not be undone;
//   3. else the page as linked.
// A redirect keeps the query and the hash (an email link's code stays).
// Nothing leaves the browser: the choice is one localStorage entry.
(function (win) {
  "use strict";
  var KEY = "tl-lang";

  function pathLang(path) {
    return path === "/es" || path.indexOf("/es/") === 0 ? "es" : "en";
  }

  function pathIn(path, lang) {
    var base = path === "/es" ? "/" : path.indexOf("/es/") === 0 ? path.slice(3) : path;
    // One leading slash only: "//host" would leave the site.
    base = base.replace(/^\/+/, "/");
    return lang === "es" ? "/es" + base : base;
  }

  function saved() {
    try {
      var value = win.localStorage.getItem(KEY);
      return value === "en" || value === "es" ? value : null;
    } catch (e) {
      return null;
    }
  }

  function save(lang) {
    try {
      win.localStorage.setItem(KEY, lang);
    } catch (e) {
      // Storage blocked: the switch still works, it is just not remembered.
    }
  }

  function canRemember() {
    try {
      win.localStorage.setItem(KEY + "-test", "1");
      win.localStorage.removeItem(KEY + "-test");
      return true;
    } catch (e) {
      return false;
    }
  }

  function browserLang() {
    var list = win.navigator.languages && win.navigator.languages.length ? win.navigator.languages : [win.navigator.language];
    for (var i = 0; i < list.length; i++) {
      var base = String(list[i] || "").toLowerCase().split("-")[0];
      if (base === "es" || base === "en") return base;
    }
    return null;
  }

  /** Where this page should go, or null to stay. */
  function target(loc) {
    var here = pathLang(loc.pathname);
    var wanted = saved() || (canRemember() ? browserLang() : null);
    // A Spanish link is followed as it is unless English was chosen.
    if (here === "es" && saved() !== "en") return null;
    if (!wanted || wanted === here) return null;
    return pathIn(loc.pathname, wanted) + loc.search + loc.hash;
  }

  var to = target(win.location);
  if (to) {
    win.location.replace(to);
    return;
  }

  // The menu shows "Sign in" or "Account", not both (T-513): signed in when
  // the browser holds a Supabase session (supabase-js keeps it in
  // localStorage as sb-<project>-auth-token). Only that key's name is read,
  // never its value. Unknown (storage blocked) leaves both links.
  function signedIn() {
    try {
      var store = win.localStorage;
      for (var i = 0; i < store.length; i++) {
        if (/^sb-[a-z0-9]+-auth-token$/.test(store.key(i) || "")) return true;
      }
      return false;
    } catch (e) {
      return null;
    }
  }
  var auth = signedIn();
  if (auth !== null) win.document.documentElement.setAttribute("data-auth", auth ? "in" : "out");

  // The switch in the menu: remember the choice, and keep the page's query
  // and hash (filters, a profile id, an open period).
  function withPlace(link) {
    var url = new URL(link.getAttribute("href"), win.location.href);
    return url.pathname + win.location.search + win.location.hash;
  }
  // A click, or a middle click that opens the page in a new tab.
  function onSwitch(event) {
    var link = event.target && event.target.closest ? event.target.closest("a[data-lang-switch]") : null;
    if (!link) return;
    save(link.getAttribute("data-lang-switch"));
    link.setAttribute("href", withPlace(link));
  }
  win.document.addEventListener("click", onSwitch);
  win.document.addEventListener("auxclick", onSwitch);
  win.document.addEventListener("DOMContentLoaded", function () {
    var links = win.document.querySelectorAll("a[data-lang-switch]");
    for (var i = 0; i < links.length; i++) links[i].setAttribute("href", withPlace(links[i]));
  });
})(window);
