use nsh_core::prelude::*;

use super::super::store;

pub fn playback_pref_section() -> impl IntoElement {
    let empty = store::state().with(|s| s.playback_pref.is_none() && s.error.is_none());
    view().children(empty.then(view))
}
