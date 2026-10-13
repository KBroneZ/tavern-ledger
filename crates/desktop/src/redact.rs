//! What the error log must never hold (T-580, GDPR): player names, account
//! ids, emails, tokens and keys, and the user's name inside a path. Every
//! message goes through [`redact`] before it is written, so the log file is
//! already safe to read and to attach to a problem report. The rules err on
//! the side of cutting too much: a log line that lost a word is fine, one that
//! kept a name is not.
//!
//! What no rule can see is a bare name in free text (no `#`, no label). The
//! app therefore does not log free text it cannot vouch for: a panic's
//! formatted message and the words quoted in a page error are cut before they
//! get here (`applog::panic_text`, `log_commands::ui_message`).

use std::path::Path;
use std::sync::{LazyLock, OnceLock};

use regex::{Captures, Regex};

/// Where a redacted value stood.
const REDACTED: &str = "[redacted]";
const BATTLETAG: &str = "[battletag]";
const EMAIL: &str = "[email]";
const TOKEN: &str = "[token]";
/// The Windows user and computer names are not cut when shorter than this
/// (they would also cut ordinary words).
const MIN_WORD: usize = 3;

/// The folders and names of this user, replaced wherever they show.
/// Folders longest first, so `%APPDATA%` wins over the profile folder it sits in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dirs {
    dirs: Vec<(String, &'static str)>,
    /// The Windows user name and the computer name, as whole words.
    words: Vec<String>,
}

impl Dirs {
    pub fn new(profile: Option<&str>, appdata: Option<&str>, local: Option<&str>) -> Self {
        let mut dirs: Vec<(String, &'static str)> = [
            (appdata, "%APPDATA%"),
            (local, "%LOCALAPPDATA%"),
            (profile, "%USERPROFILE%"),
        ]
        .into_iter()
        .filter_map(|(dir, name)| Some((normal(dir?.trim().trim_end_matches(['\\', '/'])), name)))
        .filter(|(dir, _)| !dir.is_empty())
        .collect();
        dirs.sort_by_key(|(dir, _)| std::cmp::Reverse(dir.len()));
        Dirs {
            dirs,
            words: Vec::new(),
        }
    }

    /// Also cuts these names wherever they stand as a word.
    pub fn with_words(mut self, words: &[Option<String>]) -> Self {
        self.words = words
            .iter()
            .flatten()
            .map(|w| w.trim().to_string())
            .filter(|w| w.chars().count() >= MIN_WORD)
            .collect();
        self
    }

    /// The folders and names of this user, from the environment.
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Dirs::new(
            var("USERPROFILE").as_deref(),
            var("APPDATA").as_deref(),
            var("LOCALAPPDATA").as_deref(),
        )
        .with_words(&[var("USERNAME"), var("COMPUTERNAME")])
    }
}

/// Lowercase with one kind of separator; same length as the input (ASCII
/// changes only), so a position found here is a position in the original.
fn normal(text: &str) -> String {
    text.to_ascii_lowercase().replace('/', "\\")
}

fn is_separator(c: char) -> bool {
    c == '\\' || c == '/'
}

/// Replaces each place `dir` (any case, either separator) starts a path.
fn replace_dir(text: &str, dir: &str, name: &str) -> String {
    let haystack = normal(text);
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    while let Some(found) = haystack[from..].find(dir) {
        let start = from + found;
        let end = start + dir.len();
        let whole = text[end..].chars().next().is_none_or(is_separator);
        out.push_str(&text[from..start]);
        out.push_str(if whole { name } else { &text[start..end] });
        from = end;
    }
    out.push_str(&text[from..]);
    out
}

/// Replaces `word` (any case) where it stands alone.
fn replace_word(text: &str, word: &str) -> String {
    match Regex::new(&format!(r"(?i)\b{}\b", regex::escape(word))) {
        Ok(rule) => rule.replace_all(text, "[name]").into_owned(),
        Err(_) => text.to_string(),
    }
}

macro_rules! rule {
    ($name:ident, $pattern:expr) => {
        static $name: LazyLock<Regex> =
            LazyLock::new(|| Regex::new($pattern).expect("a fixed pattern"));
    };
}

// Any `X:\Users\<name>`, also when the environment does not know it. A
// Windows user name may hold spaces, so the name runs to the next separator.
rule!(
    USER_PATH,
    r#"(?i)\b[a-z]:[\\/]+users[\\/]+[^\\/"'<>|:*?]{1,64}"#
);
// `\\host\share`: the host is a computer's name.
rule!(UNC_PATH, r#"\\\\[^\\/\s"'<>|:*?]+[\\/][^\\/\s"'<>|:*?]*"#);
// `Name#1234`, also with the full-width and small forms of `#`.
rule!(
    BATTLETAG_SHAPE,
    r"[\p{L}\p{M}\p{N}_]*[#\x{ff03}\x{fe5f}]\p{N}{3,}"
);
// The account id as the log, serde or Debug writes it: `GameAccountId=[hi=…
// lo=…]`, `"GameAccountId":{"hi":…}`, `GameAccountId { hi: …, lo: … }`,
// `game_account_id: 12`. The label stays; its value and group go.
rule!(
    ACCOUNT_ID,
    r#"(?i)(game_?account_?id)(?:["']?\s*[:=]?\s*(?:\{[^}]*\}|\[[^\]]*\]|[^\s,;)\]}"']*))?"#
);
// The two halves of an account id by themselves.
rule!(
    ACCOUNT_HALF,
    r#"(?i)\b(account_?hi|account_?lo|hi|lo)\b(["']?\s*[:=]\s*)\d+"#
);
rule!(
    JWT,
    r"\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]*)?"
);
rule!(BEARER, r"(?i)\bbearer\s+\S+");
rule!(
    KEY_PREFIX,
    r"(?i)\b(?:sb_[a-z]+_|sbp_|sk-)[A-Za-z0-9_-]{8,}"
);
// `token=…`, `"refresh_token":"…"`, `session_token: …`, `Authorization: Basic …`,
// also inside escaped JSON (`\"token\":\"…\"`) and for the email asked for in a
// URL. The value is a quoted text, or the one word after the sign (and after a
// `Basic` or `Bearer`).
rule!(
    SECRET_VALUE,
    r#"(?i)\b([\w-]*(?:token|secret|passw(?:or)?d|api[_-]?key|anon_key|authorization|e-?mail))\b(\\?["']?\s*[:=]\s*)(?:\\?"[^"\\]*\\?"|'[^']*'|(?:(?:basic|bearer)\s+)?[^\s"',;&)}\]\\]+)"#
);
// An email, also with accents and when a URL wrote its `@` as `%40`.
rule!(
    EMAIL_SHAPE,
    r"[\p{L}\p{N}._%+-]+(?:@|%40)[\p{L}\p{N}.-]+\.\p{L}{2,}"
);
// A run of letters and digits this long (with a digit in it) is a key, a hash
// or a token, not a word.
rule!(OPAQUE, r"[A-Za-z0-9+=]{32,}");
// A base64 or base64url token keeps its `-`, `_` and `/`: long, with digits and
// both cases of letters, which no path, session name or word has.
rule!(OPAQUE_MIXED, r"[A-Za-z0-9+/=_-]{43,}");

/// `text` with the user's name, account ids, emails, tokens and keys cut out.
pub fn redact(text: &str) -> String {
    static DIRS: OnceLock<Dirs> = OnceLock::new();
    redact_with(text, DIRS.get_or_init(Dirs::from_env))
}

pub fn redact_with(text: &str, dirs: &Dirs) -> String {
    let mut out = text.to_string();
    for (dir, name) in &dirs.dirs {
        out = replace_dir(&out, dir, name);
    }
    for word in &dirs.words {
        out = replace_word(&out, word);
    }
    // Labelled secrets first, so a value is cut whole before the shapes below
    // see parts of it.
    out = SECRET_VALUE
        .replace_all(&out, |c: &Captures| format!("{}{}{REDACTED}", &c[1], &c[2]))
        .into_owned();
    let steps: [(&Regex, &str); 9] = [
        (&USER_PATH, "%USERPROFILE%"),
        (&UNC_PATH, r"\\[host]\[share]"),
        (&BEARER, "Bearer [token]"),
        (&JWT, TOKEN),
        (&KEY_PREFIX, TOKEN),
        (&EMAIL_SHAPE, EMAIL),
        (&BATTLETAG_SHAPE, BATTLETAG),
        (&ACCOUNT_ID, "$1=[redacted]"),
        (&ACCOUNT_HALF, "$1$2[redacted]"),
    ];
    for (rule, with) in steps {
        out = rule.replace_all(&out, with).into_owned();
    }
    out = OPAQUE_MIXED
        .replace_all(&out, |c: &Captures| {
            let run = c[0].as_bytes();
            let has = |f: fn(&u8) -> bool| run.iter().any(f);
            if has(u8::is_ascii_digit) && has(u8::is_ascii_uppercase) && has(u8::is_ascii_lowercase)
            {
                REDACTED.to_string()
            } else {
                c[0].to_string()
            }
        })
        .into_owned();
    OPAQUE
        .replace_all(&out, |c: &Captures| {
            if c[0].bytes().any(|b| b.is_ascii_digit()) {
                REDACTED.to_string()
            } else {
                c[0].to_string()
            }
        })
        .into_owned()
}

/// A folder for the log: when it is not under a folder of this user (which
/// `redact` names by its variable), only its last two parts, because the
/// folders above could hold a name (`D:\Juegos\Ana\Hearthstone\Logs`).
pub fn place(path: &Path) -> String {
    let text = redact(&path.display().to_string());
    if text.starts_with('%') {
        return text;
    }
    let parts: Vec<&str> = text.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
    match parts.as_slice() {
        [] => String::new(),
        [.., a, b] if parts.len() > 2 => format!(r"…\{a}\{b}"),
        _ => parts.join(r"\"),
    }
}

/// Characters that read as nothing or turn the text around (zero-width, bidi
/// and line/paragraph separators): a line must show what it says.
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{ad}'
            | '\u{200b}'..='\u{200f}'
            | '\u{2028}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{feff}'
    )
}

/// At most `max` characters, one line, no control or invisible characters.
pub fn one_line(text: &str, max: usize) -> String {
    let mut line: String = text
        .chars()
        .filter(|c| !is_invisible(*c))
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max + 1)
        .collect();
    if line.chars().count() > max {
        line = line.chars().take(max.saturating_sub(1)).collect();
        line.push('…');
    }
    line.trim().to_string()
}

#[cfg(test)]
#[path = "redact_tests.rs"]
mod tests;
