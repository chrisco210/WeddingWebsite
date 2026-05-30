// ── NameSimilaritySearcher trait ──────────────────────────────────────────────

use crate::guest_list::{GuestList, PartyEntry};

/// Searches a guest list by name similarity and supports party lookup by ID.
pub trait NameSimilaritySearcher {
    /// Returns all parties whose guests score above the similarity threshold for
    /// `query`, sorted descending by score.
    fn search(&self, query: &str) -> Vec<(f64, &PartyEntry)>;

    /// Looks up a party by its exact ID.
    fn get_party(&self, party_id: &str) -> Option<&PartyEntry>;
}

// Blanket implementation so that `&T` is also a searcher when `T` is.
impl<T: NameSimilaritySearcher> NameSimilaritySearcher for &T {
    fn search(&self, query: &str) -> Vec<(f64, &PartyEntry)> {
        (*self).search(query)
    }

    fn get_party(&self, party_id: &str) -> Option<&PartyEntry> {
        (*self).get_party(party_id)
    }
}

// ── Fuzzy / phonetic scoring ──────────────────────────────────────────────────

fn score_name(guest_name: &str, guest_aliases: &[&str], query: &str) -> f64 {
    let guest_lower = guest_name.to_lowercase();
    let query_lower = query.to_lowercase();

    if guest_lower.contains(&query_lower) {
        return 1.0;
    }

    let full_score = strsim::jaro_winkler(&guest_lower, &query_lower);

    let mut guest_words: Vec<&str> = guest_lower.split_whitespace().collect();
    guest_words.extend_from_slice(guest_aliases);

    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    let word_score: f64 = query_words
        .iter()
        .map(|qw| {
            guest_words
                .iter()
                .map(|gw| strsim::jaro_winkler(qw, gw))
                .fold(0.0_f64, f64::max)
        })
        .sum::<f64>()
        / query_words.len().max(1) as f64;

    full_score.max(word_score)
}

// ── FuzzyNameSearcher ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FuzzyNameSearcher<G> {
    guest_list: G,
}

impl<G: GuestList> FuzzyNameSearcher<G> {
    pub fn new(guest_list: G) -> Self {
        Self { guest_list }
    }
}

impl<G: GuestList> NameSimilaritySearcher for FuzzyNameSearcher<G> {
    fn search(&self, query: &str) -> Vec<(f64, &PartyEntry)> {
        const THRESHOLD: f64 = 0.75;

        let mut scored: Vec<(f64, &PartyEntry)> = self
            .guest_list
            .all_parties()
            .into_iter()
            .filter_map(|party| {
                let best = party
                    .guests
                    .iter()
                    .map(|g| score_name(&g.name, &g.aliases, query))
                    .fold(0.0_f64, f64::max);
                (best >= THRESHOLD).then_some((best, party))
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    fn get_party(&self, party_id: &str) -> Option<&PartyEntry> {
        self.guest_list.get_party(party_id)
    }
}
