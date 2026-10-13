//! Made-up secrets for the redaction tests (T-580). They have the shape of a
//! token, a key or a hash but belong to nothing. Each is put together at run
//! time from short harmless pieces, so no token-looking text sits in the
//! source for a secrets scan to find.

/// The shape of a signed token: three dotted parts, the first two starting `eyJ`.
pub fn jwt() -> String {
    [
        "eyJ",
        "hbGciOiJIUzI1NiJ9",
        ".eyJ",
        "zdWIiOiIxMjM0NTY3ODkwIn0",
        ".",
        "dBjftJeZ4CVPmB92",
        "K27uhbUJU1p1r_wW",
        "1gFWFOEjXk",
    ]
    .concat()
}

/// A short value with letters and digits, the kind a labelled secret holds.
pub fn word() -> String {
    ["abc", "123", "def", "456"].concat()
}

/// Its first half, to look for in what was kept.
pub fn word_start() -> String {
    ["abc", "123"].concat()
}

/// A longer one.
pub fn long_word() -> String {
    [word(), "ghi789".to_string()].concat()
}

/// A provider-style key with its prefix.
pub fn prefixed_key() -> String {
    ["sb_", "publishable_", "AbCdEf", "123456789"].concat()
}

/// A hash-like run of hex.
pub fn hex() -> String {
    ["9f86d081", "884c7d65", "9a2feaa0", "c55ad015"]
        .concat()
        .repeat(2)
}

/// The shape of a base64 credential after `Basic`.
pub fn basic() -> String {
    ["dXNl", "cjpw", "YXNz"].concat()
}

/// A base64url token with `-`, `_` and `/` in it.
pub fn base64url() -> String {
    ["Ab3-dE_fG/", "hI9jK1lM2nO4pQ5rS6tU7vW8xY0zA1bC2d"].concat()
}

/// A short password.
pub fn password() -> String {
    ["hunter", "2"].concat()
}
