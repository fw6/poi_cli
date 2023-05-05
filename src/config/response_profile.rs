use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct ResponseProfile {
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub pick_results: HashMap<String, String>,
}

impl ResponseProfile {
    pub fn new(pick_results: HashMap<String, String>) -> Self {
        Self { pick_results }
    }
}
