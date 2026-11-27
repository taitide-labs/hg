use std::sync::Arc;

use hg_app::root_with_gateway;
use hg_gateway::contract::AppGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::geometry::Bounds;
use nsh_core::{Points, Size};
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os};
use nsh_testing::Headless;

pub(crate) const DESKTOP_VIEWPORT: Size<Points> = Size::new(Points(1240.0), Points(800.0));
pub(crate) const MOBILE_VIEWPORT: Size<Points> = Size::new(Points(390.0), Points(844.0));

pub(crate) fn start_desktop(gw: Arc<MockGateway>, scheme: ColorScheme) -> Headless {
    start_with_options(gw, Os::MacOs, scheme, DESKTOP_VIEWPORT, None)
}

pub(crate) fn start_desktop_with_manual_clock(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> Headless {
    start_with_options(
        gw,
        Os::MacOs,
        scheme,
        DESKTOP_VIEWPORT,
        Some(nsh_core::time::Clock::manual(
            std::time::SystemTime::UNIX_EPOCH,
        )),
    )
}

pub(crate) fn start_desktop_on_home(gw: Arc<MockGateway>, scheme: ColorScheme) -> Headless {
    let mut app = start_desktop(gw, scheme);
    app.wait_for("最近更新").unwrap();
    app
}

pub(crate) fn start_mobile(gw: Arc<MockGateway>, scheme: ColorScheme) -> Headless {
    start_with_options(gw, Os::Android, scheme, MOBILE_VIEWPORT, None)
}

fn start_with_options(
    gw: Arc<MockGateway>,
    os: Os,
    scheme: ColorScheme,
    viewport: Size<Points>,
    clock: Option<nsh_core::time::Clock>,
) -> Headless {
    let gw_trait: Arc<dyn AppGateway> = gw;
    let environment = Environment {
        os,
        color_scheme: scheme,
        ..Environment::default()
    };
    let mut config = EngineConfig {
        viewport,
        environment,
        ..EngineConfig::new(root_with_gateway(gw_trait))
    };
    if let Some(clock) = clock {
        config.clock = clock;
    }
    Headless::start(config).unwrap()
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
