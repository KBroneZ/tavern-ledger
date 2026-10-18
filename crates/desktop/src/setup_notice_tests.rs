//! Tests of the setup notice and the Fix button's two files, in temp folders
//! only: never the player's own `log.config` or `client.config`.

use super::*;
use std::fs;
use std::sync::atomic::AtomicU32;

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tl-setup-notice-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const POWER_ONLY: &str = "[Power]\nLogLevel=1\nFilePrinting=true\nVerbose=true\n";
const GOOD_POWER: &str = "[Power]\nLogLevel=1\nFilePrinting=true\nVerbose=true\n[LoadingScreen]\nLogLevel=1\nFilePrinting=true\n";
const GOOD_CLIENT: &str = "[Log]\nFileSizeLimit.Int=-1\n";

fn install(root: &Path, name: &str, client: Option<&str>) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("Logs")).unwrap();
    fs::write(dir.join("Hearthstone.exe"), b"").unwrap();
    if let Some(text) = client {
        fs::write(dir.join("client.config"), text).unwrap();
    }
    dir
}

fn local_dir(root: &Path, log_config: Option<&str>) -> PathBuf {
    let local = root.join("local");
    fs::create_dir_all(&local).unwrap();
    if let Some(text) = log_config {
        let path = log_config_target(&local);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    local
}

fn state_for(local_dir: &Path, install_dir: Option<&Path>) -> SetupState {
    let state = SetupState::default();
    state.use_local_app_data(Some(local_dir.to_path_buf()));
    state.set_place(Place {
        install_dir: install_dir.map(Path::to_path_buf),
        folder_lines: Vec::new(),
    });
    state
}

#[test]
fn choosing_a_good_folder_clears_the_notice_and_going_back_brings_it_again() {
    let root = temp_dir();
    let local = local_dir(&root, Some(GOOD_POWER));
    let automatic = install(&root, "E", None);
    let chosen = install(&root, "D", Some(GOOD_CLIENT));
    let state = state_for(&local, Some(&automatic));

    let (lines, view) = state.view();
    assert_eq!(view.state, "needs_fix");
    assert!(view.can_fix);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains(&automatic.display().to_string()));

    // The player chooses the other install; the next view is for that one.
    state.set_place(Place {
        install_dir: Some(chosen.clone()),
        folder_lines: Vec::new(),
    });
    let (lines, view) = state.view();
    assert_eq!(view.state, "ok");
    assert!(!view.can_fix);
    assert!(lines.is_empty());

    state.set_place(Place {
        install_dir: Some(automatic),
        folder_lines: Vec::new(),
    });
    assert_eq!(state.view().1.state, "needs_fix");
}

#[test]
fn a_player_who_fixes_the_files_by_hand_clears_the_next_view() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", Some("[Log]\nFileSizeLimit.Int=10000\n"));
    let state = state_for(&local, Some(&game));
    assert_eq!(state.view().1.state, "needs_fix");
    fs::write(game.join("client.config"), GOOD_CLIENT).unwrap();
    fs::create_dir_all(log_config_target(&local).parent().unwrap()).unwrap();
    fs::write(log_config_target(&local), GOOD_POWER).unwrap();
    let (lines, view) = state.view();
    assert_eq!((view.state, lines.len()), ("ok", 0));
}

#[test]
fn a_check_that_cannot_run_is_never_shown_as_ok() {
    let root = temp_dir();
    let local = local_dir(&root, Some(GOOD_POWER));
    // No folder in use, and the notice has no folder lines yet.
    let state = state_for(&local, None);
    let (lines, view) = state.view();
    assert_eq!(view.state, "cannot_check");
    assert!(!view.can_fix);
    assert!(lines[0].starts_with("client.config could not be checked"));

    // A folder problem: the lines say so and nothing is checked or fixable.
    state.set_place(Place {
        install_dir: None,
        folder_lines: vec!["no folder".into()],
    });
    let (lines, view) = state.view();
    assert_eq!(lines, ["no folder"]);
    assert_eq!((view.state, view.can_fix), ("cannot_check", false));
}

#[test]
fn only_the_optional_lines_missing_is_ok_with_an_offer_to_add_them() {
    let root = temp_dir();
    let local = local_dir(&root, Some(POWER_ONLY));
    let game = install(&root, "D", Some(GOOD_CLIENT));
    let (lines, view) = state_for(&local, Some(&game)).view();
    assert_eq!(view.state, "ok");
    assert!(!view.can_fix);
    assert!(view.can_fix_optional);
    assert!(lines[0].starts_with("Optional:"));
}

#[test]
fn the_view_never_writes_anything() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", None);
    let state = state_for(&local, Some(&game));
    let _ = state.view();
    let _ = state.report();
    assert!(!game.join("client.config").exists());
    assert!(!log_config_target(&local).exists());
    assert!(!local.join("Blizzard").exists());
}

#[test]
fn the_fix_creates_both_files_and_the_check_is_then_ok() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", None);
    let files = fix_files(Some(&local), Some(&game), false);
    assert_eq!(
        files,
        vec![
            FileFix {
                file: "log.config",
                result: "created",
                reason: None,
                backup: false
            },
            FileFix {
                file: "client.config",
                result: "created",
                reason: None,
                backup: false
            },
        ]
    );
    let state = state_for(&local, Some(&game));
    assert_eq!(state.view().1.state, "ok");
    let log = fs::read_to_string(log_config_target(&local)).unwrap();
    assert!(
        !log.contains("LoadingScreen"),
        "the optional lines are not added unasked"
    );
}

#[test]
fn the_optional_lines_are_added_only_when_asked() {
    let root = temp_dir();
    let local = local_dir(&root, Some(POWER_ONLY));
    let game = install(&root, "D", Some(GOOD_CLIENT));
    let files = fix_files(Some(&local), Some(&game), true);
    assert_eq!(files[0].result, "fixed");
    assert!(files[0].backup);
    assert_eq!(files[1].result, "already_ok");
    let (lines, view) = state_for(&local, Some(&game)).view();
    assert!(lines.is_empty());
    assert!(!view.can_fix_optional);
}

#[test]
fn the_two_files_are_independent_when_one_fails_the_other_still_works() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", None);
    // client.config is a folder: refused; log.config is made all the same.
    fs::create_dir(game.join("client.config")).unwrap();
    let files = fix_files(Some(&local), Some(&game), false);
    assert_eq!(files[0].result, "created");
    assert_eq!(
        (files[1].result, files[1].reason),
        ("refused", Some(Reason::NotAFile))
    );
    assert!(game.join("client.config").is_dir());

    // And the other way round: log.config cannot be read as text.
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", None);
    let log = log_config_target(&local);
    fs::create_dir_all(log.parent().unwrap()).unwrap();
    fs::write(&log, [0xFF, 0xFE, b'[', 0]).unwrap();
    let files = fix_files(Some(&local), Some(&game), false);
    assert_eq!(files[0].reason, Some(Reason::NotPlainText));
    assert_eq!(files[1].result, "created");
    assert_eq!(fs::read(&log).unwrap(), [0xFF, 0xFE, b'[', 0]);
}

#[test]
fn an_unknown_or_wrong_folder_is_refused_and_the_install_folder_is_never_made() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let not_the_game = root.join("not-the-game");
    fs::create_dir_all(&not_the_game).unwrap();
    let files = fix_files(Some(&local), Some(&not_the_game), false);
    assert_eq!(files[1].reason, Some(Reason::NoGameExe));
    assert!(!not_the_game.join("client.config").exists());

    let gone = root.join("gone");
    let files = fix_files(Some(&local), Some(&gone), false);
    assert_eq!(files[1].reason, Some(Reason::NoGameExe));
    assert!(!gone.exists());

    let files = fix_files(None, None, false);
    assert_eq!(files[0].reason, Some(Reason::NoPath));
    assert_eq!(files[1].reason, Some(Reason::NoPath));
}

#[test]
fn only_one_fix_runs_at_a_time() {
    let state = SetupState::default();
    let first = state.begin_fix().expect("nothing is running");
    assert!(state.begin_fix().is_none());
    first.finish(FixView {
        id: state.next_fix_id(),
        files: Vec::new(),
        game_running: false,
    });
    assert!(state.begin_fix().is_some());
}

#[test]
fn the_claim_comes_back_even_when_the_click_never_finishes() {
    let state = SetupState::default();
    let ticket = state.begin_fix().expect("nothing is running");
    // A panic or a dropped future drops the ticket without finishing.
    drop(ticket);
    assert!(state.begin_fix().is_some());
}

#[test]
fn a_failed_click_for_the_optional_lines_stays_while_they_are_missing() {
    let root = temp_dir();
    let local = local_dir(&root, Some(POWER_ONLY));
    let game = install(&root, "D", Some(GOOD_CLIENT));
    let report = check_install(Some(&local), Some(&game));
    let failed = fix_view(vec![done("log.config", "failed", Some(Reason::ReadOnly))]);
    let (_, view) = compose(Some(&report), &[], false, Some(&failed));
    assert_eq!(view.state, "ok");
    assert!(view.can_fix_optional);
    assert_eq!(view.fix, Some(failed));
}

fn fix_view(files: Vec<FileFix>) -> FixView {
    FixView {
        id: 1,
        files,
        game_running: true,
    }
}

fn done(file: &'static str, result: &'static str, reason: Option<Reason>) -> FileFix {
    FileFix {
        file,
        result,
        reason,
        backup: false,
    }
}

#[test]
fn the_state_follows_the_click_and_what_is_still_wrong() {
    let root = temp_dir();
    let local = local_dir(&root, Some(GOOD_POWER));
    let game = install(&root, "D", None);
    let report = check_install(Some(&local), Some(&game));
    let failed = fix_view(vec![done(
        "client.config",
        "failed",
        Some(Reason::AccessDenied),
    )]);

    let state_of =
        |fixing, last: Option<&FixView>| compose(Some(&report), &[], fixing, last).1.state;
    assert_eq!(state_of(false, None), "needs_fix");
    assert_eq!(state_of(true, None), "fixing");
    assert_eq!(state_of(false, Some(&failed)), "error");

    // Fixed by hand afterwards: the failure is not shown again.
    fs::write(game.join("client.config"), GOOD_CLIENT).unwrap();
    let report = check_install(Some(&local), Some(&game));
    let (lines, view) = compose(Some(&report), &[], false, Some(&failed));
    assert_eq!(view.state, "ok");
    assert!(lines.iter().all(|l| l.starts_with("Optional:")));
    assert_eq!(view.fix, None);
}

#[test]
fn a_fix_that_worked_stays_to_say_the_game_must_be_restarted() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", None);
    let files = fix_files(Some(&local), Some(&game), false);
    let report = check_install(Some(&local), Some(&game));
    let (_, view) = compose(Some(&report), &[], false, Some(&fix_view(files)));
    assert_eq!(view.state, "ok");
    let shown = view.fix.expect("the result is still there");
    assert!(shown.game_running);
    assert!(shown.files.iter().all(|f| f.worked()));
}

#[test]
fn the_dev_stand_in_for_local_app_data_wins_over_the_real_one() {
    let state = SetupState::default();
    let dir = temp_dir();
    state.use_local_app_data(Some(dir.clone()));
    assert_eq!(state.local_app_data(), Some(dir));
}

#[test]
fn what_a_click_did_in_one_folder_is_not_shown_for_another() {
    let root = temp_dir();
    let local = local_dir(&root, Some(GOOD_POWER));
    let first = install(&root, "E", None);
    let second = install(&root, "D", None);
    let state = state_for(&local, Some(&first));
    let ticket = state.begin_fix().expect("nothing is running");
    ticket.finish(fix_view(vec![done(
        "client.config",
        "failed",
        Some(Reason::AccessDenied),
    )]));
    assert_eq!(state.view().1.state, "error");
    // The player chooses another install: a fresh start there.
    state.set_place(Place {
        install_dir: Some(second),
        folder_lines: Vec::new(),
    });
    let (_, view) = state.view();
    assert_eq!(view.state, "needs_fix");
    assert_eq!(view.fix, None);
}

#[test]
fn once_the_tracker_has_stopped_the_notice_offers_nothing_to_write() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", None);
    let state = state_for(&local, Some(&game));
    assert!(state.view().1.can_fix);
    state.freeze();
    let (lines, view) = state.view();
    assert_eq!(lines.len(), 2, "it still says what is wrong");
    assert_eq!(
        (view.state, view.can_fix, view.can_fix_optional),
        ("unknown", false, false)
    );
}

#[test]
fn a_write_windows_kept_in_the_packages_private_store_is_not_reported_as_created() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", Some(GOOD_CLIENT));
    let private = root.join("private-local");
    // What Windows leaves behind when it redirects the write: a copy at the
    // same place in the package's private store.
    let copy = private
        .join("Blizzard")
        .join("Hearthstone")
        .join("log.config");
    fs::create_dir_all(copy.parent().unwrap()).unwrap();
    fs::write(&copy, POWER_ONLY).unwrap();

    let files = fix_files_in(Some(&local), Some(&game), false, Some(&private));
    assert_eq!(
        (files[0].result, files[0].reason),
        ("failed", Some(Reason::Virtualized)),
        "never 'created' for a file the game cannot see"
    );
    assert_eq!(files[1].result, "already_ok");
}

#[test]
fn a_write_with_no_copy_in_the_private_store_is_reported_as_it_is() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", Some(GOOD_CLIENT));
    let private = root.join("private-local");
    fs::create_dir_all(&private).unwrap();
    let files = fix_files_in(Some(&local), Some(&game), false, Some(&private));
    assert_eq!(files[0].result, "created");
    // And an app that is not packaged has no private store to look in.
    let again = fix_files_in(Some(&local), Some(&game), false, None);
    assert_eq!(again[0].result, "already_ok");
}

#[test]
fn a_file_that_needed_no_write_is_not_judged_by_an_old_private_copy() {
    let root = temp_dir();
    let local = local_dir(&root, Some(GOOD_POWER));
    let game = install(&root, "D", Some(GOOD_CLIENT));
    let private = root.join("private-local");
    let copy = private
        .join("Blizzard")
        .join("Hearthstone")
        .join("log.config");
    fs::create_dir_all(copy.parent().unwrap()).unwrap();
    fs::write(&copy, "stale").unwrap();
    let files = fix_files_in(Some(&local), Some(&game), false, Some(&private));
    assert_eq!(files[0].result, "already_ok");
}

#[test]
fn where_the_app_may_not_write_the_box_lists_the_lines_and_offers_no_button() {
    let root = temp_dir();
    let local = local_dir(&root, None);
    let game = install(&root, "D", None);
    let state = state_for(&local, Some(&game));
    let (lines, view) = state.view();
    assert_eq!(view.state, "needs_fix");
    assert!(view.can_fix);
    state.set_manual_only(true);
    let (manual_lines, manual) = state.view();
    assert_eq!(manual_lines, lines, "the same lines say what to write");
    assert_eq!(manual.state, "manual");
    assert!(!manual.can_fix && !manual.can_fix_optional);
    assert!(!state.fix_allowed());
}

#[test]
fn manual_setup_also_covers_the_optional_lines_and_leaves_a_good_setup_alone() {
    let root = temp_dir();
    let local = local_dir(&root, Some(POWER_ONLY));
    let game = install(&root, "D", Some(GOOD_CLIENT));
    let state = state_for(&local, Some(&game));
    state.set_manual_only(true);
    let (lines, view) = state.view();
    assert!(!lines.is_empty(), "the optional lines are still told");
    assert!(!view.can_fix_optional);
    assert_eq!(view.state, "ok", "nothing needed is missing");

    let good = local_dir(&temp_dir(), Some(GOOD_POWER));
    let state = state_for(&good, Some(&game));
    state.set_manual_only(true);
    let (lines, view) = state.view();
    assert!(lines.is_empty());
    assert_eq!(view.state, "ok");
}
