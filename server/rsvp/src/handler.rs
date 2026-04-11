use std::collections::{HashMap, HashSet};

use aws_lambda_events::{apigw::ApiGatewayProxyResponse, http::HeaderMap};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::guest_list::NameSimilaritySearcher;
pub use crate::store::RsvpStore;

const MAXIMUM_LENGTH: usize = 100;

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
pub struct HandlerImpl<S, G> {
    pub store: S,
    pub guest_list: G,
}

impl<S: RsvpStore, G: NameSimilaritySearcher> HandlerImpl<S, G> {
    /// Fuzzy-search guests by name; returns up to 5 matching parties.
    pub fn search(&self, query: &str) -> SearchResult {
        let matches = self
            .guest_list
            .search(query)
            .into_iter()
            .take(5)
            .map(|(_, p)| SearchMatch {
                party_id: p.id.clone(),
                display_name: p.display_name.clone(),
                guest_names: p.guests.iter().map(|g| g.name.clone()).collect(),
            })
            .collect();
        SearchResult { matches }
    }

    /// Return a party's guests along with any previously submitted RSVP data.
    pub async fn get_party(&self, party_id: &str) -> Result<PartyResult, HandlerError> {
        let party = self
            .guest_list
            .get_party(party_id)
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
        let party = self.guest_list.get_party(&input.party_id).ok_or_else(|| {
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

            if let Some(dietary_restrictions) = &response.dietary_restrictions
                && dietary_restrictions.len() > MAXIMUM_LENGTH
            {
                return Err(HandlerError::BadRequest(format!(
                    "Maximum length of dietary restrictions is 100 characters."
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
    use crate::guest_list::{CsvNameSearcher, GuestEntry, PartyEntry};
    use crate::store::RsvpStore;
    use std::collections::HashMap;
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

    // ── Test guest data ───────────────────────────────────────────────────────

    fn test_guest_map() -> HashMap<String, PartyEntry> {
        let mut map = HashMap::new();

        map.insert(
            "p001".to_string(),
            PartyEntry {
                id: "p001".to_string(),
                display_name: "Smith Family".to_string(),
                guests: vec![
                    GuestEntry {
                        name: "John Smith".to_string(),
                        aliases: vec![],
                    },
                    GuestEntry {
                        name: "Jane Smith".to_string(),
                        aliases: vec![],
                    },
                ],
            },
        );

        map.insert(
            "p002".to_string(),
            PartyEntry {
                id: "p002".to_string(),
                display_name: "Doe Family".to_string(),
                guests: vec![GuestEntry {
                    name: "Alice Doe".to_string(),
                    aliases: vec![],
                }],
            },
        );

        map.insert(
            "p003".to_string(),
            PartyEntry {
                id: "p003".to_string(),
                display_name: "Williams Family".to_string(),
                guests: vec![
                    GuestEntry {
                        name: "Emma Williams".to_string(),
                        aliases: vec![],
                    },
                    GuestEntry {
                        name: "Carol Williams".to_string(),
                        aliases: vec![],
                    },
                    GuestEntry {
                        name: "Bob Williams".to_string(),
                        aliases: vec![],
                    },
                ],
            },
        );

        map
    }

    fn test_searcher() -> CsvNameSearcher {
        CsvNameSearcher::new(test_guest_map())
    }

    fn handler() -> HandlerImpl<MockRsvpStore, CsvNameSearcher> {
        HandlerImpl {
            store: MockRsvpStore::new(),
            guest_list: test_searcher(),
        }
    }

    // ── Search tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_search_exact_full_name() {
        let results = handler().search("John Smith").matches;
        assert!(!results.is_empty(), "expected a match for 'John Smith'");
        assert!(results.iter().any(|m| m.party_id == "p001"));
    }

    #[test]
    fn test_search_first_name_only() {
        let results = handler().search("Alice").matches;
        assert!(
            results.iter().any(|m| m.party_id == "p002"),
            "expected p002"
        );
    }

    #[test]
    fn test_search_last_name_only() {
        let results = handler().search("Williams").matches;
        assert!(
            results.iter().any(|m| m.party_id == "p003"),
            "expected p003"
        );
    }

    #[test]
    fn test_search_case_insensitive() {
        assert!(
            handler()
                .search("john smith")
                .matches
                .iter()
                .any(|m| m.party_id == "p001")
        );
        assert!(
            handler()
                .search("JOHN SMITH")
                .matches
                .iter()
                .any(|m| m.party_id == "p001")
        );
    }

    #[test]
    fn test_search_fuzzy_typo() {
        // "Jon Smith" is a one-character typo of "John Smith"
        let results = handler().search("Jon Smith").matches;
        assert!(
            results.iter().any(|m| m.party_id == "p001"),
            "fuzzy search should match 'John Smith' for 'Jon Smith'"
        );
    }

    #[test]
    fn test_search_no_match_gibberish() {
        assert!(handler().search("xqzgibberish").matches.is_empty());
    }

    #[test]
    fn test_search_result_includes_all_party_guests() {
        let results = handler().search("Smith").matches;
        let smith = results.iter().find(|m| m.party_id == "p001").unwrap();
        assert!(smith.guest_names.contains(&"John Smith".to_string()));
        assert!(smith.guest_names.contains(&"Jane Smith".to_string()));
    }

    #[test]
    fn test_search_result_count_capped_at_limit() {
        assert!(handler().search("Smith").matches.len() <= 5);
    }

    #[test]
    fn test_search_results_ordered_by_score() {
        // Exact substring match should rank first
        let results = handler().search("Emma Williams").matches;
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
