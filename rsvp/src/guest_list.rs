use std::collections::HashMap;
use std::sync::OnceLock;

const GUEST_CSV: &'static str = include_str!("guests.csv");

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GuestEntry {
    pub name: String,
    pub aliases: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct PartyEntry {
    pub id: String,
    pub display_name: String,
    pub guests: Vec<GuestEntry>,
}

// ── Trait ─────────────────────────────────────────────────────────────────────

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

// ── CsvNameSearcher ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CsvNameSearcher {
    map: HashMap<String, PartyEntry>,
}

impl CsvNameSearcher {
    /// Construct a searcher from an existing map (useful for tests).
    pub fn new(map: HashMap<String, PartyEntry>) -> Self {
        Self { map }
    }

    /// Initialise from the embedded CSV exactly once and return a static
    /// reference.  This is the entry point for production use.
    pub fn init_static() -> &'static Self {
        static INSTANCE: OnceLock<CsvNameSearcher> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            map: Self::load_csv(),
        })
    }

    fn load_csv() -> HashMap<String, PartyEntry> {
        let mut map: HashMap<String, PartyEntry> = HashMap::new();
        for line in GUEST_CSV.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.splitn(4, ',');
            let (Some(id), Some(display), Some(name), Some(alias)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                panic!("Invalid guests.csv")
            };
            let id = id.trim().to_string();
            let display = display.trim().to_string();
            let name = name.trim().to_string();
            let entry = map.entry(id.clone()).or_insert_with(|| PartyEntry {
                id,
                display_name: display,
                guests: Vec::new(),
            });

            let aliases = if alias.trim().is_empty() {
                vec![]
            } else {
                vec![alias]
            };

            entry.guests.push(GuestEntry { name, aliases });
        }
        map
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

// ── NameSimilaritySearcher impl ───────────────────────────────────────────────

impl NameSimilaritySearcher for CsvNameSearcher {
    fn search(&self, query: &str) -> Vec<(f64, &PartyEntry)> {
        const THRESHOLD: f64 = 0.75;

        let mut scored: Vec<(f64, &PartyEntry)> = self
            .map
            .values()
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
        self.map.get(party_id)
    }
}
