mod avfoundation;
mod bridge;
mod pool;
mod view;

#[cfg(test)]
mod tests;

use std::rc::Rc;

use nsh_core::JsonValue;
use nsh_platform_macos::objc2::MainThreadMarker;
use nsh_platform_macos::objc2::rc::Retained;
use nsh_platform_macos::objc2::runtime::AnyObject;
use nsh_platform_macos::objc2_app_kit::NSView;
use nsh_platform_macos::{ComponentContext, Components, NativeComponent};

use self::bridge::{
    PlaybackSession, emit_error, seek_to_secs, sync_audio, sync_playback, sync_rate,
};
use self::pool::{acquire_av_player, release_av_player};
use self::view::HgVideoLayerView;
use crate::protocol::{PropChange, VIDEO_PLAYER_COMPONENT_NAME, VideoPlayerProps};

struct MacOsVideoPlayer {
    view: Retained<HgVideoLayerView>,
    player: Option<Retained<AnyObject>>,
    session: PlaybackSession,
    props: VideoPlayerProps,
    pending_seek: Option<u32>,
    context: ComponentContext,
}

impl MacOsVideoPlayer {
    fn new(context: ComponentContext) -> Self {
        #[expect(
            unsafe_code,
            reason = "NSH macOS ComponentFactory 仅在 AppKit 主线程创建原生视图"
        )]
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        Self {
            view: HgVideoLayerView::new(mtm),
            player: None,
            session: PlaybackSession::new(),
            props: VideoPlayerProps::default(),
            pending_seek: None,
            context,
        }
    }

    fn ensure_player(&mut self) -> Option<Retained<AnyObject>> {
        if let Some(player) = self.player.as_ref() {
            return Some(Retained::clone(player));
        }
        if !avfoundation::ensure_loaded() {
            emit_error(
                &self.context,
                "AVFoundation 框架不可用，无法初始化视频播放器".to_owned(),
            );
            return None;
        }
        let Some(player) = acquire_av_player() else {
            emit_error(&self.context, "无法创建 AVPlayer 实例".to_owned());
            return None;
        };
        if !self.view.attach_player(&player, self.props.aspect) {
            release_av_player(player);
            emit_error(&self.context, "无法挂载 AVPlayerLayer 图层".to_owned());
            return None;
        }
        self.session.install_time_observer(&player, &self.context);
        self.player = Some(Retained::clone(&player));
        Some(player)
    }

    fn release_active_player(&mut self) {
        self.session.clear_item_observers();
        self.view.detach_player();
        if let Some(player) = self.player.take() {
            self.session.remove_time_observer(&player);
            release_av_player(player);
        }
    }

    fn handle_source_change(&mut self) {
        let Some(raw_url) = self.props.src.clone() else {
            self.pending_seek = None;
            self.release_active_player();
            return;
        };
        let Some(player) = self.ensure_player() else {
            return;
        };
        if self
            .session
            .load_source(&player, &raw_url, &self.props, &self.context)
            && let Some(position_secs) = self.pending_seek.take()
        {
            self.session.mark_active_for_seek();
            seek_to_secs(&player, position_secs);
        }
    }

    fn handle_seek(&mut self, position_secs: u32) {
        let Some(player) = self.player.as_ref() else {
            self.pending_seek = Some(position_secs);
            return;
        };
        self.session.mark_active_for_seek();
        seek_to_secs(player, position_secs);
        if !self.props.paused {
            sync_playback(player, &self.props, self.session.is_ended());
        }
    }
}

impl Drop for MacOsVideoPlayer {
    fn drop(&mut self) {
        self.release_active_player();
    }
}

impl NativeComponent for MacOsVideoPlayer {
    fn view(&self) -> Retained<NSView> {
        self.view.as_ns_view()
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        match self.props.apply_prop(name, value) {
            PropChange::Unchanged => {}
            PropChange::Source => self.handle_source_change(),
            PropChange::Paused => {
                if let Some(player) = self.player.as_ref() {
                    sync_playback(player, &self.props, self.session.is_ended());
                }
            }
            PropChange::Rate => {
                if let Some(player) = self.player.as_ref() {
                    sync_rate(player, &self.props);
                }
            }
            PropChange::Volume | PropChange::Muted => {
                if let Some(player) = self.player.as_ref() {
                    sync_audio(player, &self.props);
                }
            }
            PropChange::Seek { position_secs } => self.handle_seek(position_secs),
            PropChange::Aspect => self.view.set_aspect(self.props.aspect),
        }
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
