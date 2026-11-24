use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogFeatureFlags {
    pub discovery: bool,
    pub detail_and_comments: bool,
    pub playback_and_cache: bool,
}

impl CatalogFeatureFlags {
    #[must_use]
    pub const fn all_enabled() -> Self {
        Self {
            discovery: true,
            detail_and_comments: true,
            playback_and_cache: true,
        }
    }
}
