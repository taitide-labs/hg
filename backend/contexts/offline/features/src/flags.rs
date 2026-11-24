use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineFeatureFlags {
    pub download_queue: bool,
    pub library_and_outbox: bool,
}

impl OfflineFeatureFlags {
    #[must_use]
    pub const fn all_enabled() -> Self {
        Self {
            download_queue: true,
            library_and_outbox: true,
        }
    }
}
