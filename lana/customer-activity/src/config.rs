use chrono::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerActivityCheckConfig {
    #[serde(default = "default_activity_check_enabled")]
    pub activity_check_enabled: bool,
    #[serde(default = "default_inactive_threshold_days")]
    pub inactive_threshold_days: Duration,
    #[serde(default = "default_escheatment_threshold_days")]
    pub escheatment_threshold_days: Duration,
    #[serde(default = "default_activity_check_hour")]
    pub activity_check_hour: Duration,
    #[serde(default = "default_activity_check_minute")]
    pub activity_check_minute: Duration,
}

impl Default for CustomerActivityCheckConfig {
    fn default() -> Self {
        Self {
            activity_check_enabled: default_activity_check_enabled(),
            inactive_threshold_days: default_inactive_threshold_days(),
            escheatment_threshold_days: default_escheatment_threshold_days(),
            activity_check_hour: default_activity_check_hour(),
            activity_check_minute: default_activity_check_minute(),
        }
    }
}

fn default_activity_check_enabled() -> bool {
    true
}

fn default_inactive_threshold_days() -> Duration {
    Duration::days(365)
}

fn default_escheatment_threshold_days() -> Duration {
    Duration::days(3650)
}

fn default_activity_check_hour() -> Duration {
    Duration::hours(0)
}

fn default_activity_check_minute() -> Duration {
    Duration::minutes(0)
}
