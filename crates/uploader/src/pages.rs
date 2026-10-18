//! The pages the sign-in listener shows in the browser (T-380): signed in,
//! a link that failed or expired, an old link, and anything else. They look
//! like the website (its colours and the mark, light and dark) but load
//! nothing: one inline style block pinned by hash in the CSP, no script, no
//! font or image fetched. Nothing from the request is echoed except an error
//! code that passes [`is_error_code`], and that is HTML-escaped anyway.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};

/// The app's language as the website names it. Anything else is English.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Es,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page<'a> {
    SignedIn,
    /// The link came back with an error; the code is shown if it is plain.
    LinkFailed(&'a str),
    Expired,
    OldLink,
    NotFound,
}

/// What a page says. `ok` picks the check mark and the green; the rest is
/// the warning mark.
struct Words {
    title: &'static str,
    text: &'static str,
    ok: bool,
}

const MAX_CODE: usize = 64;

/// An error code the server sends: letters, digits and `_`, short.
pub fn is_error_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= MAX_CODE
        && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn words(page: Page, lang: Lang) -> Words {
    let (title, text, ok) = match (page, lang) {
        (Page::SignedIn, Lang::En) => (
            "You are signed in",
            "Tavern Ledger on your PC is now linked to your account.",
            true,
        ),
        (Page::SignedIn, Lang::Es) => (
            "Sesión iniciada",
            "Tavern Ledger en tu PC ya está vinculado a tu cuenta.",
            true,
        ),
        (Page::LinkFailed(_), Lang::En) => (
            "This link did not work",
            "It may have expired or already been used. Ask for a new link in Tavern Ledger.",
            false,
        ),
        (Page::LinkFailed(_), Lang::Es) => (
            "Este enlace no funcionó",
            "Puede haber caducado o ya se usó. Pide un enlace nuevo en Tavern Ledger.",
            false,
        ),
        (Page::Expired, Lang::En) => (
            "This link has expired",
            "It is from an earlier attempt or has expired. Open the newest email from \
             Tavern Ledger, or ask for a new link in the app.",
            false,
        ),
        (Page::Expired, Lang::Es) => (
            "Este enlace ha caducado",
            "Es de un intento anterior o ha caducado. Abre el correo más reciente de \
             Tavern Ledger, o pide un enlace nuevo en la app.",
            false,
        ),
        (Page::OldLink, Lang::En) => (
            "This link is from an earlier attempt",
            "Open the newest email from Tavern Ledger.",
            false,
        ),
        (Page::OldLink, Lang::Es) => (
            "Este enlace es de un intento anterior",
            "Abre el correo más reciente de Tavern Ledger.",
            false,
        ),
        (Page::NotFound, Lang::En) => (
            "Page not found",
            "Nothing here. This address only works for the sign-in link from Tavern Ledger.",
            false,
        ),
        (Page::NotFound, Lang::Es) => (
            "Página no encontrada",
            "Aquí no hay nada. Esta dirección solo sirve para el enlace de inicio de \
             sesión de Tavern Ledger.",
            false,
        ),
    };
    Words { title, text, ok }
}

fn close_line(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "You can close this tab and go back to Tavern Ledger.",
        Lang::Es => "Ya puedes cerrar esta pestaña y volver a Tavern Ledger.",
    }
}

fn code_label(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Error code",
        Lang::Es => "Código de error",
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// The one style block. Colours are the website's tokens (web/src/styles/
/// global.css): dark is the site's own wood and gold, light is parchment.
/// The fonts are the system's: the site's would need a fetch.
const STYLE: &str = "\
:root{color-scheme:dark light;--bg:#120d0b;--card:#1e1612;--edge:#8a6232;--title:#f3dcb3;\
--text:#f7efe6;--dim:#cbbdaf;--accent:#ffb24a;--ok:#8fe0a8;--bad:#ff9a8a;--chip:#0c0807;\
--wood:#26180e;--shadow:0 18px 40px rgb(0 0 0/.5)}\
@media(prefers-color-scheme:light){:root{--bg:#f6efe4;--card:#fffaf1;--edge:#d3b98f;\
--title:#4a2c0f;--text:#2b1d12;--dim:#675240;--accent:#9a5a0a;--ok:#1f6b3a;--bad:#9b2c1c;\
--chip:#f0e4d0;--shadow:0 18px 40px rgb(80 50 20/.18)}}\
*{box-sizing:border-box}\
body{margin:0;min-height:100vh;display:grid;place-items:center;padding:24px;\
background:var(--bg);color:var(--text);\
font:1rem/1.55 \"Segoe UI\",system-ui,-apple-system,sans-serif}\
.card{width:100%;max-width:30rem;padding:36px 32px 30px;text-align:center;\
background:var(--card);border:1px solid var(--edge);border-radius:8px;\
box-shadow:var(--shadow),inset 0 0 0 1px rgb(255 214 160/.08)}\
.mark{display:inline-grid;place-items:center;width:84px;height:84px;margin:0 auto 10px;\
background:var(--wood);border:1px solid var(--edge);border-radius:12px}\
.mark svg{width:56px;height:auto}\
.brand{margin:0 0 18px;font-size:.8rem;font-weight:700;letter-spacing:.14em;\
text-transform:uppercase;color:var(--accent)}\
.status{display:block;width:44px;height:44px;margin:0 auto 12px}\
.ok{color:var(--ok)}.warn{color:var(--bad)}\
h1{margin:0 0 10px;font:700 1.65rem/1.2 \"Segoe UI Variable Display\",\"Segoe UI\",system-ui,sans-serif;\
color:var(--title)}\
p{margin:0 0 12px}.dim{color:var(--dim)}\
.close{margin:20px 0 0;padding-top:18px;border-top:1px solid var(--edge);font-weight:600}\
code{padding:2px 7px;border-radius:5px;background:var(--chip);\
font:.9em ui-monospace,\"Cascadia Mono\",Consolas,monospace}";

/// The mark from web/public/mark.svg, without its comment and label (the
/// brand name is written next to it).
const MARK: &str = "<svg viewBox=\"0 0 98 91\" aria-hidden=\"true\" focusable=\"false\">\
<path fill=\"#ffb24a\" d=\"M1.2 0L53 0L54.2 18L47.1 18L47.1 16.2C46.6 14.2 44.1 11.7 42.1 11.7L35.2 11.7Q33.7 11.7 33.7 13.2L33.7 64A1.9 1.9 0 0 1 31.8 65.9A3.6 3.6 0 0 0 28.2 69.5L28.2 75.7L19.27 75.7L20.91 13.2Q20.95 11.7 19.45 11.7L12.1 11.7C10.1 11.7 7.6 14.2 7.1 16.2L7.1 18L0 18Z\"/>\
<path fill=\"#ffd08a\" d=\"M12.59 15.3L17.26 15.3L15.57 79.3L73.25 79.3C76.2 79.3 78.25 81.3 78.25 84.3L78.25 91L4.25 91L4.25 85.5C4.25 82.4 8.51 80.5 8.71 77.5ZM37.3 15.3L40.72 15.3L43.51 17.81A3.6 3.6 0 0 0 47.1 21.6L49.7 21.6L49.7 63.3L60.8 63.3Q65.3 63.3 65.3 58.8L65.3 57.2L72.65 57.2L73.88 75.7L31.8 75.7L31.8 69.5A5.5 5.5 0 0 0 37.3 64ZM56.6 0L72.25 0L72.25 6.3Q69.7 6.6 69.5 10L72.41 53.6L64.67 53.6C62.67 53.6 60.96 54.95 60.36 56.35ZM73.23 12L79.5 12C89.5 12 95.04 17.5 95.79 27L97.88 52C97.97 64.5 93 72 82 72L77.23 72L76.55 61.7L79 61.7C85.2 61.7 87.22 59.5 87.22 53.5L85.05 27.5C85.05 24.2 84.05 22.7 81.85 22.7L73.95 22.7Z\"/>\
</svg>";

const CHECK: &str = "<svg class=\"status ok\" viewBox=\"0 0 44 44\" aria-hidden=\"true\" \
focusable=\"false\"><circle cx=\"22\" cy=\"22\" r=\"20\" fill=\"none\" stroke=\"currentColor\" \
stroke-width=\"3\"/><path d=\"M13 23l6 6 12-13\" fill=\"none\" stroke=\"currentColor\" \
stroke-width=\"3.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg>";

const WARN: &str = "<svg class=\"status warn\" viewBox=\"0 0 44 44\" aria-hidden=\"true\" \
focusable=\"false\"><circle cx=\"22\" cy=\"22\" r=\"20\" fill=\"none\" stroke=\"currentColor\" \
stroke-width=\"3\"/><path d=\"M22 12v13\" fill=\"none\" stroke=\"currentColor\" \
stroke-width=\"3.5\" stroke-linecap=\"round\"/><circle cx=\"22\" cy=\"31.5\" r=\"2.2\" \
fill=\"currentColor\"/></svg>";

/// The `style-src` source for the one style block.
pub fn style_hash() -> String {
    format!(
        "'sha256-{}'",
        STANDARD.encode(Sha256::digest(STYLE.as_bytes()))
    )
}

/// The policy sent with every page: nothing may load, only that style block.
pub fn content_security_policy() -> String {
    format!("default-src 'none'; style-src {}", style_hash())
}

pub fn render(page: Page, lang: Lang) -> String {
    let w = words(page, lang);
    let html_lang = match lang {
        Lang::En => "en",
        Lang::Es => "es",
    };
    let code = match page {
        Page::LinkFailed(code) if is_error_code(code) => format!(
            "<p class=\"dim\">{}: <code>{}</code></p>",
            code_label(lang),
            escape(code)
        ),
        _ => String::new(),
    };
    format!(
        "<!doctype html><html lang=\"{html_lang}\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<meta name=\"color-scheme\" content=\"dark light\"><title>{title} · Tavern Ledger</title>\
<style>{STYLE}</style></head><body><main class=\"card\">\
<div class=\"mark\">{MARK}</div><p class=\"brand\">Tavern Ledger</p>{status}\
<h1>{title}</h1><p>{text}</p>{code}<p class=\"close\">{close}</p></main></body></html>",
        title = escape(w.title),
        text = escape(w.text),
        status = if w.ok { CHECK } else { WARN },
        close = close_line(lang),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Page<'static>; 5] = [
        Page::SignedIn,
        Page::LinkFailed("flow_state_not_found"),
        Page::Expired,
        Page::OldLink,
        Page::NotFound,
    ];

    #[test]
    fn every_page_says_to_close_the_tab_in_its_language() {
        for page in ALL {
            assert!(render(page, Lang::En)
                .contains("You can close this tab and go back to Tavern Ledger."));
            assert!(render(page, Lang::Es)
                .contains("Ya puedes cerrar esta pestaña y volver a Tavern Ledger."));
        }
    }

    #[test]
    fn the_signed_in_page_says_so_in_each_language() {
        let en = render(Page::SignedIn, Lang::En);
        assert!(en.contains("<html lang=\"en\">") && en.contains("You are signed in"));
        let es = render(Page::SignedIn, Lang::Es);
        assert!(es.contains("<html lang=\"es\">") && es.contains("Sesión iniciada"));
    }

    #[test]
    fn each_failure_page_has_its_own_words() {
        let failed = render(Page::LinkFailed("x"), Lang::En);
        assert!(failed.contains("This link did not work"));
        assert!(render(Page::Expired, Lang::En).contains("This link has expired"));
        assert!(render(Page::OldLink, Lang::En).contains("earlier attempt"));
        assert!(render(Page::NotFound, Lang::Es).contains("Página no encontrada"));
        assert!(render(Page::Expired, Lang::Es).contains("ha caducado"));
    }

    #[test]
    fn a_page_is_self_contained() {
        for page in ALL {
            for lang in [Lang::En, Lang::Es] {
                let html = render(page, lang);
                assert!(!html.contains("<script"), "{page:?}");
                assert!(!html.contains("http"), "{page:?}: no URL of any kind");
                assert!(!html.contains("<link") && !html.contains("<img"));
                assert!(!html.contains("url("), "no fetched font or image");
                assert!(!html.contains("@import") && !html.contains("@font-face"));
                assert_eq!(html.matches("<style>").count(), 1, "one style block");
                assert!(html.contains("<svg"), "the mark is inline");
                assert!(html.contains("prefers-color-scheme:light"));
            }
        }
    }

    #[test]
    fn the_policy_allows_only_the_one_style_block_by_hash() {
        let csp = content_security_policy();
        assert_eq!(
            csp,
            format!("default-src 'none'; style-src {}", style_hash())
        );
        // The hash is of the exact text between <style> and </style>.
        let html = render(Page::SignedIn, Lang::En);
        let inner = html
            .split_once("<style>")
            .and_then(|(_, rest)| rest.split_once("</style>"))
            .unwrap()
            .0;
        assert_eq!(inner, STYLE);
        assert!(!csp.contains("unsafe") && !csp.contains("script-src"));
    }

    #[test]
    fn the_style_hash_is_pinned() {
        // Changing the style changes this on purpose: update it with the style.
        assert_eq!(style_hash(), PINNED_HASH);
    }

    const PINNED_HASH: &str = "'sha256-rskrUDO3/NITguBrQZ5aVJVQsnxwYZv3PojU3XvOUgE='";

    #[test]
    fn only_a_plain_error_code_is_shown_and_it_is_escaped() {
        let shown = render(Page::LinkFailed("flow_state_not_found"), Lang::En);
        assert!(shown.contains("<code>flow_state_not_found</code>"));
        let es = render(Page::LinkFailed("otp_x"), Lang::Es);
        assert!(es.contains("Código de error"));
        for bad in ["<script>alert(1)</script>", "a b", "", "é", &"a".repeat(65)] {
            let html = render(Page::LinkFailed(bad), Lang::En);
            assert!(!html.contains("<code>"), "{bad:?}");
            assert!(!html.contains("alert"), "{bad:?}");
        }
        assert_eq!(
            escape("<a href=\"x\">&'"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }

    #[test]
    fn the_code_check_matches_the_listener() {
        assert!(is_error_code("otp_expired"));
        assert!(is_error_code(&"a".repeat(64)));
        assert!(!is_error_code(&"a".repeat(65)));
        assert!(!is_error_code("a-b"));
    }
}
