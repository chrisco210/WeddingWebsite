use std::collections::HashMap;

use thiserror::Error;

use xxhash_rust::xxh3::xxh3_64;

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
    pub welcome_dinner_invite: bool,
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

#[derive(Debug, Error)]
enum ParseCsvError {
    #[error("Found invalid CSV when parsing {0}")]
    InvalidCsvError(String),
}

// Parses a guest list of the format party_display_name,guest_name,guest_alias
fn parse_guest_csv(
    guest_csv_str: String,
    welcome_party_list: &Vec<String>,
) -> Result<HashMap<String, PartyEntry>, ParseCsvError> {
    let mut map: HashMap<String, PartyEntry> = HashMap::new();
    for line in guest_csv_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(4, ',');
        let (Some(display), Some(name), Some(alias)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(InvalidCsvError(line.to_string()));
        };

        let display = display.trim();
        let name = name.trim();
        let alias = alias.trim();

        let id = xxh3_64(display.as_bytes()).to_string();

        let display = display.trim().to_string();
        let name = name.trim().to_string();
        let entry = map.entry(id.clone()).or_insert_with(|| {
            let welcome_dinner = welcome_party_list.contains(&display);
            PartyEntry {
                id,
                display_name: display,
                guests: Vec::new(),
                welcome_dinner_invite: welcome_dinner,
            }
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

// S3 backed guest list factory

pub struct S3GuestListFactory {
    map: HashMap<String, PartyEntry>,
}

async fn read_object_to_str(
    s3_client: &aws_sdk_s3::Client,
    bucket_name: &String,
    object_key: &String,
    expected_bucket_owner: &String,
) -> anyhow::Result<String> {
    let key = s3_client
        .get_object()
        .bucket(bucket_name)
        .key(object_key)
        .expected_bucket_owner(expected_bucket_owner)
        .send()
        .await?;
    let bytes = key.body.collect().await?.into_bytes();
    let result = String::from_utf8(bytes.to_vec())?;

    Ok(result)
}

impl S3GuestListFactory {
    pub async fn new(
        s3_client: aws_sdk_s3::Client,
        bucket_name: String,
        expected_bucket_owner: String,
        guest_list_key: String,
        welcome_dinner_key: String,
    ) -> anyhow::Result<S3GuestListFactory> {
        let guest_list = read_object_to_str(
            &s3_client,
            &bucket_name,
            &guest_list_key,
            &expected_bucket_owner,
        )
        .await?;

        let welcome_dinner = read_object_to_str(
            &s3_client,
            &bucket_name,
            &welcome_dinner_key,
            &expected_bucket_owner,
        )
        .await?
        .split('\n')
        .map(String::from)
        .collect();

        let map = parse_guest_csv(guest_list, &welcome_dinner)?;

        Ok(S3GuestListFactory { map })
    }
}

impl GuestListFactory<MapGuestList> for S3GuestListFactory {
    fn build(&self) -> MapGuestList {
        MapGuestList::new(self.map.clone())
    }
}
