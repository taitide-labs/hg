use nsh_core::prelude::*;

use super::super::store;

pub fn download_policy_section() -> impl IntoElement {
    let empty = store::state().with(|s| s.download_policy.is_none());
    view().children(empty.then(view))
}
