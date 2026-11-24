use std::cell::RefCell;

use nsh_platform_macos::objc2::rc::Retained;
use nsh_platform_macos::objc2::runtime::{AnyClass, AnyObject, Bool};
use nsh_platform_macos::objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
};
use nsh_platform_macos::objc2_app_kit::{NSResponder, NSView};
use nsh_platform_macos::objc2_foundation::{
    NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use super::avfoundation::{class, video_gravity_for_aspect};
use crate::protocol::VideoAspect;

pub(super) struct VideoLayerIvars {
    player_layer: RefCell<Option<Retained<AnyObject>>>,
}

define_class!(
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = VideoLayerIvars]
    #[name = "HgVideoLayerView"]
    pub(super) struct HgVideoLayerView;

    unsafe impl NSObjectProtocol for HgVideoLayerView {}

    impl HgVideoLayerView {
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> Option<Retained<NSView>> {
            None
        }

        #[unsafe(method(layout))]
        fn layout(&self) {
            #[expect(unsafe_code, reason = "调用父类 NSView layout 方法")]
            unsafe {
                let _: () = msg_send![super(self), layout];
            }
            self.sync_layer_frame();
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, new_size: NSSize) {
            #[expect(unsafe_code, reason = "调用父类 NSView setFrameSize: 方法")]
            unsafe {
                let _: () = msg_send![super(self), setFrameSize: new_size];
            }
            self.sync_layer_frame();
        }
    }
);

impl HgVideoLayerView {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let allocated = Self::alloc(mtm).set_ivars(VideoLayerIvars {
            player_layer: RefCell::new(None),
        });
        #[expect(
            unsafe_code,
            reason = "调用 NSView 指定初始化方法 initWithFrame: 并启用 layer-backed 模式"
        )]
        let view: Retained<Self> =
            unsafe { msg_send![super(allocated), initWithFrame: NSRect::ZERO] };
        view.setWantsLayer(true);
        view
    }

    pub(super) fn as_ns_view(&self) -> Retained<NSView> {
        Retained::from(self)
    }

    pub(super) fn attach_player(&self, player: &AnyObject, aspect: VideoAspect) -> bool {
        let mut slot = self.ivars().player_layer.borrow_mut();
        if let Some(layer) = slot.as_ref() {
            #[expect(unsafe_code, reason = "将池中复用的 AVPlayer 绑定到既有 AVPlayerLayer")]
            unsafe {
                let _: () = msg_send![&**layer, setPlayer: player];
            }
            drop(slot);
            self.set_aspect(aspect);
            self.sync_layer_frame();
            return true;
        }
        let Some(cls) = class(c"AVPlayerLayer") else {
            return false;
        };
        #[expect(
            unsafe_code,
            reason = "创建 AVPlayerLayer 并将其作为子图层挂载到 HgVideoLayerView 根图层"
        )]
        let created: Option<Retained<AnyObject>> = unsafe {
            let layer: Option<Retained<AnyObject>> = msg_send![cls, playerLayerWithPlayer: player];
            if let Some(ref player_layer) = layer {
                let root_layer: Option<Retained<AnyObject>> = msg_send![self, layer];
                if let Some(root) = root_layer {
                    let _: () = msg_send![&*root, addSublayer: &**player_layer];
                }
            }
            layer
        };
        let Some(layer) = created else {
            return false;
        };
        *slot = Some(layer);
        drop(slot);
        self.set_aspect(aspect);
        self.sync_layer_frame();
        true
    }

    pub(super) fn detach_player(&self) {
        if let Some(layer) = self.ivars().player_layer.borrow().as_ref() {
            #[expect(unsafe_code, reason = "解绑 AVPlayerLayer 对 AVPlayer 的强引用")]
            unsafe {
                let _: () = msg_send![&**layer, setPlayer: std::ptr::null::<AnyObject>()];
            }
        }
    }

    pub(super) fn set_aspect(&self, aspect: VideoAspect) {
        let Some(layer) = self
            .ivars()
            .player_layer
            .borrow()
            .as_ref()
            .map(Retained::clone)
        else {
            return;
        };
        let gravity = NSString::from_str(video_gravity_for_aspect(aspect));
        #[expect(unsafe_code, reason = "设置 AVPlayerLayer videoGravity 画面比例模式")]
        unsafe {
            let _: () = msg_send![&*layer, setVideoGravity: &*gravity];
        }
    }

    fn sync_layer_frame(&self) {
        let Some(layer) = self
            .ivars()
            .player_layer
            .borrow()
            .as_ref()
            .map(Retained::clone)
        else {
            return;
        };
        let bounds = self.bounds();
        #[expect(
            unsafe_code,
            reason = "在无隐式动画事务中将 AVPlayerLayer frame 同步至视图 bounds"
        )]
        unsafe {
            if let Some(tx) = AnyClass::get(c"CATransaction") {
                let _: () = msg_send![tx, begin];
                let _: () = msg_send![tx, setDisableActions: Bool::YES];
                let _: () = msg_send![&*layer, setFrame: bounds];
                let _: () = msg_send![tx, commit];
            } else {
                let _: () = msg_send![&*layer, setFrame: bounds];
            }
        }
    }
}
