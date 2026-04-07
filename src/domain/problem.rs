use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemMetadata {
    pub id: String,
    pub title: String,
    pub difficulty: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub url: String,
    pub fetched_at: DateTime<FixedOffset>,
}
