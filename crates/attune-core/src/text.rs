//! Small text utilities shared across the app.

/// Truncate `s` to at most `max_bytes`, never splitting a UTF-8 codepoint.
/// Returns the largest prefix of `s` whose byte length is ≤ `max_bytes`
/// ending on a `char` boundary (so the result is always valid UTF-8).
/// Returns `s` unchanged when it already fits.
///
/// Byte-indexing a `&str` (`&s[..n]`) panics when `n` lands mid-codepoint —
/// which happens for any multibyte text (Turkish, German, Japanese, …)
/// once it exceeds a fixed byte cap. That panic crashed agent runs and the
/// note Q&A on multilingual transcripts (GET-175).
pub fn truncate_on_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_whole_string_when_under_cap() {
        assert_eq!(truncate_on_char_boundary("héllo", 100), "héllo");
        // Exactly at the cap (byte length) is kept whole.
        let s = "abcd";
        assert_eq!(truncate_on_char_boundary(s, s.len()), s);
    }

    #[test]
    fn never_splits_a_multibyte_codepoint_at_any_cap() {
        // Mostly multibyte; every cap must yield a valid UTF-8 prefix.
        let s = "ünïcødé-Şu-日本語";
        for cap in 0..=s.len() + 2 {
            let out = truncate_on_char_boundary(s, cap);
            assert!(out.len() <= cap.min(s.len()));
            assert!(s.starts_with(out));
            assert!(s.is_char_boundary(out.len()));
        }
    }

    #[test]
    fn turkish_transcript_over_cap_does_not_panic() {
        // The exact GET-175 scenario: a non-ASCII transcript past the cap.
        let s = "Şu an ekranı mı kaydediyor? ".repeat(50);
        let out = truncate_on_char_boundary(&s, 101);
        assert!(out.len() <= 101);
        assert!(s.starts_with(out));
        assert!(s.is_char_boundary(out.len()));
    }

    #[test]
    fn cap_landing_mid_codepoint_backs_off() {
        // "ü" = 2 bytes (0xC3 0xBC). A cap of 1 into a leading "ü" must
        // back off to the previous boundary (here, empty).
        assert_eq!(truncate_on_char_boundary("ü", 1), "");
        // "aü": cap 2 lands inside "ü" (bytes a=1, ü=2-3) → keep "a".
        assert_eq!(truncate_on_char_boundary("aü", 2), "a");
    }
}
