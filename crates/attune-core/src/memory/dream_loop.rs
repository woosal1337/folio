//! Out-of-band memory consolidation. v2 finding 030 / GET-56.
//!
//! Nightly-while-charging job that:
//!   1. Clusters near-duplicate memories by cosine similarity.
//!   2. Merges each cluster into a single canonical memory.
//!   3. Flags pairs that contradict across sessions.
//!   4. Writes timeline-shaped synthesis pages.
//!
//! Inspired by OpenClaw and the Anthropic "Dreaming" pattern. This
//! module owns the pure pieces: cosine similarity, clustering, the
//! contradiction detector, and the schedule policy. The actual job
//! is driven by a tokio task that hooks into the power state event
//! stream (only runs while plugged in + screen-locked).

use serde::{Deserialize, Serialize};

pub const DEFAULT_DUP_THRESHOLD: f32 = 0.92;
pub const DEFAULT_CONTRA_THRESHOLD: f32 = 0.55;
pub const DEFAULT_MIN_BATTERY_PCT: u32 = 80;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsolidationItem {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cluster {
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContradictionPair {
    pub left_id: String,
    pub right_id: String,
    pub similarity: f32,
}

/// Cosine similarity in [-1.0, 1.0]. Returns 0.0 when either vector
/// is empty or zero-norm.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

/// Greedy single-link clustering: every pair with similarity >=
/// `threshold` joins the same cluster. Returns clusters of size 2+;
/// singletons are dropped because they need no consolidation.
pub fn cluster_near_duplicates(items: &[ConsolidationItem], threshold: f32) -> Vec<Cluster> {
    let n = items.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in (i + 1)..n {
            if cosine(&items[i].embedding, &items[j].embedding) >= threshold {
                union(&mut parent, i, j);
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<String>> = std::collections::BTreeMap::new();
    for (idx, item) in items.iter().enumerate() {
        let root = find(&mut parent, idx);
        groups.entry(root).or_default().push(item.id.clone());
    }
    groups
        .into_values()
        .filter(|members| members.len() >= 2)
        .map(|mut members| {
            members.sort();
            Cluster { members }
        })
        .collect()
}

/// Detect contradiction candidates: pairs that are similar enough
/// to plausibly be about the same fact but not similar enough to
/// be the same statement. Threshold is the "interesting middle":
/// CONTRA_THRESHOLD <= similarity < DUP_THRESHOLD.
pub fn detect_contradictions(
    items: &[ConsolidationItem],
    contra_threshold: f32,
    dup_threshold: f32,
) -> Vec<ContradictionPair> {
    let mut out = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let sim = cosine(&items[i].embedding, &items[j].embedding);
            if sim >= contra_threshold && sim < dup_threshold {
                out.push(ContradictionPair {
                    left_id: items[i].id.clone(),
                    right_id: items[j].id.clone(),
                    similarity: sim,
                });
            }
        }
    }
    out
}

/// True when the consolidation job is allowed to run: plugged in,
/// battery high enough, and the screen has been idle long enough.
pub fn should_run(plugged_in: bool, battery_pct: u32, screen_idle_minutes: u32) -> bool {
    plugged_in && battery_pct >= DEFAULT_MIN_BATTERY_PCT && screen_idle_minutes >= 10
}

fn find(parent: &mut [usize], i: usize) -> usize {
    if parent[i] != i {
        parent[i] = find(parent, parent[i]);
    }
    parent[i]
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, embedding: Vec<f32>) -> ConsolidationItem {
        ConsolidationItem {
            id: id.into(),
            content: format!("content for {id}"),
            embedding,
        }
    }

    #[test]
    fn cosine_of_identical_vectors_is_one() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_of_orthogonal_vectors_is_zero() {
        assert!((cosine(&[1.0, 0.0], &[0.0, 1.0])).abs() < 1e-5);
    }

    #[test]
    fn cosine_handles_zero_norm_vectors() {
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }

    #[test]
    fn cluster_merges_similar_pairs() {
        let a = item("a", vec![1.0, 0.0]);
        let b = item("b", vec![0.99, 0.05]);
        let c = item("c", vec![0.0, 1.0]);
        let clusters = cluster_near_duplicates(&[a, b, c], 0.9);
        assert_eq!(clusters.len(), 1);
        let members = &clusters[0].members;
        assert!(members.contains(&"a".to_string()));
        assert!(members.contains(&"b".to_string()));
        assert!(!members.contains(&"c".to_string()));
    }

    #[test]
    fn cluster_drops_singletons() {
        let a = item("a", vec![1.0, 0.0]);
        let b = item("b", vec![0.0, 1.0]);
        let clusters = cluster_near_duplicates(&[a, b], 0.9);
        assert!(clusters.is_empty());
    }

    #[test]
    fn detect_contradictions_finds_middle_similarity_pairs() {
        let a = item("a", vec![1.0, 0.0]);
        let b = item("b", vec![0.7, 0.7]);
        let pairs = detect_contradictions(&[a, b], 0.5, 0.92);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn detect_contradictions_skips_pairs_above_dup_threshold() {
        let a = item("a", vec![1.0, 0.0]);
        let b = item("b", vec![0.99, 0.05]);
        let pairs = detect_contradictions(&[a, b], 0.5, 0.92);
        assert!(pairs.is_empty());
    }

    #[test]
    fn should_run_requires_power_battery_and_idle() {
        assert!(should_run(true, 90, 20));
        assert!(!should_run(false, 90, 20));
        assert!(!should_run(true, 70, 20));
        assert!(!should_run(true, 90, 1));
    }
}
