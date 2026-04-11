use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::problem::ProblemProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemMetadata {
    pub id: String,
    #[serde(default)]
    pub provider: ProblemProvider,
    pub title: String,
    pub difficulty: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub source: Option<String>,
    #[serde(default)]
    pub contest: Option<String>,
    pub url: String,
    pub fetched_at: DateTime<FixedOffset>,
}
