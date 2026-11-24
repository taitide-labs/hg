mod library_delete;
mod library_query;
mod queue_control;
mod queue_query;

use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use crate::App;
use crate::app::flags::{self, FeatureFlags};

pub(super) fn start_on_offline(override_flags: FeatureFlags) -> Headless {
    flags::set_for_test(override_flags);
    let environment = Environment {
        os: Os::MacOs,
        color_scheme: ColorScheme::Light,
        ..Environment::default()
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment,
        ..EngineConfig::new(Root::of::<App>())
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();
    app.press("离线").unwrap();
    app
}
