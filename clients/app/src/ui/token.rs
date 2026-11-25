use nsh_base::theme;
use nsh_core::prelude::*;

pub const INSTANT_MS: f32 = 120.0;
pub const FAST_MS: f32 = 180.0;
pub const BASE_MS: f32 = 240.0;
pub const SLOW_MS: f32 = 320.0;
pub const STAGGER_STEP_MS: f32 = 40.0;
pub const STAGGER_MAX_INDEX: usize = 7;
pub const PRESS_SCALE: f32 = 0.97;
pub const ICON_PRESS_SCALE: f32 = 0.95;

fn color_or(name: &str, fallback: AdaptiveColor) -> AdaptiveColor {
    theme().color(name).unwrap_or(fallback)
}

#[must_use]
pub fn bg() -> AdaptiveColor {
    theme().background
}

#[must_use]
pub fn fg() -> AdaptiveColor {
    theme().foreground
}

#[must_use]
pub fn card() -> AdaptiveColor {
    theme().card
}

#[must_use]
pub fn card_fg() -> AdaptiveColor {
    theme().card_foreground
}

#[must_use]
pub fn primary() -> AdaptiveColor {
    theme().primary
}

#[must_use]
pub fn primary_fg() -> AdaptiveColor {
    theme().primary_foreground
}

#[must_use]
pub fn secondary() -> AdaptiveColor {
    theme().secondary
}

#[must_use]
pub fn secondary_fg() -> AdaptiveColor {
    theme().secondary_foreground
}

#[must_use]
pub fn muted() -> AdaptiveColor {
    theme().muted
}

#[must_use]
pub fn muted_fg() -> AdaptiveColor {
    theme().muted_foreground
}

#[must_use]
pub fn accent() -> AdaptiveColor {
    theme().accent
}

#[must_use]
pub fn accent_fg() -> AdaptiveColor {
    theme().accent_foreground
}

#[must_use]
pub fn border() -> AdaptiveColor {
    theme().border
}

#[must_use]
pub fn input_border() -> AdaptiveColor {
    theme().input
}

#[must_use]
pub fn ring() -> AdaptiveColor {
    theme().ring
}

#[must_use]
pub fn destructive() -> AdaptiveColor {
    theme().destructive
}

#[must_use]
pub fn sidebar() -> AdaptiveColor {
    theme().sidebar
}

#[must_use]
pub fn sidebar_fg() -> AdaptiveColor {
    theme().sidebar_foreground
}

#[must_use]
pub fn sidebar_accent() -> AdaptiveColor {
    theme().sidebar_accent
}

#[must_use]
pub fn sidebar_border() -> AdaptiveColor {
    theme().sidebar_border
}

#[must_use]
pub fn brand_soft() -> AdaptiveColor {
    color_or("brand-soft", theme().primary.opacity(0.14))
}

#[must_use]
pub fn brand_soft_fg() -> AdaptiveColor {
    color_or("brand-soft-foreground", theme().primary)
}

#[must_use]
pub fn success() -> AdaptiveColor {
    color_or("success", theme().chart_2)
}

#[must_use]
pub fn success_soft() -> AdaptiveColor {
    color_or("success-soft", theme().muted)
}

#[must_use]
pub fn warning() -> AdaptiveColor {
    color_or("warning", theme().chart_4)
}

#[must_use]
pub fn warning_soft() -> AdaptiveColor {
    color_or("warning-soft", theme().muted)
}

#[must_use]
pub fn surface_subtle() -> AdaptiveColor {
    color_or("surface-subtle", theme().muted)
}

#[must_use]
pub fn cinema_bg() -> AdaptiveColor {
    color_or("cinema-bg", theme().foreground)
}

#[must_use]
pub fn cinema_surface() -> AdaptiveColor {
    color_or("cinema-surface", theme().foreground)
}

#[must_use]
pub fn cinema_surface_elevated() -> AdaptiveColor {
    color_or("cinema-surface-elevated", theme().muted)
}

#[must_use]
pub fn cinema_border() -> AdaptiveColor {
    color_or("cinema-border", theme().border)
}

#[must_use]
pub fn cinema_fg() -> AdaptiveColor {
    color_or("cinema-fg", theme().primary_foreground)
}

#[must_use]
pub fn cinema_muted() -> AdaptiveColor {
    color_or("cinema-muted", theme().muted_foreground)
}

#[must_use]
pub fn cinema_scrim() -> AdaptiveColor {
    color_or("cinema-scrim", theme().foreground)
}

#[must_use]
pub fn cinema_success_bg() -> AdaptiveColor {
    color_or("cinema-success-bg", theme().muted)
}

#[must_use]
pub fn cinema_success_fg() -> AdaptiveColor {
    color_or("cinema-success-fg", theme().chart_2)
}

#[must_use]
pub fn cinema_brand_bg() -> AdaptiveColor {
    color_or("cinema-brand-bg", theme().muted)
}

#[must_use]
pub fn cinema_brand_fg() -> AdaptiveColor {
    color_or("cinema-brand-fg", theme().primary)
}

#[must_use]
pub fn poster_surface(cover_url: &str) -> AdaptiveColor {
    let key = match cover_url {
        "poster://amber-noir" => "poster-amber",
        "poster://crimson-ink" => "poster-crimson",
        "poster://slate-gold" => "poster-slate",
        "poster://indigo-spark" => "poster-indigo",
        "poster://teal-mist" => "poster-teal",
        "poster://rose-warm" => "poster-rose",
        _ => "poster-slate",
    };
    color_or(key, theme().secondary)
}

#[must_use]
pub fn radius() -> Points {
    theme().radius
}

#[must_use]
pub fn radius_card() -> Points {
    px(theme().number("radius-card").unwrap_or(14.0))
}

#[must_use]
pub fn radius_control() -> Points {
    px(theme().number("radius-control").unwrap_or(8.0))
}

#[must_use]
pub fn radius_badge() -> Points {
    px(theme().number("radius-badge").unwrap_or(6.0))
}

#[must_use]
pub fn stagger_enter(index: usize) -> Motion {
    let clamped = index.min(STAGGER_MAX_INDEX);
    let delay = u16::try_from(clamped).map_or(0.0, f32::from) * STAGGER_STEP_MS;
    motion().entering(MotionPreset::FadeInUp.spec().timing(move |mut t| {
        t.duration_ms = SLOW_MS;
        t.delay_ms = delay;
        t
    }))
}

#[must_use]
pub fn fade_enter() -> Motion {
    motion().entering(MotionPreset::FadeIn.spec().timing(|mut t| {
        t.duration_ms = BASE_MS;
        t
    }))
}
