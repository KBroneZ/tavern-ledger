// Imported first by every page script: sets the language from the page's own
// <html lang> before anything draws (T-508, D-086).
import { setLang } from "../i18n.ts";

setLang(document.documentElement.lang);
