// The installable web app's page side (T-637). Loaded after the page by every
// page of the accounts-open build, as a plain script from our own origin (no
// inline script; the closed build ships none). It does three small things:
//   1. registers /sw.js, the worker that keeps the shell and the pages you
//      visited for offline reading (it never keeps account pages, sign-in
//      pages or anything from another site; see lib/pwa/rules.js);
//   2. shows the footer's "Add to home screen" line where the browser can
//      install the site: a button when Chrome offers an install, a line of
//      words in Safari on iPhone and iPad, which have no install prompt. It
//      never opens a prompt on its own and is hidden once the site is
//      installed;
//   3. makes the offline page's "Try again" button reload the page.
// Nothing leaves the browser.
(function (win, doc) {
  "use strict";
  var nav = win.navigator;

  if ("serviceWorker" in nav) {
    win.addEventListener("load", function () {
      nav.serviceWorker.register("/sw.js", { scope: "/", updateViaCache: "none" }).catch(function (error) {
        win.console.warn("Tavern Ledger: offline reading is off, the worker did not start.", error && error.message);
      });
    });
  }

  function installed() {
    return (win.matchMedia && win.matchMedia("(display-mode: standalone)").matches) || nav.standalone === true;
  }

  // Safari on iPhone and iPad (iPadOS 13 and later says "Macintosh" but has a touch screen).
  function iosSafari() {
    var ua = nav.userAgent || "";
    var ios = /iPhone|iPad|iPod/.test(ua) || (nav.platform === "MacIntel" && nav.maxTouchPoints > 1);
    return ios && /Safari/.test(ua) && !/CriOS|FxiOS|EdgiOS|OPiOS|GSA/.test(ua);
  }

  var hint = doc.querySelector("[data-install]");
  if (hint && !installed()) {
    var button = hint.querySelector("[data-install-button]");
    var words = hint.querySelector("[data-install-ios]");
    var offered = null;
    var hide = function () {
      hint.hidden = true;
    };
    win.addEventListener("beforeinstallprompt", function (event) {
      // Keep the browser's own banner away; the player starts it from the footer.
      event.preventDefault();
      offered = event;
      button.hidden = false;
      hint.hidden = false;
    });
    button.addEventListener("click", function () {
      if (!offered) return;
      var event = offered;
      offered = null;
      hide();
      Promise.resolve(event.prompt()).catch(function (error) {
        win.console.warn("Tavern Ledger: the install prompt did not open.", error && error.message);
      });
    });
    win.addEventListener("appinstalled", hide);
    if (iosSafari()) {
      words.hidden = false;
      hint.hidden = false;
    }
  }

  var reload = doc.querySelector("[data-reload]");
  if (reload) {
    reload.addEventListener("click", function () {
      win.location.reload();
    });
  }
})(window, document);
