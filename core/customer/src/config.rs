use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustomerConfig {
    #[serde(default = "default_inactivity_threshold_days")]
    pub inactivity_threshold_days: u32,
    #[serde(default = "default_escheatment_threshold_days")]
    pub escheatment_threshold_days: u32,
}

impl Default for CustomerConfig {
    fn default() -> Self {
        Self {
            inactivity_threshold_days: default_inactivity_threshold_days(),
            escheatment_threshold_days: default_escheatment_threshold_days(),
        }
    }
}

fn default_inactivity_threshold_days() -> u32 {
    365
}

fn default_escheatment_threshold_days() -> u32 {
    3650
}
