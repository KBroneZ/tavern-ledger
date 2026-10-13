//! Tests of what the error log keeps out (T-580).

use std::path::Path;

use super::*;
use crate::fake_secrets as fake;

fn dirs() -> Dirs {
    Dirs::new(
        Some(r"C:\Users\jane"),
        Some(r"C:\Users\jane\AppData\Roaming"),
        Some(r"C:\Users\jane\AppData\Local"),
    )
}

fn clean(text: &str) -> String {
    redact_with(text, &dirs())
}

// --- paths ---

#[test]
fn the_app_data_folder_is_named_by_its_variable() {
    assert_eq!(
        clean(r"cannot open C:\Users\jane\AppData\Roaming\TavernLedger\games.jsonl"),
        r"cannot open %APPDATA%\TavernLedger\games.jsonl"
    );
}

#[test]
fn the_profile_folder_is_named_by_its_variable() {
    assert_eq!(
        clean(r"read C:\Users\jane\Downloads\report.json"),
        r"read %USERPROFILE%\Downloads\report.json"
    );
    assert_eq!(
        clean(r"C:\Users\jane\AppData\Local\Blizzard"),
        r"%LOCALAPPDATA%\Blizzard"
    );
}

#[test]
fn a_path_matches_in_any_case_and_with_either_slash() {
    assert_eq!(
        clean("c:/users/JANE/AppData/Roaming/TavernLedger"),
        "%APPDATA%/TavernLedger"
    );
}

#[test]
fn another_user_folder_is_cut_even_when_the_environment_does_not_know_it() {
    assert_eq!(
        redact_with(r"E:\Users\Bob Smith\x", &Dirs::default()),
        r"%USERPROFILE%\x"
    );
    assert_eq!(
        redact_with(r"D:\Users\bob\Desktop\a.txt", &Dirs::default()),
        r"%USERPROFILE%\Desktop\a.txt"
    );
}

#[test]
fn a_folder_that_only_starts_like_the_profile_is_not_cut_as_the_profile() {
    let text = r"C:\Users\janet\x";
    let out = clean(text);
    assert!(!out.contains("janet"), "{out}");
    assert!(out.starts_with("%USERPROFILE%"), "{out}");
}

#[test]
fn folders_outside_the_profile_stay() {
    let text = r"D:\Games\Hearthstone\Logs";
    assert_eq!(clean(text), text);
}

// --- names and ids ---

#[test]
fn a_battletag_is_cut() {
    for tag in [
        "Jaina#1234",
        "Ünïcode名前#98765",
        "Name＃1234",
        "a_b#123456",
    ] {
        let out = clean(&format!("player {tag} left"));
        assert_eq!(out, "player [battletag] left", "{tag}");
    }
}

#[test]
fn account_ids_are_cut() {
    let out = clean("GameAccountId=[hi=144115193835963207 lo=987654321] seen");
    assert!(!out.contains("14411"), "{out}");
    assert!(!out.contains("987654"), "{out}");
    let out = clean("entity [hi=144115193835963207 lo=987654321]");
    assert!(!out.contains("14411"), "{out}");
    assert!(!out.contains("987654"), "{out}");
}

#[test]
fn an_email_is_cut() {
    assert_eq!(
        clean("sign-in for jane.doe+tl@example.com failed"),
        "sign-in for [email] failed"
    );
}

// --- secrets ---

#[test]
fn tokens_and_keys_are_cut() {
    let jwt = fake::jwt();
    let word = fake::word();
    let cases = [
        format!("header Authorization: Bearer {jwt}"),
        format!("jwt {jwt}"),
        format!("key {}", fake::prefixed_key()),
        format!("access_token={word}"),
        format!(r#"{{"refresh_token":"{word}"}}"#),
        format!("password: {}", fake::password()),
        format!("sha {}", "ab12".repeat(16)),
    ];
    for case in cases {
        let out = clean(&case);
        let kept = ["eyJ", "AbCdEf", &fake::word_start(), "hunter2", "ab12ab12"];
        for secret in kept {
            assert!(!out.contains(secret), "{case} -> {out}");
        }
    }
}

#[test]
fn plain_words_ids_and_session_names_stay() {
    for text in [
        "game 3 saved (status ok) in Hearthstone_2026_10_09_15_26_25",
        "upload: 2 games uploaded, 1 refused by the server (game_too_old)",
        "BG20_HERO_202 and TB_BaconShop_HERO_PH",
        "the server had an error (503)",
        "the tracker stopped; RebuildHistoryFromTheOldestRecordedLogLines",
        "no token available",
    ] {
        assert_eq!(clean(text), text);
    }
}

// --- one line ---

#[test]
fn a_message_is_one_trimmed_line() {
    assert_eq!(one_line("a\nb\r\n\tc", 100), "a b   c");
    let long = "x".repeat(50);
    let cut = one_line(&long, 10);
    assert_eq!(cut.chars().count(), 10);
    assert!(cut.ends_with('…'));
    assert_eq!(one_line("short", 10), "short");
}

// --- what the first review found ---

#[test]
fn account_ids_in_json_and_debug_form_are_cut() {
    for text in [
        r#"{"GameAccountId":{"hi":144115193835963207,"lo":987654321}}"#,
        "GameAccountId { hi: 144115193835963207, lo: 987654321 }",
        "game_account_id: 987654321",
        "AccountHi=144115193835963207 AccountLo=987654321",
        r#"{"hi" : 144115193835963207, "lo":987654321}"#,
    ] {
        let out = clean(text);
        assert!(
            !out.contains("14411") && !out.contains("987654"),
            "{text} -> {out}"
        );
    }
}

#[test]
fn secrets_in_escaped_json_prefixed_names_and_basic_auth_are_cut() {
    let (word, basic, start) = (fake::word(), fake::basic(), fake::word_start());
    let escaped = format!(r#"body {{\"token\":\"{word}\"}}"#);
    let texts = [
        escaped,
        format!("session_token={word}"),
        format!("client_secret: {word}"),
        format!("Authorization: Basic {basic}"),
        r#"{"password":"my long pass phrase"}"#.to_string(),
        "email=jane%40example.com&x=1".to_string(),
    ];
    for text in texts {
        let out = clean(&text);
        let kept = [
            start.as_str(),
            basic.as_str(),
            "long pass",
            "phrase",
            "jane",
            "example.com",
        ];
        for secret in kept {
            assert!(!out.contains(secret), "{text} -> {out}");
        }
    }
}

#[test]
fn base64_tokens_with_dashes_and_slashes_are_cut_but_names_stay() {
    let token = fake::base64url();
    assert!(!clean(&format!("token seen {token} here")).contains("Ab3"));
    for text in [
        "Hearthstone_2026_10_09_15_26_25-game1",
        "tavern-ledger-update-check-failed-while-reading-the-answer",
    ] {
        assert_eq!(clean(text), text);
    }
}

#[test]
fn emails_with_accents_are_cut() {
    assert_eq!(clean("for josé.ñandú@correo.es now"), "for [email] now");
}

#[test]
fn a_network_path_loses_its_computer_name() {
    let out = clean(r"cannot open \\DESKTOP-ANA\share\games.jsonl");
    assert!(!out.contains("ANA"), "{out}");
}

#[test]
fn the_user_and_computer_names_are_cut_wherever_they_stand_alone() {
    let dirs = Dirs::default().with_words(&[
        Some("andia".into()),
        Some("PC-ANA".into()),
        Some("ab".into()),
    ]);
    assert_eq!(
        redact_with("for Andia on pc-ana (ab) andiamo", &dirs),
        "for [name] on [name] (ab) andiamo"
    );
}

#[test]
fn a_folder_outside_the_users_is_shortened_to_its_last_two_parts() {
    let short = place(Path::new(r"D:\Juegos\Ana\Hearthstone\Logs"));
    assert_eq!(short, r"…\Hearthstone\Logs");
    assert!(!short.contains("Ana"));
    assert_eq!(place(Path::new(r"D:\Logs")), r"D:\Logs");
}

#[test]
fn text_that_reads_backwards_or_hides_is_cleaned() {
    assert_eq!(one_line("a\u{202e}b\u{200b}c\u{2028}d", 100), "abcd");
}
