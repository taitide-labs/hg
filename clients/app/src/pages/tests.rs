use std::sync::Arc;
use std::time::{Duration, SystemTime};

use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_gateway::contract::AppGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::geometry::Bounds;
use nsh_core::prelude::*;
use nsh_core::{EventKind, EventPayload, Points, Size, ViewKind};
use nsh_engine::{Clock, ColorScheme, EngineConfig, Environment, Os};
use nsh_testing::Headless;
use parking_lot::Mutex;

pub(crate) const DESKTOP_VIEWPORT: Size<Points> = Size::new(Points(1240.0), Points(800.0));
pub(crate) const MOBILE_VIEWPORT: Size<Points> = Size::new(Points(390.0), Points(844.0));

type PageBuilder = Arc<dyn Fn() -> AnyElement + Send + Sync>;

#[derive(Clone)]
struct PageHarnessProps {
    gateway: Arc<dyn AppGateway>,
    builder: PageBuilder,
}

impl PartialEq for PageHarnessProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.gateway, &other.gateway) && Arc::ptr_eq(&self.builder, &other.builder)
    }
}

impl nsh_core::element::Props for PageHarnessProps {
    fn spec(_: &mut nsh_core::element::TsTypes) -> Vec<nsh_core::element::PropSpec> {
        Vec::new()
    }

    fn initial() -> Self {
        Self {
            gateway: Arc::new(MockGateway::new()),
            builder: Arc::new(|| view().into_element()),
        }
    }

    fn set(
        &mut self,
        name: &str,
        _: Option<nsh_core::element::AnyValue>,
    ) -> Result<bool, nsh_core::element::PropError> {
        Err(nsh_core::element::PropError(format!(
            "unknown prop `{name}`"
        )))
    }

    fn set_slot(&mut self, _: &str, _: Option<nsh_core::element::Slot>) -> bool {
        false
    }

    fn set_children(&mut self, _: nsh_core::element::Children) -> bool {
        false
    }
}

struct PageHarness;

impl nsh_core::element::Component for PageHarness {
    type Props = PageHarnessProps;
    type Event = nsh_core::element::NoEvent;

    fn view(props: Self::Props, _cx: Scope<Self::Event>) -> impl IntoElement {
        crate::app::init::initialize_with_gateway(props.gateway);
        view()
            .flex_1()
            .bg(crate::ui::token::bg)
            .child((props.builder)())
    }
}

type OpenSeriesCall = (SeriesId, Option<EpisodeNumber>);

#[derive(Clone, Default)]
pub(crate) struct OpenSeriesSpy {
    calls: Arc<Mutex<Vec<OpenSeriesCall>>>,
}

impl OpenSeriesSpy {
    pub(crate) fn record(&self, series_id: SeriesId, ep: Option<EpisodeNumber>) {
        self.calls.lock().push((series_id, ep));
    }

    pub(crate) fn last(&self) -> Option<(SeriesId, Option<EpisodeNumber>)> {
        self.calls.lock().last().copied()
    }
}

#[derive(Clone, Default)]
pub(crate) struct EventSpy<T> {
    events: Arc<Mutex<Vec<T>>>,
}

impl<T: Clone> EventSpy<T> {
    pub(crate) fn push(&self, value: T) {
        self.events.lock().push(value);
    }

    pub(crate) fn snapshot(&self) -> Vec<T> {
        self.events.lock().clone()
    }
}

pub(crate) fn mount_desktop_page<E, F>(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
    build: F,
) -> Headless
where
    E: IntoElement,
    F: Fn() -> E + Send + Sync + 'static,
{
    mount_page_with_options(gw, Os::MacOs, scheme, DESKTOP_VIEWPORT, false, build)
}

pub(crate) fn mount_desktop_page_manual_clock<E, F>(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
    build: F,
) -> Headless
where
    E: IntoElement,
    F: Fn() -> E + Send + Sync + 'static,
{
    mount_page_with_options(gw, Os::MacOs, scheme, DESKTOP_VIEWPORT, true, build)
}

pub(crate) fn mount_mobile_page<E, F>(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
    build: F,
) -> Headless
where
    E: IntoElement,
    F: Fn() -> E + Send + Sync + 'static,
{
    mount_page_with_options(gw, Os::Android, scheme, MOBILE_VIEWPORT, false, build)
}

pub(crate) fn mount_page_with_options<E, F>(
    gw: Arc<MockGateway>,
    os: Os,
    scheme: ColorScheme,
    viewport: Size<Points>,
    manual_clock: bool,
    build: F,
) -> Headless
where
    E: IntoElement,
    F: Fn() -> E + Send + Sync + 'static,
{
    let gateway: Arc<dyn AppGateway> = gw;
    let builder: PageBuilder = Arc::new(move || build().into_element());
    let root = nsh_core::element::Root::new::<PageHarness>(move || PageHarnessProps {
        gateway: Arc::clone(&gateway),
        builder: Arc::clone(&builder),
    });
    let environment = Environment {
        os,
        color_scheme: scheme,
        ..Environment::default()
    };
    let mut config = EngineConfig {
        viewport,
        environment,
        ..EngineConfig::new(root)
    };
    if manual_clock {
        config.clock = Clock::manual(SystemTime::UNIX_EPOCH);
    }
    Headless::start(config).unwrap()
}

pub(crate) fn emit_video_progress(app: &mut Headless, position_secs: u32, duration_secs: u32) {
    dispatch_native_video_event(
        app,
        "progress",
        serde_json::json!({
            "position_secs": position_secs,
            "duration_secs": duration_secs,
        }),
    );
}

pub(crate) fn emit_video_ended(app: &mut Headless) {
    dispatch_native_video_event(app, "ended", serde_json::json!({}));
}

pub(crate) fn emit_video_error(app: &mut Headless, message: &str) {
    dispatch_native_video_event(app, "error", serde_json::json!({ "message": message }));
}

fn dispatch_native_video_event(app: &mut Headless, name: &str, data: serde_json::Value) {
    let video_node = app
        .screen()
        .layout_nodes()
        .into_iter()
        .find(|node| node.kind == ViewKind::Native)
        .map(|node| node.id)
        .expect("native video node must be mounted");

    app.engine().control().dispatch_event(
        video_node,
        EventKind::NativeEvent,
        EventPayload::Native {
            name: Arc::from(name),
            data,
        },
    );
    app.wait(Duration::from_millis(30)).unwrap();
}

pub(crate) fn node_bounds(app: &Headless, selector: &str) -> Bounds<Points> {
    let id = app.screen().id(selector).unwrap_or_else(|err| {
        panic!(
            "selector `{selector}` not found ({err}); visible texts: {:?}",
            app.screen().texts()
        )
    });
    app.screen()
        .layout_nodes()
        .into_iter()
        .find(|node| node.id == id)
        .map(|node| node.bounds)
        .unwrap_or_else(|| panic!("layout node for `{selector}` ({id:?}) missing"))
}

pub(crate) fn assert_visible_in_viewport(app: &Headless, selector: &str, viewport: Size<Points>) {
    let bounds = node_bounds(app, selector);
    assert!(
        bounds.size.width.0 > 0.0 && bounds.size.height.0 > 0.0,
        "`{selector}` must have non-zero size, got {:?}",
        bounds.size
    );
    assert!(
        bounds.origin.x.0 >= -2.0
            && bounds.origin.y.0 >= -2.0
            && bounds.origin.x.0 + bounds.size.width.0 <= viewport.width.0 + 2.0,
        "`{selector}` bounds {bounds:?} exceed viewport {viewport:?}"
    );
}

pub(crate) fn assert_left_aligned(app: &Headless, sel_a: &str, sel_b: &str, tolerance: f32) {
    let a = node_bounds(app, sel_a);
    let b = node_bounds(app, sel_b);
    assert!(
        (a.origin.x.0 - b.origin.x.0).abs() <= tolerance,
        "`{sel_a}` (x={}) and `{sel_b}` (x={}) must be left-aligned within {tolerance}pt",
        a.origin.x.0,
        b.origin.x.0
    );
}

pub(crate) fn assert_same_height(app: &Headless, sel_a: &str, sel_b: &str, tolerance: f32) {
    let a = node_bounds(app, sel_a);
    let b = node_bounds(app, sel_b);
    assert!(
        (a.size.height.0 - b.size.height.0).abs() <= tolerance,
        "`{sel_a}` (h={}) and `{sel_b}` (h={}) must share height within {tolerance}pt",
        a.size.height.0,
        b.size.height.0
    );
}

pub(crate) fn assert_above(app: &Headless, upper_sel: &str, lower_sel: &str) {
    let upper = node_bounds(app, upper_sel);
    let lower = node_bounds(app, lower_sel);
    assert!(
        upper.origin.y.0 + upper.size.height.0 <= lower.origin.y.0 + 2.0,
        "`{upper_sel}` (bottom={}) must sit above `{lower_sel}` (top={})",
        upper.origin.y.0 + upper.size.height.0,
        lower.origin.y.0
    );
}

pub(crate) fn assert_left_of(app: &Headless, left_sel: &str, right_sel: &str) {
    let left = node_bounds(app, left_sel);
    let right = node_bounds(app, right_sel);
    assert!(
        left.origin.x.0 + left.size.width.0 <= right.origin.x.0 + 2.0,
        "`{left_sel}` (right={}) must sit left of `{right_sel}` (left={})",
        left.origin.x.0 + left.size.width.0,
        right.origin.x.0
    );
}

pub(crate) fn count_first_row_cards(app: &Headless, selectors: &[String]) -> usize {
    let nodes = app.screen().layout_nodes();
    let mut ys = Vec::new();
    for sel in selectors {
        if let Ok(id) = app.screen().id(sel)
            && let Some(node) = nodes.iter().find(|n| n.id == id)
        {
            ys.push(node.bounds.origin.y.0);
        }
    }
    let Some(min_y) = ys.iter().copied().reduce(f32::min) else {
        return 0;
    };
    ys.into_iter().filter(|y| (y - min_y).abs() <= 4.0).count()
}

pub(crate) fn assert_node_stable<F>(app: &mut Headless, selector: &str, action: F)
where
    F: FnOnce(&mut Headless),
{
    let before = app
        .screen()
        .id(selector)
        .unwrap_or_else(|err| panic!("selector `{selector}` missing before action: {err}"));
    action(app);
    let after = app
        .screen()
        .id(selector)
        .unwrap_or_else(|err| panic!("selector `{selector}` missing after action: {err}"));
    assert_eq!(
        after, before,
        "`{selector}` remounted unexpectedly across state update"
    );
}
