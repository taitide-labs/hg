use nsh_core::prelude::*;

use super::super::store;

pub fn storage_section<A>(_clear_action: impl Fn() -> A + Copy + 'static) -> impl IntoElement
where
    A: IntoElement + 'static,
{
    let empty = store::state().with(|s| s.storage.is_none());
    view().children(empty.then(view))
}
