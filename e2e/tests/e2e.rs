#[cfg(test)]
#[path = "e2e/discovery_and_playback.rs"]
mod discovery_and_playback;
#[cfg(test)]
#[path = "e2e/fake_upstream.rs"]
mod fake_upstream;
#[cfg(test)]
#[path = "e2e/harness.rs"]
mod harness;
#[cfg(test)]
#[path = "e2e/live_smoke.rs"]
mod live_smoke;
#[cfg(test)]
#[path = "e2e/offline_download_and_halt.rs"]
mod offline_download_and_halt;
#[cfg(test)]
#[path = "e2e/resilience_and_persistence.rs"]
mod resilience_and_persistence;
#[cfg(test)]
#[path = "e2e/settings_and_cache.rs"]
mod settings_and_cache;
