use std::collections::HashMap;

use thiserror::Error;

use crate::guest_list::ParseCsvError::InvalidCsvError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GuestEntry {
    pub name: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PartyEntry {
    pub id: String,
    pub display_name: String,
    pub guests: Vec<GuestEntry>,
}

pub trait GuestListFactory<G: GuestList> {
    fn build(&self) -> G;
}

// ── GuestList trait ───────────────────────────────────────────────────────────

/// Read-only interface over the backing guest data store.
pub trait GuestList {
    fn get_party(&self, party_id: &str) -> Option<&PartyEntry>;
    fn all_parties(&self) -> Vec<&PartyEntry>;
}

impl<T: GuestList> GuestList for &T {
    fn get_party(&self, party_id: &str) -> Option<&PartyEntry> {
        (*self).get_party(party_id)
    }

    fn all_parties(&self) -> Vec<&PartyEntry> {
        (*self).all_parties()
    }
}

#[derive(Clone)]
pub struct MapGuestList {
    map: HashMap<String, PartyEntry>,
}

impl MapGuestList {
    /// Construct from an existing map (useful for tests).
    pub fn new(map: HashMap<String, PartyEntry>) -> Self {
        Self { map }
    }
}

impl GuestList for MapGuestList {
    fn get_party(&self, party_id: &str) -> Option<&PartyEntry> {
        self.map.get(party_id)
    }

    fn all_parties(&self) -> Vec<&PartyEntry> {
        self.map.values().collect()
    }
}

// Static CSV based guest list

#[derive(Clone)]
pub struct CsvGuestListFactory {
    content: &'static str,
}

impl CsvGuestListFactory {
    pub const fn new(content: &'static str) -> Self {
        Self { content }
    }
}

#[derive(Debug, Error)]
enum ParseCsvError {
    #[error("Found invalid CSV when parsing {0}")]
    InvalidCsvError(String)
}

fn parse_guest_csv(guest_csv_str: String) -> Result<HashMap<String, PartyEntry>, ParseCsvError> {
    let mut map: HashMap<String, PartyEntry> = HashMap::new();
    for line in guest_csv_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(4, ',');
        let (Some(id), Some(display), Some(name), Some(alias)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(InvalidCsvError(line.to_string()));
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
            vec![alias.trim().to_string()]
        };

        entry.guests.push(GuestEntry { name, aliases });
    }

    Ok(map)
}

impl GuestListFactory<MapGuestList> for CsvGuestListFactory {
    /// Initialise from the embedded CSV exactly once and return a static
    /// reference.  This is the entry point for production use.
    fn build(&self) -> MapGuestList {
        let map = parse_guest_csv(self.content.to_string())
            .expect("Invalid guests.csv");
        MapGuestList::new(map)
    }
}

// S3 backed guest list factory

pub struct S3GuestListFactory {
    map: HashMap<String, PartyEntry>,
}

impl S3GuestListFactory {
    pub async fn new(
        s3_client: aws_sdk_s3::Client,
        bucket_name: String,
        object_key: String,
        expected_bucket_owner: String,
    ) -> anyhow::Result<S3GuestListFactory> {
        let key = s3_client
            .get_object()
            .bucket(bucket_name)
            .key(object_key)
            .expected_bucket_owner(expected_bucket_owner)
            .send()
            .await?;
        let bytes = key.body.collect().await?.into_bytes();
        let content = String::from_utf8(bytes.to_vec())?;
        let map = parse_guest_csv(content)?;
        Ok(S3GuestListFactory { map })
    }
}

impl GuestListFactory<MapGuestList> for S3GuestListFactory {
    fn build(&self) -> MapGuestList {
        MapGuestList::new(self.map.clone())
    }
}
