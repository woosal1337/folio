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

        let s = "abcd";
        assert_eq!(truncate_on_char_boundary(s, s.len()), s);
    }

    #[test]
    fn never_splits_a_multibyte_codepoint_at_any_cap() {
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
        let s = "Şu an ekranı mı kaydediyor? ".repeat(50);
        let out = truncate_on_char_boundary(&s, 101);
        assert!(out.len() <= 101);
        assert!(s.starts_with(out));
        assert!(s.is_char_boundary(out.len()));
    }

    #[test]
    fn cap_landing_mid_codepoint_backs_off() {
        assert_eq!(truncate_on_char_boundary("ü", 1), "");

        assert_eq!(truncate_on_char_boundary("aü", 2), "a");
    }
}
