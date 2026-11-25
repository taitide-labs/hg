use std::time::Duration;

use nsh_base::{
    Anchor, ToastManager, ToastMotion, ToastOptions, ToastStackState, ToastTransitionStatus, toast,
    toast_stack,
};
use nsh_core::{prelude::*, time::now};

use crate::ui::controls::bind_interval;
use crate::ui::token;

const TOAST_TICK_MS: Duration = Duration::from_millis(50);
const TOAST_DURATION: Duration = Duration::from_millis(2200);

#[derive(Clone)]
struct ToastStore {
    manager: ToastManager<u64, SharedString>,
    next_id: u64,
}

impl Default for ToastStore {
    fn default() -> Self {
        Self {
            manager: ToastManager::new(ToastMotion::sonner()),
            next_id: 1,
        }
    }
}

#[derive(Clone, Default)]
struct VisibleToasts(Vec<(u64, SharedString, ToastTransitionStatus)>);

fn manager_state() -> Signal<ToastStore> {
    global::<ToastStore>()
}

fn visible_state() -> Signal<VisibleToasts> {
    global::<VisibleToasts>()
}

fn sync_visible_toasts() {
    let entries = manager_state().with_untracked(|s| {
        s.manager
            .visible(3)
            .map(|(id, msg, status)| (*id, msg.clone(), status))
            .collect::<Vec<_>>()
    });
    visible_state().set(VisibleToasts(entries));
}

pub fn show_toast(message: impl Into<SharedString>) {
    let message: SharedString = message.into();
    let instant = now();
    manager_state().update(|s| {
        let id = s.next_id;
        s.next_id = s.next_id.wrapping_add(1).max(1);
        s.manager.push(
            id,
            message,
            ToastOptions::timeout(Some(TOAST_DURATION)),
            instant,
        );
    });
    sync_visible_toasts();
}

pub fn toast_host() -> impl IntoElement {
    let stack_state = ToastStackState::new();
    let has_active = computed(|| visible_state().with(|list| !list.0.is_empty()));

    bind_interval(
        TOAST_TICK_MS,
        move || has_active.get(),
        move || {
            let expanded = stack_state.is_expanded();
            let mut changed = false;
            manager_state().update(|s| {
                changed = s.manager.advance(now(), expanded).changed;
            });
            if changed {
                sync_visible_toasts();
            }
        },
    );

    let items = computed(|| visible_state().with(|list| list.0.clone()));
    overlay()
        .inset_0()
        .justify_end()
        .items_center()
        .pb_6()
        .pointer_events(PointerEvents::None)
        .child(
            toast_stack()
                .placement(Anchor::BottomCenter)
                .state(stack_state)
                .w(px(300.0))
                .child(each(
                    items,
                    |(id, _, _)| *id,
                    |item, _| {
                        let toast_id = item.with_untracked(|(id, _, _)| *id);
                        let label = computed(move || item.with(|(_, msg, _)| msg.clone()));
                        toast()
                            .transition_status(move || item.with(|(_, _, status)| *status))
                            .accessibility_label(move || Some(label.get()))
                            .on_dismiss_request(move || {
                                manager_state().update(|s| {
                                    s.manager.dismiss(&toast_id, now());
                                });
                                sync_visible_toasts();
                            })
                            .px_3p5()
                            .py_2()
                            .rounded(token::radius_card)
                            .bg(token::popover)
                            .border_1()
                            .border_color(token::border)
                            .items_center()
                            .child(
                                text(move || label.get())
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(token::popover_fg),
                            )
                    },
                )),
        )
}
