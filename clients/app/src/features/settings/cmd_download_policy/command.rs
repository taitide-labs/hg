use hg_contracts::offline::DownloadPolicy;

pub const CONCURRENCY_PRESETS: &[(u8, &str)] =
    &[(1, "1 个"), (2, "2 个"), (3, "3 个"), (4, "4 个")];

#[must_use]
pub const fn clamp_concurrency(value: u8) -> u8 {
    if value < 1 {
        1
    } else if value > 4 {
        4
    } else {
        value
    }
}

#[must_use]
pub fn toggle_wifi_only(current: &DownloadPolicy) -> DownloadPolicy {
    DownloadPolicy {
        wifi_only: !current.wifi_only,
        ..current.clone()
    }
}

#[must_use]
pub fn with_max_concurrent_tasks(
    current: &DownloadPolicy,
    max_concurrent_tasks: u8,
) -> DownloadPolicy {
    DownloadPolicy {
        max_concurrent_tasks: clamp_concurrency(max_concurrent_tasks),
        ..current.clone()
    }
}
