use std::collections::HashMap;

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

pub trait GuestListFactory<G: GuestList> {
    fn new(&self) -> G;
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

impl GuestListFactory<MapGuestList> for CsvGuestListFactory {
    /// Initialise from the embedded CSV exactly once and return a static
    /// reference.  This is the entry point for production use.
    fn new(&self) -> MapGuestList {
        let mut map: HashMap<String, PartyEntry> = HashMap::new();
        for line in self.content.lines() {
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
        MapGuestList::new(map)
    }
}
