use crate::protocol::VideoAspect;

pub const MAX_RENDER_DIMENSION: u32 = 3840;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetRect {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl TargetRect {
    #[must_use]
    pub fn right(self) -> f32 {
        self.left + self.width
    }

    #[must_use]
    pub fn bottom(self) -> f32 {
        self.top + self.height
    }
}

#[must_use]
pub fn compute_target_rect(
    aspect: VideoAspect,
    natural_size: (f32, f32),
    viewport_size: (f32, f32),
) -> TargetRect {
    let (vw, vh) = viewport_size;
    if !vw.is_finite() || !vh.is_finite() || vw <= 0.0 || vh <= 0.0 {
        return TargetRect {
            left: 0.0,
            top: 0.0,
            width: 0.0,
            height: 0.0,
        };
    }

    match aspect {
        VideoAspect::Original => fit_contain(natural_size, (vw, vh)),
        VideoAspect::Fill => fit_cover(natural_size, (vw, vh)),
        VideoAspect::Cinema16x9 => fit_contain((16.0, 9.0), (vw, vh)),
        VideoAspect::Portrait9x16 => fit_contain((9.0, 16.0), (vw, vh)),
    }
}

#[must_use]
pub fn clamp_render_dimensions(video_size: (u32, u32), max_dimension: u32) -> (u32, u32) {
    let (w, h) = video_size;
    if w == 0 || h == 0 || max_dimension == 0 {
        return (0, 0);
    }
    if w <= max_dimension && h <= max_dimension {
        return (w, h);
    }

    let max_f = f64::from(max_dimension);
    let wf = f64::from(w);
    let hf = f64::from(h);
    let scale = (max_f / wf).min(max_f / hf);
    let scaled_w = (wf * scale).round().clamp(1.0, max_f) as u32;
    let scaled_h = (hf * scale).round().clamp(1.0, max_f) as u32;
    (scaled_w, scaled_h)
}

fn fit_contain(source: (f32, f32), viewport: (f32, f32)) -> TargetRect {
    fit_with_scale(source, viewport, true)
}

fn fit_cover(source: (f32, f32), viewport: (f32, f32)) -> TargetRect {
    fit_with_scale(source, viewport, false)
}

fn fit_with_scale(source: (f32, f32), viewport: (f32, f32), contain: bool) -> TargetRect {
    let (sw, sh) = source;
    let (vw, vh) = viewport;
    if !sw.is_finite() || !sh.is_finite() || sw <= 0.0 || sh <= 0.0 {
        return TargetRect {
            left: 0.0,
            top: 0.0,
            width: vw,
            height: vh,
        };
    }

    let scale_x = vw / sw;
    let scale_y = vh / sh;
    let scale = if contain {
        scale_x.min(scale_y)
    } else {
        scale_x.max(scale_y)
    };
    let width = sw * scale;
    let height = sh * scale;
    TargetRect {
        left: (vw - width) * 0.5,
        top: (vh - height) * 0.5,
        width,
        height,
    }
}
