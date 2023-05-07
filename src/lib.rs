pub mod cli;
mod config;
mod utils;

pub use cli::{parse_key_value, KeyValType};
pub use utils::process_error_output;

pub use config::{
    get_status_text, GeoCodingConfig, GeoCodingProfile, LoadConfig, RequestProfile,
    ResponseProfile, ValidateConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtraArgs {
    pub headers: Vec<(String, String)>,
    pub query: Vec<(String, String)>,
    pub body: Vec<(String, String)>,
}

impl IntoIterator for ExtraArgs {
    type Item = (KeyValType, Vec<(String, String)>);
    type IntoIter = std::array::IntoIter<(KeyValType, Vec<(String, String)>), 3>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter([
            (KeyValType::Header, self.headers),
            (KeyValType::Query, self.query),
            (KeyValType::Body, self.body),
        ])
    }
}

impl ExtraArgs {
    pub fn new_with_headers(headers: Vec<(String, String)>) -> Self {
        Self {
            headers,
            ..Default::default()
        }
    }

    pub fn new_with_query(query: Vec<(String, String)>) -> Self {
        Self {
            query,
            ..Default::default()
        }
    }
}
