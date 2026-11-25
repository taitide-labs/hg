use std::rc::Rc;

use nsh_base::{HandleEdge, SheetEvent, button, resize_handle, sheet, switch};
use nsh_core::geometry::Axis;
use nsh_core::prelude::*;

use crate::ui::icons::{IconKind, icon};
use crate::ui::token;

pub fn toggle_switch(
    checked: impl Fn() -> bool + Copy + 'static,
    a11y_label: &'static str,
    on_toggle: impl Fn() + 'static,
) -> impl IntoElement {
    switch()
        .checked(Prop::bind(checked))
        .accessibility_label(a11y_label)
        .hit_slop(EdgeInsets::all(px(6.0)))
        .track_style(move |s| {
            let track_bg = if checked() {
                token::primary()
            } else {
                token::border()
            };
            s.w(px(36.0))
                .h(px(20.0))
                .p_0p5()
                .rounded_full()
                .flex_row()
                .items_center()
                .bg(track_bg)
                .transition_colors()
                .duration(token::INSTANT_MS)
        })
        .thumb_style(move |s| {
            let offset = if checked() { px(16.0) } else { px(0.0) };
            s.size(px(16.0))
                .rounded_full()
                .bg(token::primary_fg())
                .translate_x(offset)
                .transition_transform()
                .duration(token::INSTANT_MS)
        })
        .on_change(move |_: &bool| on_toggle())
}

pub fn setting_row(
    title: &'static str,
    description: &'static str,
    control: impl IntoElement,
) -> impl IntoElement {
    view()
        .w_full()
        .min_h(px(44.0))
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            view()
                .flex_1()
                .min_w(px(200.0))
                .gap_0p5()
                .child(
                    text(title)
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(token::card_fg),
                )
                .child(text(description).text_xs().text_color(token::muted_fg)),
        )
        .child(
            view()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .child(control),
        )
}

pub fn sidebar_resize_handle(
    on_drag_move: impl Fn(f32) + 'static,
    on_drag_end: impl Fn(f32) + 'static,
) -> impl IntoElement {
    resize_handle()
        .axis(Axis::Horizontal)
        .edge(HandleEdge::Trailing)
        .accessibility_label("调整侧边栏宽度")
        .on_drag_move(move |by: &Points| on_drag_move(by.0))
        .on_drag_end(move |by: &Points| on_drag_end(by.0))
        .child(
            view()
                .w(px(2.0))
                .h_full()
                .transition_colors()
                .duration(token::INSTANT_MS)
                .group_hover(|s| s.bg(token::primary))
                .group_active(|s| s.bg(token::primary)),
        )
}

pub fn mobile_nav_drawer(
    open: impl Fn() -> bool + Copy + 'static,
    on_open_change: impl Fn(bool) + 'static,
    content: impl IntoElement,
) -> impl IntoElement {
    let on_open_change = Rc::new(on_open_change);
    sheet()
        .open(Prop::bind(move || Some(open())))
        .side(Placement::Left)
        .close_on_escape(true)
        .close_on_backdrop_press(true)
        .accessibility_label("导航菜单")
        .backdrop_style(|s| s.bg(token::cinema_scrim().opacity(0.45)))
        .popup_style(|s| {
            s.w(px(264.0))
                .h_full()
                .bg(token::sidebar())
                .border_r_1()
                .border_color(token::sidebar_border())
                .pt_safe_area_plus(px(12.0))
                .pb_safe_area_plus(px(12.0))
                .px_3()
        })
        .on_open_change(move |ev: &SheetEvent| {
            if let SheetEvent::OpenChange(next, _) = ev {
                on_open_change(*next);
            }
        })
        .child(content)
}

pub fn submit_button(
    label: &'static str,
    a11y_label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .accessibility_label(a11y_label)
        .h(px(34.0))
        .px(px(14.0))
        .flex_row()
        .items_center()
        .justify_center()
        .rounded(token::radius_control)
        .bg(token::primary)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(move || token::primary().opacity(0.9)))
        .active(|s| {
            let s = s.opacity(0.88);
            if reduce_motion() {
                s
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(token::primary_fg),
        )
}

pub fn sidebar_toggle_button(
    collapsed: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .accessibility_label(Prop::bind(move || {
            Some(SharedString::from(if collapsed() {
                "展开侧边栏"
            } else {
                "收起侧边栏"
            }))
        }))
        .size(px(28.0))
        .items_center()
        .justify_center()
        .rounded(token::radius_control)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(token::accent))
        .active(|s| {
            let s = s.opacity(0.88);
            if reduce_motion() {
                s
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(icon(IconKind::PanelLeft, 14.0, token::fg))
}

pub fn tag_chip_button(
    label: String,
    active: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let active = Rc::new(active);
    let active_bg = Rc::clone(&active);
    let active_hover = Rc::clone(&active);
    let active_fg = active;
    button()
        .on_press(on_press)
        .h(px(28.0))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .justify_center()
        .rounded(token::radius_control)
        .bg(move || {
            if active_bg() {
                token::primary()
            } else {
                token::secondary()
            }
        })
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(move |s| {
            let active_hover = Rc::clone(&active_hover);
            s.bg(move || {
                if active_hover() {
                    token::primary().opacity(0.9)
                } else {
                    token::accent()
                }
            })
        })
        .active(|s| {
            let s = s.opacity(0.88);
            if reduce_motion() {
                s
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || {
                    if active_fg() {
                        token::primary_fg()
                    } else {
                        token::fg()
                    }
                }),
        )
}

pub fn chip_remove_button(a11y_label: String, on_press: impl Fn() + 'static) -> impl IntoElement {
    button()
        .on_press(on_press)
        .accessibility_label(a11y_label)
        .size(px(20.0))
        .items_center()
        .justify_center()
        .rounded_full()
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(token::accent))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(icon(IconKind::Close, 11.0, token::muted_fg))
}
