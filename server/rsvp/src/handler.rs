use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use aws_lambda_events::{apigw::ApiGatewayProxyResponse, http::HeaderMap};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::store::RsvpStore;

// ── Embedded guest list ───────────────────────────────────────────────────────
// Edit guests.csv (in the same directory as this file) to add your guests.
// Format: party_id,party_display_name,guest_name
// Lines beginning with '#' and blank lines are ignored.

const GUEST_CSV: &str = include_str!("guests.csv");

#[derive(Debug, Clone)]
struct GuestEntry {
    name: String,
}

#[derive(Debug, Clone)]
struct PartyEntry {
    id: String,
    display_name: String,
    guests: Vec<GuestEntry>,
}

static GUEST_LIST: OnceLock<HashMap<String, PartyEntry>> = OnceLock::new();

fn guest_list() -> &'static HashMap<String, PartyEntry> {
    GUEST_LIST.get_or_init(|| {
        let mut map: HashMap<String, PartyEntry> = HashMap::new();
        for line in GUEST_CSV.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.splitn(3, ',');
            let (Some(id), Some(display), Some(name)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let id = id.trim().to_string();
            let display = display.trim().to_string();
            let name = name.trim().to_string();
            let entry = map.entry(id.clone()).or_insert_with(|| PartyEntry {
                id,
                display_name: display,
                guests: Vec::new(),
            });
            entry.guests.push(GuestEntry { name });
        }
        map
    })
}

// ── Fuzzy / phonetic name search ─────────────────────────────────────────────

fn score_name(guest_name: &str, query: &str) -> f64 {
    let guest_lower = guest_name.to_lowercase();
    let query_lower = query.to_lowercase();

    if guest_lower.contains(&query_lower) {
        return 1.0;
    }

    let full_score = strsim::jaro_winkler(&guest_lower, &query_lower);

    let guest_words: Vec<&str> = guest_lower.split_whitespace().collect();
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

fn search_parties(query: &str, max_results: usize) -> Vec<SearchMatch> {
    const THRESHOLD: f64 = 0.75;

    let mut scored: Vec<(f64, &PartyEntry)> = guest_list()
        .values()
        .filter_map(|party| {
            let best = party
                .guests
                .iter()
                .map(|g| score_name(&g.name, query))
                .fold(0.0_f64, f64::max);
            (best >= THRESHOLD).then_some((best, party))
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    scored
        .into_iter()
        .take(max_results)
        .map(|(_, p)| SearchMatch {
            party_id: p.id.clone(),
            display_name: p.display_name.clone(),
            guest_names: p.guests.iter().map(|g| g.name.clone()).collect(),
        })
        .collect()
}

// ── Public DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SearchMatch {
    pub party_id: String,
    pub display_name: String,
    pub guest_names: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestRsvp {
    pub name: String,
    pub attending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dietary_restrictions: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuestWithRsvp {
    pub name: String,
    pub rsvp: Option<GuestRsvp>,
}

#[derive(Debug, Serialize)]
pub struct PartyResult {
    pub party_id: String,
    pub display_name: String,
    pub guests: Vec<GuestWithRsvp>,
}

#[derive(Debug, Deserialize)]
pub struct PutRsvpInput {
    pub party_id: String,
    pub responses: Vec<GuestRsvp>,
}

// ── Error types ───────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<HandlerError> for ApiGatewayProxyResponse {
    fn from(err: HandlerError) -> Self {
        let status = match &err {
            HandlerError::NotFound(_) => 404,
            HandlerError::BadRequest(_) => 400,
            HandlerError::Internal(_) => 500,
        };
        ApiGatewayProxyResponse {
            status_code: status,
            multi_value_headers: HeaderMap::new(),
            is_base64_encoded: Some(false),
            body: Some(aws_lambda_events::encodings::Body::Text(err.to_string())),
            headers: HeaderMap::new(),
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct HandlerImpl<S> {
    pub store: S,
}

impl<S: RsvpStore> HandlerImpl<S> {
    /// Fuzzy-search guests by name; returns up to 5 matching parties.
    pub fn search(&self, query: &str) -> SearchResult {
        SearchResult {
            matches: search_parties(query, 5),
        }
    }

    /// Return a party's guests along with any previously submitted RSVP data.
    pub async fn get_party(&self, party_id: &str) -> Result<PartyResult, HandlerError> {
        let party = guest_list()
            .get(party_id)
            .ok_or_else(|| HandlerError::NotFound(format!("party '{party_id}' not found")))?;

        let existing: HashMap<String, GuestRsvp> = self
            .store
            .load(party_id)
            .await?
            .into_iter()
            .map(|r| (r.name.clone(), r))
            .collect();

        let guests = party
            .guests
            .iter()
            .map(|g| GuestWithRsvp {
                name: g.name.clone(),
                rsvp: existing.get(&g.name).cloned(),
            })
            .collect();

        Ok(PartyResult {
            party_id: party.id.clone(),
            display_name: party.display_name.clone(),
            guests,
        })
    }

    /// Validate and persist RSVP responses for a party.
    pub async fn submit_rsvp(&self, input: PutRsvpInput) -> Result<(), HandlerError> {
        let party = guest_list().get(&input.party_id).ok_or_else(|| {
            HandlerError::NotFound(format!("party '{}' not found", input.party_id))
        })?;

        let valid_names: HashSet<&str> = party.guests.iter().map(|g| g.name.as_str()).collect();
        for response in &input.responses {
            if !valid_names.contains(response.name.as_str()) {
                return Err(HandlerError::BadRequest(format!(
                    "'{}' is not in party '{}'",
                    response.name, input.party_id
                )));
            }
        }

        self.store.save(&input.party_id, &input.responses).await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // ── Mock store ────────────────────────────────────────────────────────────

    /// In-memory RsvpStore for unit tests — no database required.
    #[derive(Clone, Default)]
    struct MockRsvpStore {
        data: Arc<Mutex<HashMap<String, Vec<GuestRsvp>>>>,
    }

    impl MockRsvpStore {
        fn new() -> Self {
            Self::default()
        }
    }

    impl RsvpStore for MockRsvpStore {
        async fn load(&self, party_id: &str) -> Result<Vec<GuestRsvp>, HandlerError> {
            Ok(self
                .data
                .lock()
                .unwrap()
                .get(party_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn save(&self, party_id: &str, responses: &[GuestRsvp]) -> Result<(), HandlerError> {
            self.data
                .lock()
                .unwrap()
                .insert(party_id.to_string(), responses.to_vec());
            Ok(())
        }
    }

    fn handler() -> HandlerImpl<MockRsvpStore> {
        HandlerImpl {
            store: MockRsvpStore::new(),
        }
    }

    // ── Search tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_search_exact_full_name() {
        let results = search_parties("John Smith", 5);
        assert!(!results.is_empty(), "expected a match for 'John Smith'");
        assert!(results.iter().any(|m| m.party_id == "p001"));
    }

    #[test]
    fn test_search_first_name_only() {
        let results = search_parties("Alice", 5);
        assert!(
            results.iter().any(|m| m.party_id == "p002"),
            "expected p002"
        );
    }

    #[test]
    fn test_search_last_name_only() {
        let results = search_parties("Williams", 5);
        assert!(
            results.iter().any(|m| m.party_id == "p003"),
            "expected p003"
        );
    }

    #[test]
    fn test_search_case_insensitive() {
        assert!(
            search_parties("john smith", 5)
                .iter()
                .any(|m| m.party_id == "p001")
        );
        assert!(
            search_parties("JOHN SMITH", 5)
                .iter()
                .any(|m| m.party_id == "p001")
        );
    }

    #[test]
    fn test_search_fuzzy_typo() {
        // "Jon Smith" is a one-character typo of "John Smith"
        let results = search_parties("Jon Smith", 5);
        assert!(
            results.iter().any(|m| m.party_id == "p001"),
            "fuzzy search should match 'John Smith' for 'Jon Smith'"
        );
    }

    #[test]
    fn test_search_no_match_gibberish() {
        assert!(search_parties("xqzgibberish", 5).is_empty());
    }

    #[test]
    fn test_search_result_includes_all_party_guests() {
        let results = search_parties("Smith", 5);
        let smith = results.iter().find(|m| m.party_id == "p001").unwrap();
        assert!(smith.guest_names.contains(&"John Smith".to_string()));
        assert!(smith.guest_names.contains(&"Jane Smith".to_string()));
    }

    #[test]
    fn test_search_result_count_capped_at_limit() {
        assert!(search_parties("Smith", 1).len() <= 1);
    }

    #[test]
    fn test_search_results_ordered_by_score() {
        // Exact substring match should rank first
        let results = search_parties("Emma Williams", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].party_id, "p003");
    }

    // ── Validation tests ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_party_not_found() {
        let err = handler().get_party("nonexistent").await.unwrap_err();
        assert!(matches!(err, HandlerError::NotFound(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn test_submit_rsvp_party_not_found() {
        let err = handler()
            .submit_rsvp(PutRsvpInput {
                party_id: "nonexistent".to_string(),
                responses: vec![],
            })
            .await
            .unwrap_err();
        assert!(matches!(err, HandlerError::NotFound(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn test_submit_rsvp_guest_not_in_party() {
        let err = handler()
            .submit_rsvp(PutRsvpInput {
                party_id: "p001".to_string(),
                responses: vec![GuestRsvp {
                    name: "Not A Real Guest".to_string(),
                    attending: true,
                    dietary_restrictions: None,
                }],
            })
            .await
            .unwrap_err();
        assert!(matches!(err, HandlerError::BadRequest(_)), "got {err:?}");
    }

    // ── Store round-trip tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_party_no_prior_rsvp() {
        let result = handler().get_party("p001").await.unwrap();

        assert_eq!(result.party_id, "p001");
        assert_eq!(result.display_name, "Smith Family");
        assert_eq!(result.guests.len(), 2);
        assert!(
            result.guests.iter().all(|g| g.rsvp.is_none()),
            "all RSVPs should be None before any submission"
        );
    }

    #[tokio::test]
    async fn test_submit_then_get_party_returns_rsvp_data() {
        let h = handler();

        h.submit_rsvp(PutRsvpInput {
            party_id: "p001".to_string(),
            responses: vec![
                GuestRsvp {
                    name: "John Smith".to_string(),
                    attending: true,
                    dietary_restrictions: Some("vegetarian".to_string()),
                },
                GuestRsvp {
                    name: "Jane Smith".to_string(),
                    attending: false,
                    dietary_restrictions: None,
                },
            ],
        })
        .await
        .unwrap();

        let result = h.get_party("p001").await.unwrap();

        let john = result
            .guests
            .iter()
            .find(|g| g.name == "John Smith")
            .unwrap();
        let john_rsvp = john.rsvp.as_ref().expect("John should have an RSVP");
        assert!(john_rsvp.attending);
        assert_eq!(
            john_rsvp.dietary_restrictions.as_deref(),
            Some("vegetarian")
        );

        let jane = result
            .guests
            .iter()
            .find(|g| g.name == "Jane Smith")
            .unwrap();
        let jane_rsvp = jane.rsvp.as_ref().expect("Jane should have an RSVP");
        assert!(!jane_rsvp.attending);
        assert!(jane_rsvp.dietary_restrictions.is_none());
    }

    #[tokio::test]
    async fn test_submit_overwrites_previous_submission() {
        let h = handler();

        h.submit_rsvp(PutRsvpInput {
            party_id: "p002".to_string(),
            responses: vec![GuestRsvp {
                name: "Alice Doe".to_string(),
                attending: false,
                dietary_restrictions: None,
            }],
        })
        .await
        .unwrap();

        h.submit_rsvp(PutRsvpInput {
            party_id: "p002".to_string(),
            responses: vec![GuestRsvp {
                name: "Alice Doe".to_string(),
                attending: true,
                dietary_restrictions: Some("gluten-free".to_string()),
            }],
        })
        .await
        .unwrap();

        let result = h.get_party("p002").await.unwrap();
        let alice = result
            .guests
            .iter()
            .find(|g| g.name == "Alice Doe")
            .unwrap();
        let rsvp = alice.rsvp.as_ref().expect("Alice should have an RSVP");

        assert!(
            rsvp.attending,
            "second submission should have overwritten the first"
        );
        assert_eq!(rsvp.dietary_restrictions.as_deref(), Some("gluten-free"));
    }

    #[tokio::test]
    async fn test_partial_party_submission() {
        // Only one of three party members submits; the others should stay None
        let h = handler();

        h.submit_rsvp(PutRsvpInput {
            party_id: "p003".to_string(),
            responses: vec![GuestRsvp {
                name: "Carol Williams".to_string(),
                attending: true,
                dietary_restrictions: None,
            }],
        })
        .await
        .unwrap();

        let result = h.get_party("p003").await.unwrap();
        assert_eq!(result.guests.len(), 3);

        let carol = result
            .guests
            .iter()
            .find(|g| g.name == "Carol Williams")
            .unwrap();
        assert!(carol.rsvp.is_some());

        let without_rsvp = result
            .guests
            .iter()
            .filter(|g| g.name != "Carol Williams")
            .filter(|g| g.rsvp.is_none())
            .count();
        assert_eq!(without_rsvp, 2);
    }

    #[tokio::test]
    async fn test_store_is_shared_across_clone() {
        // HandlerImpl::clone shares the same underlying store (Arc)
        let h1 = handler();
        let h2 = h1.clone();

        h1.submit_rsvp(PutRsvpInput {
            party_id: "p001".to_string(),
            responses: vec![GuestRsvp {
                name: "John Smith".to_string(),
                attending: true,
                dietary_restrictions: None,
            }],
        })
        .await
        .unwrap();

        let result = h2.get_party("p001").await.unwrap();
        let john = result
            .guests
            .iter()
            .find(|g| g.name == "John Smith")
            .unwrap();
        assert!(john.rsvp.is_some(), "h2 should see data written by h1");
    }
}
