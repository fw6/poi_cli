use std::collections::HashMap;

use crate::utils::write2csv;
use crate::ExtraArgs;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde::Serialize;

use super::response_profile::ResponseProfile;
use super::LoadConfig;
use super::ValidateConfig;
use super::{is_default, RequestProfile};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeoCodingConfig {
    #[serde(flatten)]
    pub profiles: HashMap<String, GeoCodingProfile>,
}

impl LoadConfig for GeoCodingConfig {}
impl ValidateConfig for GeoCodingConfig {
    fn validate(&self) -> Result<()> {
        for (name, profile) in &self.profiles {
            profile
                .validate()
                .context(format!("failed to validate profile: {}", name))?;
        }
        Ok(())
    }
}

impl GeoCodingConfig {
    pub fn new(profiles: HashMap<String, GeoCodingProfile>) -> Self {
        Self { profiles }
    }

    pub fn get_profile(&self, name: &str) -> Option<&GeoCodingProfile> {
        self.profiles.get(name)
    }
}

impl GeoCodingProfile {
    pub async fn query(&self, args: ExtraArgs, path: &Option<String>) -> Result<String> {
        let res = self.req.send(&args).await?;

        let result = res.get_results(&self.res).await?;

        write2csv(result, path)
    }
}

impl ValidateConfig for GeoCodingProfile {
    fn validate(&self) -> Result<()> {
        self.req.validate().context("req failed to validate")?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoCodingProfile {
    pub req: RequestProfile,

    #[serde(skip_serializing_if = "is_default", default)]
    pub res: ResponseProfile,
}

impl GeoCodingProfile {
    pub fn new(req: RequestProfile, res: ResponseProfile) -> Self {
        Self { req, res }
    }
}
