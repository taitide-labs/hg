use std::rc::Rc;

use nsh_core::JsonValue;
use nsh_platform_macos::objc2::MainThreadMarker;
use nsh_platform_macos::objc2::rc::Retained;
use nsh_platform_macos::objc2_app_kit::NSView;
use nsh_platform_macos::{ComponentContext, Components, NativeComponent};

use crate::protocol::{VIDEO_PLAYER_COMPONENT_NAME, VideoPlayerProps};

struct MacOsVideoPlayer {
    view: Retained<NSView>,
    props: VideoPlayerProps,
    _context: ComponentContext,
}

impl MacOsVideoPlayer {
    fn new(context: ComponentContext) -> Self {
        #[expect(
            unsafe_code,
            reason = "NSH macOS ComponentFactory 仅在 AppKit 主线程创建原生视图"
        )]
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        Self {
            view: NSView::new(mtm),
            props: VideoPlayerProps::default(),
            _context: context,
        }
    }
}

impl NativeComponent for MacOsVideoPlayer {
    fn view(&self) -> Retained<NSView> {
        Retained::clone(&self.view)
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        let _ = self.props.apply_prop(name, value);
    }
}

pub fn register(components: &Components) {
    components.register(
        VIDEO_PLAYER_COMPONENT_NAME,
        Rc::new(|context: ComponentContext| -> Box<dyn NativeComponent> {
            Box::new(MacOsVideoPlayer::new(context))
        }),
    );
}
