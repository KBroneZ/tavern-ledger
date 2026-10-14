// The characters a display name may use: the database's list
// (private.display_name_chars_ok). A to Z with accents (U+00C0 to U+017F
// without the multiplication and division signs), digits, space, _ . and - (at least one letter or digit).
// Other alphabets, symbols and combining marks are refused, so a name cannot
// pass for another one with look-alike letters; the database has the last word.
export const NAME_CHARS = /^(?=.*[A-Za-z0-9À-ÖØ-öø-ſ])[A-Za-z0-9À-ÖØ-öø-ſ _.-]+$/;
