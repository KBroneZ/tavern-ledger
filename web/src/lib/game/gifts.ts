// Dark Gifts on a minion (T-528, D-107) as words: each gift's name and our
// short summary in the page's language, from src/data/gifts.json (the one
// table the app shares, tools/gift_words.py). A gift the table does not know
// shows its card id with no summary; a minion the game marks as gifted with
// no gift named shows "Dark Gift (unknown)".
import table from "../../data/gifts.json" with { type: "json" };
import { lang } from "../i18n.ts";
import { g } from "./strings.ts";

interface Words {
  name: string;
  text: string;
}
const GIFTS: Readonly<Record<string, { en: Words; es: Words }>> = table.gifts;

export interface GiftMark {
  /** The gift's card id; null for a gift the log does not name. */
  id: string | null;
  name: string;
  /** "Dark Gift: name: summary", for the tooltip and screen readers. */
  title: string;
}

export function giftMarks(m: { gifts: readonly string[]; giftUnknown: boolean }): GiftMark[] {
  const marks = m.gifts.map((id) => {
    const words = Object.prototype.hasOwnProperty.call(GIFTS, id) ? GIFTS[id][lang()] : null;
    const name = words?.name ?? id;
    const tip = words ? g("gift.tip", { name, text: words.text }) : name;
    return { id, name, title: g("gift.title", { tip }) };
  });
  if (marks.length === 0 && m.giftUnknown) {
    marks.push({ id: null, name: g("gift.unknown"), title: g("gift.title", { tip: `${g("gift.unknown")}: ${g("gift.unknownText")}` }) });
  }
  return marks;
}

/** A gift's name in the page's language, else its card id (the choices list). */
export function giftName(id: string): string {
  return Object.prototype.hasOwnProperty.call(GIFTS, id) ? GIFTS[id][lang()].name : id;
}
