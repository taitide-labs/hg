use std::f32::consts::TAU;

use nsh_core::canvas::{LineCap, LineJoin};
use nsh_core::geometry::{Bounds, point, size};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Discover,
    Bookmark,
    BookmarkFilled,
    Download,
    Settings,
    Search,
    Menu,
    PanelLeft,
    Play,
    Pause,
    Volume,
    VolumeMuted,
    Expand,
    Smartphone,
    Monitor,
    ExternalLink,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronDown,
    Heart,
    Message,
    List,
    Flame,
    Clock,
    Close,
    Plus,
    Minus,
    ThumbsDown,
}

fn polyline(p: Painter, pts: &[(f32, f32)]) -> Painter {
    let Some((first, rest)) = pts.split_first() else {
        return p;
    };
    let mut p = p.begin_path().move_to(first.0, first.1);
    for &(x, y) in rest {
        p = p.line_to(x, y);
    }
    p.stroke()
}

fn box_rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Points> {
    Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
}

fn rounded_box(p: Painter, x: f32, y: f32, w: f32, h: f32, r: f32) -> Painter {
    p.begin_path()
        .round_rect(box_rect(x, y, w, h), px(r))
        .stroke()
}

fn ring_circle(p: Painter, x: f32, y: f32, r: f32, filled: bool) -> Painter {
    let p = p.begin_path().arc(x, y, r, 0.0, TAU);
    if filled { p.fill() } else { p.stroke() }
}

fn draw_discover(p: Painter) -> Painter {
    let p = ring_circle(p, 10.0, 10.0, 7.2, false);
    polyline(
        p,
        &[
            (7.5, 12.5),
            (9.0, 9.0),
            (12.5, 7.5),
            (11.0, 11.0),
            (7.5, 12.5),
        ],
    )
}

fn draw_bookmark(p: Painter) -> Painter {
    polyline(
        p,
        &[
            (5.5, 3.5),
            (14.5, 3.5),
            (14.5, 16.5),
            (10.0, 13.5),
            (5.5, 16.5),
            (5.5, 3.5),
        ],
    )
}

fn draw_bookmark_filled(p: Painter) -> Painter {
    p.begin_path()
        .move_to(5.5, 3.5)
        .line_to(14.5, 3.5)
        .line_to(14.5, 16.5)
        .line_to(10.0, 13.5)
        .line_to(5.5, 16.5)
        .close_path()
        .fill()
}

fn draw_menu(p: Painter) -> Painter {
    let p = polyline(p, &[(3.5, 5.5), (16.5, 5.5)]);
    let p = polyline(p, &[(3.5, 10.0), (16.5, 10.0)]);
    polyline(p, &[(3.5, 14.5), (16.5, 14.5)])
}

fn draw_panel_left(p: Painter) -> Painter {
    let p = rounded_box(p, 3.0, 3.5, 14.0, 13.0, 2.2);
    polyline(p, &[(8.0, 3.5), (8.0, 16.5)])
}

fn draw_download(p: Painter) -> Painter {
    let p = polyline(p, &[(10.0, 3.5), (10.0, 12.5)]);
    let p = polyline(p, &[(6.8, 9.5), (10.0, 12.8), (13.2, 9.5)]);
    polyline(p, &[(4.0, 14.0), (4.0, 16.2), (16.0, 16.2), (16.0, 14.0)])
}

fn draw_settings(p: Painter) -> Painter {
    let p = ring_circle(p, 10.0, 10.0, 2.8, false);
    let p = ring_circle(p, 10.0, 10.0, 6.4, false);
    let p = polyline(p, &[(10.0, 2.2), (10.0, 3.6)]);
    let p = polyline(p, &[(10.0, 16.4), (10.0, 17.8)]);
    let p = polyline(p, &[(2.2, 10.0), (3.6, 10.0)]);
    polyline(p, &[(16.4, 10.0), (17.8, 10.0)])
}

fn draw_volume(p: Painter, muted: bool) -> Painter {
    let p = polyline(
        p,
        &[
            (3.5, 8.0),
            (6.8, 8.0),
            (10.5, 4.8),
            (10.5, 15.2),
            (6.8, 12.0),
            (3.5, 12.0),
            (3.5, 8.0),
        ],
    );
    if muted {
        let p = polyline(p, &[(13.5, 7.5), (17.5, 12.5)]);
        polyline(p, &[(17.5, 7.5), (13.5, 12.5)])
    } else {
        let p = polyline(p, &[(13.5, 7.6), (14.8, 10.0), (13.5, 12.4)]);
        polyline(p, &[(15.8, 6.0), (17.4, 10.0), (15.8, 14.0)])
    }
}

fn draw_expand(p: Painter) -> Painter {
    let p = polyline(p, &[(3.5, 7.5), (3.5, 3.5), (7.5, 3.5)]);
    let p = polyline(p, &[(12.5, 3.5), (16.5, 3.5), (16.5, 7.5)]);
    let p = polyline(p, &[(16.5, 12.5), (16.5, 16.5), (12.5, 16.5)]);
    polyline(p, &[(7.5, 16.5), (3.5, 16.5), (3.5, 12.5)])
}

fn draw_external(p: Painter) -> Painter {
    let p = polyline(p, &[(11.0, 4.0), (16.0, 4.0), (16.0, 9.0)]);
    let p = polyline(p, &[(16.0, 4.0), (9.5, 10.5)]);
    polyline(
        p,
        &[
            (14.0, 12.0),
            (14.0, 16.0),
            (4.0, 16.0),
            (4.0, 6.0),
            (8.0, 6.0),
        ],
    )
}

fn draw_list(p: Painter) -> Painter {
    let p = polyline(p, &[(7.5, 5.5), (16.5, 5.5)]);
    let p = polyline(p, &[(7.5, 10.0), (16.5, 10.0)]);
    let p = polyline(p, &[(7.5, 14.5), (16.5, 14.5)]);
    let p = ring_circle(p, 4.2, 5.5, 1.0, true);
    let p = ring_circle(p, 4.2, 10.0, 1.0, true);
    ring_circle(p, 4.2, 14.5, 1.0, true)
}

fn draw_heart(p: Painter) -> Painter {
    polyline(
        p,
        &[
            (10.0, 16.2),
            (4.5, 11.0),
            (4.0, 7.8),
            (6.2, 5.2),
            (8.8, 5.8),
            (10.0, 7.4),
            (11.2, 5.8),
            (13.8, 5.2),
            (16.0, 7.8),
            (15.5, 11.0),
            (10.0, 16.2),
        ],
    )
}

fn draw_flame(p: Painter) -> Painter {
    polyline(
        p,
        &[
            (10.0, 3.0),
            (14.2, 8.2),
            (14.5, 12.8),
            (10.0, 16.8),
            (5.5, 12.8),
            (6.0, 8.8),
            (8.5, 10.2),
            (10.0, 3.0),
        ],
    )
}

fn draw_glyph(kind: IconKind, p: Painter) -> Painter {
    match kind {
        IconKind::Discover => draw_discover(p),
        IconKind::Bookmark => draw_bookmark(p),
        IconKind::BookmarkFilled => draw_bookmark_filled(p),
        IconKind::Download => draw_download(p),
        IconKind::Settings => draw_settings(p),
        IconKind::Search => polyline(
            ring_circle(p, 9.0, 9.0, 5.2, false),
            &[(13.0, 13.0), (16.8, 16.8)],
        ),
        IconKind::Menu => draw_menu(p),
        IconKind::PanelLeft => draw_panel_left(p),
        IconKind::Play => p
            .begin_path()
            .move_to(6.5, 4.5)
            .line_to(16.0, 10.0)
            .line_to(6.5, 15.5)
            .close_path()
            .fill(),
        IconKind::Pause => {
            let p = p
                .begin_path()
                .round_rect(box_rect(5.5, 4.5, 3.2, 11.0), px(1.0))
                .fill();
            p.begin_path()
                .round_rect(box_rect(11.3, 4.5, 3.2, 11.0), px(1.0))
                .fill()
        }
        IconKind::Volume => draw_volume(p, false),
        IconKind::VolumeMuted => draw_volume(p, true),
        IconKind::Expand => draw_expand(p),
        IconKind::Smartphone => polyline(
            rounded_box(p, 5.5, 2.5, 9.0, 15.0, 2.2),
            &[(9.0, 15.0), (11.0, 15.0)],
        ),
        IconKind::Monitor => {
            let p = rounded_box(p, 2.5, 3.5, 15.0, 10.0, 2.0);
            let p = polyline(p, &[(10.0, 13.5), (10.0, 16.5)]);
            polyline(p, &[(6.8, 16.5), (13.2, 16.5)])
        }
        IconKind::ExternalLink => draw_external(p),
        IconKind::ChevronLeft => polyline(p, &[(12.5, 5.0), (7.5, 10.0), (12.5, 15.0)]),
        IconKind::ChevronRight => polyline(p, &[(7.5, 5.0), (12.5, 10.0), (7.5, 15.0)]),
        IconKind::ChevronUp => polyline(p, &[(5.0, 12.5), (10.0, 7.5), (15.0, 12.5)]),
        IconKind::ChevronDown => polyline(p, &[(5.0, 7.5), (10.0, 12.5), (15.0, 7.5)]),
        IconKind::Heart => draw_heart(p),
        IconKind::Message => polyline(
            rounded_box(p, 3.0, 4.0, 14.0, 10.5, 2.5),
            &[(6.5, 14.5), (5.5, 17.2), (9.5, 14.5)],
        ),
        IconKind::List => draw_list(p),
        IconKind::Flame => draw_flame(p),
        IconKind::Clock => polyline(
            ring_circle(p, 10.0, 10.0, 6.8, false),
            &[(10.0, 6.2), (10.0, 10.2), (13.0, 12.0)],
        ),
        IconKind::Close => {
            let p = polyline(p, &[(5.5, 5.5), (14.5, 14.5)]);
            polyline(p, &[(14.5, 5.5), (5.5, 14.5)])
        }
        IconKind::Plus => {
            let p = polyline(p, &[(10.0, 5.0), (10.0, 15.0)]);
            polyline(p, &[(5.0, 10.0), (15.0, 10.0)])
        }
        IconKind::Minus => polyline(p, &[(5.0, 10.0), (15.0, 10.0)]),
        IconKind::ThumbsDown => {
            let p = rounded_box(p, 13.5, 4.0, 3.0, 7.5, 1.0);
            polyline(
                p,
                &[
                    (13.5, 4.5),
                    (6.5, 4.5),
                    (4.5, 10.5),
                    (9.0, 10.5),
                    (8.2, 16.0),
                    (10.2, 16.0),
                    (13.5, 11.0),
                ],
            )
        }
    }
}

pub fn icon(
    kind: IconKind,
    size_pt: f32,
    tint: impl Fn() -> AdaptiveColor + 'static,
) -> impl IntoElement {
    view()
        .size(px(size_pt))
        .pointer_events(PointerEvents::None)
        .child(
            canvas(move || {
                let stroke = color(tint());
                let scale = size_pt / 20.0;
                let painter = Painter::new()
                    .scale(scale, scale)
                    .stroke_color(stroke)
                    .fill_color(stroke)
                    .line_width(1.6)
                    .line_cap(LineCap::Round)
                    .line_join(LineJoin::Round);
                draw_glyph(kind, painter)
            })
            .size_full(),
        )
}
