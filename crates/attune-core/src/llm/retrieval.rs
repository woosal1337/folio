use std::path::Path;

use crate::transcription::SessionTranscript;

pub fn relevance_score(text: &str, query_tokens: &[&str]) -> f32 {
    if query_tokens.is_empty() || text.is_empty() {
        return 0.0;
    }
    let text_lc = text.to_lowercase();
    let matched = query_tokens
        .iter()
        .filter(|t| !t.is_empty() && text_lc.contains(*t))
        .count();
    matched as f32 / query_tokens.len() as f32
}

pub fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .map(|t| t.to_lowercase())
        .filter(|t| t.len() >= 3)
        .collect()
}

pub fn combined_score(relevance: f32, days_ago: f64) -> f32 {
    let recency = (-days_ago / 30.0).exp() as f32;
    relevance * 0.8 + recency * 0.2
}

pub fn transcript_excerpt(
    session_dir: &Path,
    query_tokens: &[&str],
    max_chars: usize,
) -> Option<String> {
    let transcript_path = session_dir.join("transcript.json");
    let transcript = SessionTranscript::read_json(&transcript_path).ok()?;

    let text = transcript
        .channels
        .iter()
        .flat_map(|ch| ch.segments.iter())
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    if text.is_empty() {
        return None;
    }

    let sentences: Vec<&str> = text
        .split(['.', '?', '!'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let best_window = sentences.windows(3).max_by(|a, b| {
        let sa = relevance_score(&a.join(". "), query_tokens);
        let sb = relevance_score(&b.join(". "), query_tokens);
        sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
    })?;

    let excerpt = best_window.join(". ");
    if excerpt.len() <= max_chars {
        Some(excerpt)
    } else {
        let truncated = excerpt[..max_chars]
            .rsplit_once(' ')
            .map(|(s, _)| s)
            .unwrap_or(&excerpt[..max_chars]);
        Some(format!("{truncated}…"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relevance_score_perfect_match() {
        let tokens: Vec<&str> = vec!["pricing", "decision"];
        assert!((relevance_score("we made a pricing decision", &tokens) - 1.0).abs() < 0.01);
    }

    #[test]
    fn relevance_score_partial_match() {
        let tokens: Vec<&str> = vec!["pricing", "decision", "roadmap"];
        let score = relevance_score("pricing was discussed", &tokens);
        assert!((score - 0.333).abs() < 0.01);
    }

    #[test]
    fn relevance_score_no_match() {
        let tokens: Vec<&str> = vec!["pricing"];
        assert_eq!(relevance_score("nothing relevant", &tokens), 0.0);
    }

    #[test]
    fn tokenize_strips_short_words() {
        let tokens = tokenize_query("what did we decide on pricing?");
        assert!(tokens.contains(&"decide".to_string()));
        assert!(tokens.contains(&"pricing".to_string()));
        assert!(!tokens.contains(&"on".to_string()));
        assert!(!tokens.contains(&"we".to_string()));
    }

    #[test]
    fn combined_score_weights_recency() {
        let recent = combined_score(0.5, 1.0);
        let old = combined_score(0.5, 180.0);
        assert!(recent > old);
    }
}
