//! Tests of `setup::merge_config`: what the Fix button writes into the text of
//! `log.config` and `client.config` (D-154).

use super::*;

const POWER_LINES: &str = "LogLevel=1\r\nFilePrinting=true\r\nVerbose=true\r\n";

fn merged(text: &str, wanted: &[Wanted]) -> String {
    let out = merge_config(text.as_bytes(), wanted)
        .expect("text is accepted")
        .expect("something to change");
    String::from_utf8(out).unwrap()
}

fn unchanged(text: &str, wanted: &[Wanted]) {
    assert_eq!(merge_config(text.as_bytes(), wanted), Ok(None));
}

#[test]
fn an_empty_file_gets_exactly_the_required_section() {
    assert_eq!(merged("", &[POWER]), format!("[Power]\r\n{POWER_LINES}"));
    assert_eq!(
        merged("", &[LOG]),
        "[Log]\r\nFileSizeLimit.Int=-1\r\n".to_string()
    );
}

#[test]
fn a_file_with_only_a_byte_order_mark_keeps_it() {
    assert_eq!(
        merged("\u{feff}", &[LOG]),
        "\u{feff}[Log]\r\nFileSizeLimit.Int=-1\r\n"
    );
}

#[test]
fn a_missing_section_is_added_at_the_end_with_the_files_line_ends() {
    let text = "[Graphics]\nWidth=1\n";
    assert_eq!(
        merged(text, &[LOG]),
        "[Graphics]\nWidth=1\n\n[Log]\nFileSizeLimit.Int=-1\n"
    );
}

#[test]
fn a_missing_key_goes_at_the_end_of_its_section_not_after_the_next_one() {
    let text = "[Power]\nLogLevel=1\n\n[Decks]\nLogLevel=1\n";
    assert_eq!(
        merged(text, &[POWER]),
        "[Power]\nLogLevel=1\nFilePrinting=true\nVerbose=true\n\n[Decks]\nLogLevel=1\n"
    );
}

#[test]
fn a_comment_before_the_next_section_stays_with_that_section() {
    let text = "[Log]\nOther=1\n; the next one\n[Graphics]\nWidth=1\n";
    assert_eq!(
        merged(text, &[LOG]),
        "[Log]\nOther=1\nFileSizeLimit.Int=-1\n; the next one\n[Graphics]\nWidth=1\n"
    );
}

#[test]
fn a_wrong_value_is_the_only_line_changed_and_keeps_its_spacing() {
    let text = "; mine\n[Log]\nOther = 2\n  FileSizeLimit.Int = 10000 \nLast=3\n";
    assert_eq!(
        merged(text, &[LOG]),
        "; mine\n[Log]\nOther = 2\n  FileSizeLimit.Int = -1 \nLast=3\n"
    );
}

#[test]
fn a_right_value_is_left_alone_in_any_case_and_spacing() {
    unchanged("[log]\n filesizelimit.int =  -1\n", &[LOG]);
    unchanged(
        "[Power]\nloglevel=1\nFILEPRINTING=True\nVerbose = TRUE\n",
        &[POWER],
    );
}

#[test]
fn with_duplicates_the_last_line_the_game_reads_is_the_one_fixed() {
    let text = "[Log]\nFileSizeLimit.Int=-1\nFileSizeLimit.Int=5\n";
    assert_eq!(
        merged(text, &[LOG]),
        "[Log]\nFileSizeLimit.Int=-1\nFileSizeLimit.Int=-1\n"
    );
    // A bad line the last one already overrides is not touched.
    unchanged("[Log]\nFileSizeLimit.Int=5\nFileSizeLimit.Int=-1\n", &[LOG]);
}

#[test]
fn a_section_written_twice_gets_a_missing_key_in_its_last_block() {
    let text = "[Power]\nLogLevel=1\n[Decks]\nA=1\n[Power]\nFilePrinting=true\n";
    assert_eq!(
        merged(text, &[POWER]),
        "[Power]\nLogLevel=1\n[Decks]\nA=1\n[Power]\nFilePrinting=true\nVerbose=true\n"
    );
}

#[test]
fn comments_and_keys_in_other_sections_do_not_count_as_the_setting() {
    let text = "FileSizeLimit.Int=-1\n; FileSizeLimit.Int=-1\n# FileSizeLimit.Int=-1\n[Graphics]\nFileSizeLimit.Int=-1\n";
    assert_eq!(
        merged(text, &[LOG]),
        format!("{text}\n[Log]\nFileSizeLimit.Int=-1\n")
    );
}

#[test]
fn crlf_files_stay_crlf() {
    let text = "[Log]\r\nOther=1\r\n";
    assert_eq!(
        merged(text, &[LOG]),
        "[Log]\r\nOther=1\r\nFileSizeLimit.Int=-1\r\n"
    );
}

#[test]
fn mixed_line_ends_use_the_majority_for_new_lines_and_keep_the_rest() {
    let text = "[Log]\r\nA=1\r\nB=2\nC=3\r\n";
    assert_eq!(
        merged(text, &[LOG]),
        "[Log]\r\nA=1\r\nB=2\nC=3\r\nFileSizeLimit.Int=-1\r\n"
    );
    let text = "[Log]\nA=1\nB=2\r\nC=3\n";
    assert_eq!(
        merged(text, &[LOG]),
        "[Log]\nA=1\nB=2\r\nC=3\nFileSizeLimit.Int=-1\n"
    );
}

#[test]
fn a_byte_order_mark_is_kept() {
    let text = "\u{feff}[Log]\r\nOther=1\r\n";
    assert_eq!(
        merged(text, &[LOG]),
        "\u{feff}[Log]\r\nOther=1\r\nFileSizeLimit.Int=-1\r\n"
    );
    unchanged("\u{feff}[Log]\r\nFileSizeLimit.Int=-1\r\n", &[LOG]);
}

#[test]
fn a_missing_final_newline_stays_missing() {
    assert_eq!(
        merged("[Log]\nOther=1", &[LOG]),
        "[Log]\nOther=1\nFileSizeLimit.Int=-1"
    );
    assert_eq!(
        merged("[Graphics]\nWidth=1", &[LOG]),
        "[Graphics]\nWidth=1\n\n[Log]\nFileSizeLimit.Int=-1"
    );
    assert_eq!(
        merged("[Log]\nFileSizeLimit.Int=5", &[LOG]),
        "[Log]\nFileSizeLimit.Int=-1"
    );
}

#[test]
fn a_blank_line_at_the_end_is_not_doubled_before_a_new_section() {
    assert_eq!(
        merged("[Graphics]\nWidth=1\n\n", &[LOG]),
        "[Graphics]\nWidth=1\n\n[Log]\nFileSizeLimit.Int=-1\n"
    );
}

#[test]
fn several_sections_are_handled_in_one_pass() {
    let text = "[Power]\nLogLevel=1\n";
    assert_eq!(
        merged(text, &[POWER, LOADING_SCREEN]),
        "[Power]\nLogLevel=1\nFilePrinting=true\nVerbose=true\n\n[LoadingScreen]\nLogLevel=1\nFilePrinting=true\n"
    );
}

#[test]
fn the_second_run_changes_nothing() {
    for start in [
        "",
        "\u{feff}",
        "[Graphics]\nWidth=1",
        "[Power]\nLogLevel=0\nVerbose=false\n[Decks]\nLogLevel=1\n",
        "[Log]\r\nFileSizeLimit.Int = 10000\r\n",
    ] {
        let once = merge_config(start.as_bytes(), &[POWER, LOG, LOADING_SCREEN])
            .unwrap()
            .expect("the start needs a change");
        assert_eq!(merge_config(&once, &[POWER, LOG, LOADING_SCREEN]), Ok(None));
    }
}

#[test]
fn what_it_writes_passes_the_check() {
    for start in [
        "",
        "[Power]\nLogLevel=0\nFilePrinting=false\n",
        "garbage before\n[Decks]\nA=1\n",
        "[power]\n LogLevel = 5\n",
    ] {
        let out = merge_config(start.as_bytes(), &[POWER]).unwrap().unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(
            check_text(&text, POWER_SECTION, &REQUIRED_POWER),
            FileCheck::Ok
        );
    }
}

#[test]
fn non_ascii_text_in_the_players_lines_is_kept() {
    let text = "; añadido por mí — ñandú\n[Log]\nNombre=Ñu\n";
    assert_eq!(
        merged(text, &[LOG]),
        "; añadido por mí — ñandú\n[Log]\nNombre=Ñu\nFileSizeLimit.Int=-1\n"
    );
}

#[test]
fn text_that_is_not_utf8_or_ascii_is_refused_and_nothing_is_made() {
    // UTF-16 with its byte order mark, both orders.
    let le: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("[Log]\n".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let be: Vec<u8> = [0xFE, 0xFF]
        .into_iter()
        .chain("[Log]\n".encode_utf16().flat_map(u16::to_be_bytes))
        .collect();
    assert_eq!(merge_config(&le, &[LOG]), Err(MergeRefusal::NotPlainText));
    assert_eq!(merge_config(&be, &[LOG]), Err(MergeRefusal::NotPlainText));
    // UTF-16 without a mark has zero bytes; a single stray byte is not UTF-8.
    assert_eq!(
        merge_config(b"[\0L\0o\0g\0]\0", &[LOG]),
        Err(MergeRefusal::NotPlainText)
    );
    assert_eq!(
        merge_config(b"[Log]\nA=\xFF\n", &[LOG]),
        Err(MergeRefusal::NotPlainText)
    );
}

#[test]
fn a_lone_carriage_return_is_not_a_line_end() {
    // The game's reader splits on \n; text with a stray \r is one line.
    let text = "[Log]\nA=1\rB=2\n";
    assert_eq!(
        merged(text, &[LOG]),
        "[Log]\nA=1\rB=2\nFileSizeLimit.Int=-1\n"
    );
}
