use hg_contracts::offline::DownloadPolicy;
use nsh_core::prelude::*;

use super::super::store;
use super::command;
use crate::gateway;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct PolicySaveGen(u64);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PolicyError(Option<String>);

#[must_use]
pub fn selected_concurrency() -> u8 {
    store::state().with(|s| {
        s.download_policy
            .as_ref()
            .map_or(2, |p| command::clamp_concurrency(p.max_concurrent_tasks))
    })
}

#[must_use]
pub fn error_message() -> Option<String> {
    global::<PolicyError>().with(|e| e.0.clone())
}

pub fn load_download_policy() {
    let gw = gateway::current();
    spawn(async move {
        match gw.download_policy().await {
            Ok(policy) => {
                global::<PolicyError>().set(PolicyError(None));
                store::state().update(|s| {
                    s.download_policy = Some(policy);
                });
            }
            Err(err) => {
                global::<PolicyError>().set(PolicyError(Some(err.to_string())));
            }
        }
    });
}

pub fn toggle_wifi_only() {
    let Some(current) = store::state().with_untracked(|s| s.download_policy.clone()) else {
        return;
    };
    let next = command::toggle_wifi_only(&current);
    save_policy(current, next);
}

pub fn select_max_concurrent(count: u8) {
    let Some(current) = store::state().with_untracked(|s| s.download_policy.clone()) else {
        return;
    };
    let next = command::with_max_concurrent_tasks(&current, count);
    save_policy(current, next);
}

fn save_policy(previous: DownloadPolicy, next: DownloadPolicy) {
    let seq = next_save_generation();
    global::<PolicyError>().set(PolicyError(None));
    store::state().update(|s| {
        s.download_policy = Some(next.clone());
    });

    let gw = gateway::current();
    spawn(async move {
        if let Err(err) = gw.update_download_policy(next).await
            && is_latest_generation(seq)
        {
            global::<PolicyError>().set(PolicyError(Some(err.to_string())));
            store::state().update(|s| {
                s.download_policy = Some(previous);
            });
        }
    });
}

fn next_save_generation() -> u64 {
    let mut next = 0;
    global::<PolicySaveGen>().update(|g| {
        g.0 = g.0.saturating_add(1);
        next = g.0;
    });
    next
}

fn is_latest_generation(seq: u64) -> bool {
    global::<PolicySaveGen>().with_untracked(|g| g.0 == seq)
}
