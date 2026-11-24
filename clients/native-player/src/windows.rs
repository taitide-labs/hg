use std::rc::Rc;

use nsh_core::JsonValue;
use nsh_platform_windows::windows::Win32::Graphics::Direct2D::ID2D1DeviceContext;
use nsh_platform_windows::{ComponentContext, Components, NativeComponent};

use crate::protocol::{VIDEO_PLAYER_COMPONENT_NAME, VideoPlayerProps};

struct WindowsVideoPlayer {
    props: VideoPlayerProps,
    _context: ComponentContext,
}

impl WindowsVideoPlayer {
    fn new(context: ComponentContext) -> Self {
        Self {
            props: VideoPlayerProps::default(),
            _context: context,
        }
    }
}

impl NativeComponent for WindowsVideoPlayer {
    fn draw(&self, _dc: &ID2D1DeviceContext, _size: (f32, f32)) {}

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        let _ = self.props.apply_prop(name, value);
    }
}

pub fn register(components: &Components) {
    components.register(
        VIDEO_PLAYER_COMPONENT_NAME,
        Rc::new(|context: ComponentContext| -> Box<dyn NativeComponent> {
            Box::new(WindowsVideoPlayer::new(context))
        }),
    );
}
