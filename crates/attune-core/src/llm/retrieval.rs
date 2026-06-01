//! Iterative retrieval for cross-library Ask Attune (GET-203).
//!
//! Implements a two-tier notes-over-transcripts retrieval strategy:
//!
//! ## Tier 1 — summary scan (cheap, broad)
//!
//! Score all note summaries against the query using a simple
//! term-frequency model (each query token found in the summary adds
//! weight). Sort by score × recency decay. Take the top-N shortlist.
//!
//! ## Tier 2 — transcript splice (expensive, verbatim)
//!
//! For notes whose Tier-1 score crosses a high-confidence threshold,
//! pull a short excerpt from the raw transcript that's most similar
//! to the query and append it below the summary. Capped at 3 notes
//! to keep context budget manageable.
//!
//! This replaces the previous "most-recent N" strategy with "most-
//! relevant N", making longitudinal cross-library questions ("what
//! did we decide about X across all calls this quarter?") answerable
//! without blowing the token budget.

use std::path::Path;

use crate::transcription::SessionTranscript;

/// Score a text blob against a query. Each unique query token found
/// in `text` (case-insensitive) contributes 1 point. Repeated tokens
/// don't add extra weight — this is a set-intersection model.
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

/// Tokenize a natural-language query into lowercase terms suitable
/// for relevance scoring. Strips punctuation and discards stop-words
/// shorter than 3 characters.
pub fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .map(|t| t.to_lowercase())
        .filter(|t| t.len() >= 3)
        .collect()
}

/// Combined score: relevance × recency. `days_ago` is the age of the
/// note; older notes are down-weighted exponentially (half-life ~30d)
/// so ties are broken in favour of recent meetings.
pub fn combined_score(relevance: f32, days_ago: f64) -> f32 {
    let recency = (-days_ago / 30.0).exp() as f32; // 0..1, newer = closer to 1
    relevance * 0.8 + recency * 0.2
}

/// Pull the transcript excerpt most similar to `query_tokens` from a
/// session directory. Returns up to `max_chars` of the best paragraph.
///
/// This is Tier 2: called only for notes that scored high in Tier 1.
pub fn transcript_excerpt(
    session_dir: &Path,
    query_tokens: &[&str],
    max_chars: usize,
) -> Option<String> {
    let transcript_path = session_dir.join("transcript.json");
    let transcript = SessionTranscript::read_json(&transcript_path).ok()?;

    // Flatten to sentences (split on '.', '?', '!').
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

    // Find the sentence window with the highest relevance score.
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
        // Truncate on a word boundary.
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
        // Same relevance, recent note should score higher.
        let recent = combined_score(0.5, 1.0);
        let old = combined_score(0.5, 180.0);
        assert!(recent > old);
    }
}
