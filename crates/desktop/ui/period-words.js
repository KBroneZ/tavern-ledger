// Words for the main window's "Today and this week" summary (T-361): the
// viewer's local day and week, and the summary the app gives for each, worded
// per mode. Pure: plain data in, plain data out, so it is tested without a page
// (ui-tests/period-words.test.js). app.js draws it with textContent only. The
// app decides every value; this file only words them.
//
// Empty is not unknown is not zero: no games says so, a missing average says
// "no finished game", and the rating change is its own state (off, unknown or
// a number, where "net 0" is a number the app knows).
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  const SOLO = "GT_BATTLEGROUNDS";
  const DUOS = "GT_BATTLEGROUNDS_DUO";
  const MINUS = "−";
  const REASONS = ["best_place", "close_loss", "big_swing"];

  // The viewer's own clock: the parts of a moment and the UTC time of a local
  // midnight. The tests swap it for a fixed zone with named times.
  const localZone = {
    parts: (now) => ({ y: now.getFullYear(), m: now.getMonth(), d: now.getDate(), weekday: now.getDay() }),
    midnight: (y, m, d) => new Date(y, m, d),
  };

  // The local day and the local week, as the app wants them: local midnights
  // in UTC seconds. The week starts on Monday and ends the next Monday; a
  // clock change makes a day 23 or 25 hours long, and so the week 167 or 169.
  function ranges(now, zone = localZone) {
    const { y, m, d, weekday } = zone.parts(now);
    const sinceMonday = (weekday + 6) % 7;
    const secs = (date) => Math.floor(date.getTime() / 1000);
    const monday = zone.midnight(y, m, d - sinceMonday);
    return {
      todayStart: secs(zone.midnight(y, m, d)),
      todayEnd: secs(zone.midnight(y, m, d + 1)),
      weekStart: secs(monday),
      weekEnd: secs(zone.midnight(y, m, d - sinceMonday + 7)),
      weekFrom: monday,
    };
  }

  // "+40" / "−25"; null for anything that is not a change.
  function signed(delta) {
    if (!Number.isInteger(delta) || delta === 0) return null;
    return `${delta > 0 ? "+" : MINUS}${I18n.num(Math.abs(delta))}`;
  }

  // The rating change of a mode: off, unknown or a number. Anything the app
  // does not state plainly is unknown, never zero.
  function net(state) {
    const kind = state && state.state;
    if (kind === "off") return { state: "off", text: I18n.t("period.net.off"), up: null };
    if (kind === "known" && Number.isInteger(state.change)) {
      if (state.change === 0) return { state: "known", text: I18n.t("period.net.zero"), up: null };
      return { state: "known", text: I18n.t("period.net.known", { value: signed(state.change) }), up: state.change > 0 };
    }
    return { state: "unknown", text: I18n.t("period.net.unknown"), up: null };
  }

  function revisit(r, nameOf) {
    const hero = r.hero && r.hero.id ? r.hero : null;
    const reason = I18n.t(`period.reason.${r.reason}`);
    const placeText = I18n.t("period.revisit.place", { n: r.place });
    const change = signed(r.change);
    const said = [reason];
    if (hero) said.push(nameOf(hero));
    said.push(placeText);
    if (change) said.push(change);
    return { reason: r.reason, session: r.session, index: r.index, hero, place: r.place, placeText, change, up: change ? r.change > 0 : null, label: reason, said: said.join(", ") };
  }

  // One row per mode played: games, average place, record, the rating change
  // and the games worth revisiting. Every number says how many games are behind
  // it: a game with no place (not finished in the log) counts in the games but
  // not in the average or the record, so those say "9 with a place".
  function modeRow(m, nameOf) {
    const t = m.summary.tally;
    const placed = Number.isInteger(t.placed) ? t.placed : 0;
    const half = m.summary.top_half;
    const topGames = Number.isInteger(m.summary.top_half_games) ? m.summary.top_half_games : 0;
    const parts = [I18n.plural("period.games", t.games)];
    if (placed > 0 && placed < t.games) parts.push(I18n.t("period.placed", { n: I18n.num(placed) }));
    const average =
      placed > 0 && typeof t.average_place === "number"
        ? I18n.t("period.avg", { value: I18n.num(t.average_place, 1) })
        : I18n.t("period.avg.none");
    const record =
      placed > 0
        ? I18n.t("period.record", { top: I18n.num(topGames), bottom: I18n.num(placed - topGames), half })
        : I18n.t("period.record.none");
    const revisits = (Array.isArray(m.revisit) ? m.revisit : [])
      .filter((r) => r && REASONS.includes(r.reason) && Number.isInteger(r.place) && Number.isInteger(r.index) && typeof r.session === "string")
      .map((r) => revisit(r, nameOf));
    return {
      mode: m.summary.mode,
      label: I18n.t(`mode.${m.summary.mode}`),
      games: parts.join(" · "),
      average,
      record,
      net: net(m.net_rating),
      revisit: revisits,
    };
  }

  // The words of one stretch (`list` is the app's answer for it). No mode with
  // a game is "no games", not a row of zeros.
  function period(list, nameOf) {
    const rows = (Array.isArray(list) ? list : [])
      .filter((m) => m && m.summary && (m.summary.mode === SOLO || m.summary.mode === DUOS) && m.summary.tally && m.summary.tally.games > 0)
      .map((m) => modeRow(m, nameOf));
    return { empty: rows.length === 0, modes: rows };
  }

  // "The week starts on Monday (19/10/2026)": which day the week starts on,
  // said in the window.
  function weekNote(range) {
    return I18n.t("period.weekNote", { date: I18n.date(range.weekFrom) });
  }

  const api = { ranges, signed, net, period, weekNote };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLPeriod = api;
})();
