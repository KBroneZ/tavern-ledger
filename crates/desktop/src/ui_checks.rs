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
