//! Checks on the pages' markup and the browser-side scripts (T-317): the
//! accessibility fixes that no Rust code exercises. The scripts' own logic
//! (focus keeping, arrow-key arranging) is tested in `ui-tests/` with Node,
//! which this runs, so `cargo test` covers it.

use std::path::Path;
use std::process::Command;

const INDEX: &str = include_str!("../ui/index.html");
const OVERLAY: &str = include_str!("../ui/overlay.html");
const SWITCH: &str = include_str!("../ui/overlay-switch.js");
const APP: &str = include_str!("../ui/app.js");
const OVERLAY_CSS: &str = include_str!("../ui/overlay.css");
const LAYOUT: &str = include_str!("../ui/overlay-layout.js");
const ROUND_TWO: &str = include_str!("../ui/round-two.js");
const ROUND_TWO_CSS: &str = include_str!("../ui/round-two.css");
const STYLE: &str = include_str!("../ui/style.css");
const SHOP_CSS: &str = include_str!("../ui/shop.css");
const CURIOUS_CSS: &str = include_str!("../ui/curious.css");
const TURNS_CSS: &str = include_str!("../ui/turns.css");
const COMPACT_CSS: &str = include_str!("../ui/compact.css");
/// The main window's scripts that draw pictures or tables, besides app.js.
const WINDOW_SCRIPTS: [(&str, &str); 6] = [
    ("shop.js", include_str!("../ui/shop.js")),
    ("extras.js", include_str!("../ui/extras.js")),
    ("round-two.js", include_str!("../ui/round-two.js")),
    ("curious.js", include_str!("../ui/curious.js")),
    ("turns.js", include_str!("../ui/turns.js")),
    ("compact.js", include_str!("../ui/compact.js")),
];

/// The checked-in files are LF; a Windows checkout may turn them into CRLF.
fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// The body of the first CSS rule that starts with `selector`.
fn rule<'a>(css: &'a str, selector: &str) -> &'a str {
    let at = position(css, selector);
    let end = at + css[at..].find('}').unwrap();
    &css[at..end]
}

fn position(page: &str, needle: &str) -> usize {
    page.find(needle)
        .unwrap_or_else(|| panic!("{needle} is not in the page"))
}

#[test]
fn the_browser_side_tests_pass() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new("node")
        .current_dir(dir)
        .args(["--test", "ui-tests/*.test.js"])
        .output()
        .expect("Node is needed to run the tests in ui-tests/");
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn everything_but_the_masthead_and_footer_is_inside_main() {
    let start = position(INDEX, "<main id=\"home\">");
    let end = position(INDEX, "</main>");
    for id in ["id=\"setup\"", "id=\"overlay-switch\"", "id=\"upload\""] {
        let at = position(INDEX, id);
        assert!(start < at && at < end, "{id} is outside <main>");
    }
}

#[test]
fn a_toolbar_role_is_not_claimed_without_arrow_keys_between_its_buttons() {
    assert!(!OVERLAY.contains("role=\"toolbar\""));
    assert!(!INDEX.contains("role=\"toolbar\""));
}

#[test]
fn the_arrange_button_has_one_signal_not_two() {
    // Its text changes between "Arrange overlay" and "Lock overlay".
    assert!(SWITCH.contains("Lock overlay") && SWITCH.contains("Arrange overlay"));
    assert!(!SWITCH.contains("aria-pressed"));
    assert!(!INDEX.contains("id=\"overlay-arrange\" aria-pressed"));
}

#[test]
fn the_overlay_can_be_arranged_by_keyboard() {
    for panel in ["status", "tribes", "opponents", "legend"] {
        assert!(OVERLAY.contains(&format!("data-focus=\"slot:{panel}\"")));
    }
    assert!(OVERLAY.contains("id=\"key-hint\""), "the hint for the keys");
    let keys = position(OVERLAY, "overlay-keys.js");
    let layout = position(OVERLAY, "overlay-layout.js");
    assert!(keys < layout, "the layout script uses the key rules");
    let focus = position(OVERLAY, "focus-keeper.js");
    assert!(focus < layout, "and keeps focus through its redraws");
}

#[test]
fn the_main_window_keeps_focus_through_its_redraws() {
    let keeper = position(INDEX, "focus-keeper.js");
    assert!(keeper < position(INDEX, "app.js"));
    assert!(APP.contains("TLFocus.keep(document, renderNow)"));
    assert!(APP.contains("TLFocus.keep(document, renderLobbyNow)"));
    assert!(APP.contains("TLFocus.keep(document, renderTribesDialogNow)"));
}

#[test]
fn round_two_details_are_folded_keyboard_controls_with_words_not_colour() {
    // Loaded before the window's script, with its own stylesheet.
    assert!(position(INDEX, "round-two.js") < position(INDEX, "app.js"));
    assert!(INDEX.contains("href=\"round-two.css\""));
    assert!(INDEX.contains("id=\"choice-stats-body\""));
    // Native <details>: Enter and Space open it; closed until the user opens
    // it, and its summary is found again after a redraw.
    assert!(ROUND_TWO.contains("ui.el(\"details\""));
    assert!(ROUND_TWO.contains("box.open = openFolds.has(key)"));
    assert!(ROUND_TWO.contains("data-focus\", `fold:${key}`"));
    // Keyword and pick marks are words the screen reader reads, not colours.
    for word in ["\"Taunt\"", "\"Divine Shield\"", "\"Golden\"", "\"taken\""] {
        assert!(ROUND_TWO.contains(word), "{word} is shown as a word");
    }
    assert!(!ROUND_TWO_CSS.contains("display: none"));
}

#[test]
fn the_health_chart_prints_its_values() {
    // Round and health as text in the drawing, not only in the aria-label.
    assert!(APP.contains("\"round-label\""));
    assert!(APP.contains("\"value-label\""));
}

#[test]
fn the_overlay_can_be_arranged_with_single_clicks() {
    // WCAG 2.5.7: buttons next to the selected panel, made from the key rules.
    assert!(OVERLAY.contains("id=\"arrange-pad\""));
    assert!(OVERLAY.contains("id=\"pad-big\" aria-pressed=\"false\""));
    assert!(LAYOUT.contains("TLKeys.PAD") && LAYOUT.contains("TLKeys.press(action, bigSteps)"));
    // Saved like keys: the same delayed save.
    assert!(LAYOUT.contains("queueKeySave(id,") && LAYOUT.contains("queueKeySave(\"box\","));
    // Hidden while locked: no target then.
    assert!(LAYOUT.contains("const target = unlocked ? selectedRect() : null;"));
    assert!(LAYOUT.contains("pad.hidden = !target;"));
    let place = position(OVERLAY, "overlay-place.js");
    assert!(place < position(OVERLAY, "overlay-layout.js"));
}

#[test]
fn the_overlay_buttons_are_at_least_24_pixels() {
    // WCAG 2.5.8: the toolbar and the move-and-size buttons share one rule.
    let css = lf(OVERLAY_CSS);
    let body = rule(&css, ".toolbar button,\n.arrange-pad button {");
    assert!(body.contains("min-width: 28px;") && body.contains("min-height: 28px;"));
}

#[test]
fn the_overlay_controls_are_placed_away_from_the_panels() {
    // Not pinned to the top and bottom centre any more (T-319).
    for rule in [".toolbar {", ".key-hint {"] {
        let at = position(OVERLAY_CSS, rule);
        let end = at + OVERLAY_CSS[at..].find('}').unwrap();
        assert!(!OVERLAY_CSS[at..end].contains("translateX"), "{rule}");
    }
    assert!(LAYOUT.contains("TLPlace.spot(") && LAYOUT.contains("TLPlace.padSpot("));
}

#[test]
fn the_focus_ring_has_two_colours() {
    // A light ring with a dark one around it (the contrast is tested in overlay_layout.rs).
    for ring in [
        "outline: 3px solid var(--accent);\n  outline-offset: 4px;\n  box-shadow: 0 0 0 9px var(--panel-solid);",
        "outline: 2px solid var(--accent);\n  outline-offset: 1px;\n  box-shadow: 0 0 0 5px var(--panel-solid);",
    ] {
        assert!(lf(OVERLAY_CSS).contains(ring), "{ring}");
    }
}

#[test]
fn card_pictures_are_cropped_to_the_art_with_no_white_edges() {
    // T-320: HearthstoneJSON's 256x art pads a tall picture with white bars,
    // so cover alone keeps white in the shape. One shared rule crops the
    // source (object-view-box), fills the shape and puts a dark background
    // under it; no other stylesheet changes the shape or the fit.
    let style = lf(STYLE);
    let shared = rule(
        &style,
        ".portrait, .recap-art, .shop-art, .face, .curious-art {",
    );
    for needed in [
        "border-radius: 50%;",
        "object-fit: cover;",
        "background: var(--panel-solid);",
    ] {
        assert!(shared.contains(needed), "{needed}");
    }
    // At the start of a line: the shared rule above names them too.
    let heroes = rule(&style, "\n.portrait, .recap-art {");
    assert!(heroes.contains("object-view-box: inset("));
    let cards = rule(&style, "\n.shop-art, .face, .curious-art {");
    assert!(cards.contains("object-view-box: inset(1% 16%"));
    for (name, css) in [
        ("shop.css", SHOP_CSS),
        ("curious.css", CURIOUS_CSS),
        ("turns.css", TURNS_CSS),
    ] {
        for art in [".shop-art {", ".curious-art {", ".face {"] {
            if let Some(at) = css.find(art) {
                let end = at + css[at..].find('}').unwrap();
                for banned in ["border-radius", "object-fit", "background"] {
                    assert!(!css[at..end].contains(banned), "{name} {art} sets {banned}");
                }
            }
        }
    }
    for css in [
        STYLE,
        SHOP_CSS,
        CURIOUS_CSS,
        TURNS_CSS,
        COMPACT_CSS,
        ROUND_TWO_CSS,
    ] {
        let css = css.to_lowercase();
        for banned in [
            "object-fit: contain",
            "background: #fff",
            "background: white",
        ] {
            assert!(!css.contains(banned), "{banned}");
        }
    }
}

#[test]
fn every_picture_in_the_window_loads_lazily_with_a_placeholder() {
    // One helper in app.js asks cards.js for pictures; the other scripts get
    // it through `ui.art`, so no picture skips the lazy load or the placeholder.
    assert!(APP.contains("const ART_OPTIONS = { lazy: true, placeholder: true };"));
    assert_eq!(APP.matches("TLCards.art(").count(), 1);
    for (name, script) in WINDOW_SCRIPTS {
        assert!(
            !script.contains("TLCards.art("),
            "{name} draws a picture without ui.art"
        );
    }
    assert!(STYLE.contains(".art-missing {"));
}

#[test]
fn the_recap_fills_the_window_and_closes_with_x_or_escape() {
    // T-322: a view in the page, not a dialog and not a fold.
    assert!(!INDEX.contains("id=\"recap-dialog\""));
    assert!(INDEX.contains(
        "<section id=\"recap-view\" class=\"recap recap-view\" aria-labelledby=\"recap-title\" hidden>"
    ));
    assert!(INDEX.contains("<h2 id=\"recap-title\" tabindex=\"-1\">"));
    assert!(INDEX.contains("id=\"recap-close\" class=\"close-x\" aria-label=\"Close the recap\""));
    assert!(position(INDEX, "recap-view.js") < position(INDEX, "app.js"));
    assert!(APP.contains("TLRecapView.create("));
    assert!(APP.contains(
        "hide: [document.getElementById(\"masthead\"), document.getElementById(\"home\")]"
    ));
    // `hidden` must win over the layout's own display values.
    assert!(STYLE.contains("[hidden] { display: none !important; }"));
}

#[test]
fn there_is_no_wide_native_scrollbar_but_a_cue_of_more() {
    assert!(
        STYLE.contains("html { scrollbar-width: thin; scrollbar-color: transparent transparent; }")
    );
    assert!(STYLE.contains("html.scroll-show {"));
    assert!(STYLE.contains("html[data-more-below] body::after { opacity: 1; }"));
    // Windows contrast themes keep the system scrollbar.
    assert!(STYLE.contains("html, html.scroll-show { scrollbar-color: auto; }"));
    assert!(position(INDEX, "scroll-cue.js") < position(INDEX, "app.js"));
    assert!(APP.contains("TLScrollCue.start(window, document);"));
}

#[test]
fn long_lists_share_one_compact_helper() {
    assert!(position(INDEX, "compact.js") < position(INDEX, "app.js"));
    assert!(INDEX.contains("href=\"compact.css\""));
    for table in [
        "key: `ledger:${mode}`",
        "key: `heroes:${mode}`",
        "key: `tribes:${mode}`",
        "key: `entered:${mode}`",
    ] {
        assert!(APP.contains(table), "{table}");
    }
    for (name, script) in WINDOW_SCRIPTS.iter().take(3) {
        assert!(
            script.contains("TLCompact.table(") || script.contains("C.table("),
            "{name}"
        );
    }
    // The button says how many rows it opens and what it controls.
    let compact = WINDOW_SCRIPTS[5].1;
    assert!(compact.contains("Show all (${all.length})"));
    assert!(compact.contains("button.setAttribute(\"aria-expanded\""));
    assert!(compact.contains("th.setAttribute(\"aria-sort\""));
}
