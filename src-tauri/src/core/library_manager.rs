use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LibraryRecord {
    pub id: String,
    pub name: String,
    pub url: String,
    pub library_type: String,
    pub asset_types: String,
    pub trusted: bool,
    pub last_indexed_at: Option<i64>,
    pub item_count: Option<i64>,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LibraryItemRecord {
    pub id: String,
    pub library_id: String,
    pub asset_type: String,
    pub name: String,
    pub description: Option<String>,
    pub subpath: Option<String>,
    pub metadata_json: Option<String>,
    pub indexed_at: i64,
}

#[cfg(test)]
#[path = "tests/library_manager.rs"]
mod tests;
