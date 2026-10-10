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
const OVERLAY_CSS_FILE: &str = include_str!("../ui/overlay.css");
const LAYOUT: &str = include_str!("../ui/overlay-layout.js");

/// The stylesheet with LF line ends: a Windows checkout with
/// `core.autocrlf=true` has CRLF in the working tree.
fn overlay_css() -> String {
    OVERLAY_CSS_FILE.replace("\r\n", "\n")
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
    let start = position(INDEX, "<main>");
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
    let css = overlay_css();
    let rule = position(&css, ".toolbar button,\n.arrange-pad button {");
    let end = rule + css[rule..].find('}').unwrap();
    let body = &css[rule..end];
    assert!(body.contains("min-width: 28px;") && body.contains("min-height: 28px;"));
}

#[test]
fn the_overlay_controls_are_placed_away_from_the_panels() {
    // Not pinned to the top and bottom centre any more (T-319).
    let css = overlay_css();
    for rule in [".toolbar {", ".key-hint {"] {
        let at = position(&css, rule);
        let end = at + css[at..].find('}').unwrap();
        assert!(!css[at..end].contains("translateX"), "{rule}");
    }
    assert!(LAYOUT.contains("TLPlace.spot(") && LAYOUT.contains("TLPlace.padSpot("));
}

#[test]
fn the_focus_ring_has_two_colours() {
    // A light ring with a dark one around it (the contrast is tested in overlay_layout.rs).
    let css = overlay_css();
    for ring in [
        "outline: 3px solid var(--accent);\n  outline-offset: 4px;\n  box-shadow: 0 0 0 9px var(--panel-solid);",
        "outline: 2px solid var(--accent);\n  outline-offset: 1px;\n  box-shadow: 0 0 0 5px var(--panel-solid);",
    ] {
        assert!(css.contains(ring), "{ring}");
    }
}
