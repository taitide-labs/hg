use std::cell::Cell;
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use nsh_platform_macos::ComponentContext;
use nsh_platform_macos::objc2::rc::{Retained, Weak};
use nsh_platform_macos::objc2::runtime::{AnyObject, Bool, ProtocolObject};
use nsh_platform_macos::objc2::{msg_send, sel};
use nsh_platform_macos::objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObjectProtocol, NSOperationQueue, NSString, NSTimer,
};

use super::avfoundation::{CMTime, class, cmtime_to_secs_u32, parse_nsurl};
use crate::protocol::{VideoPlayerEvent, VideoPlayerProps};

const PROGRESS_INTERVAL_SECS: f64 = 0.5;
const LOAD_POLL_INTERVAL_SECS: f64 = 0.1;
const LOAD_POLL_MAX_TICKS: u32 = 300;
const AV_PLAYER_ITEM_STATUS_READY: isize = 1;
const AV_PLAYER_ITEM_STATUS_FAILED: isize = 2;

pub(super) struct PlaybackSession {
    time_observer: Option<Retained<AnyObject>>,
    end_observer: Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
    fail_observer: Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
    load_timer: Option<Retained<NSTimer>>,
    item_active: Rc<Cell<bool>>,
    item_ended: Rc<Cell<bool>>,
    item_failed: Rc<Cell<bool>>,
    pending_seek_secs: Rc<Cell<Option<u32>>>,
    paused_state: Rc<Cell<bool>>,
    rate_state: Rc<Cell<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SeekDisposition {
    Immediate,
    Deferred,
}

pub(super) fn should_emit_progress(active: bool, ended: bool) -> bool {
    active && !ended
}

pub(super) fn resolve_seek_disposition(item_status: Option<isize>) -> SeekDisposition {
    if item_status == Some(AV_PLAYER_ITEM_STATUS_READY) {
        SeekDisposition::Immediate
    } else {
        SeekDisposition::Deferred
    }
}

impl PlaybackSession {
    pub(super) fn new() -> Self {
        Self {
            time_observer: None,
            end_observer: None,
            fail_observer: None,
            load_timer: None,
            item_active: Rc::new(Cell::new(false)),
            item_ended: Rc::new(Cell::new(false)),
            item_failed: Rc::new(Cell::new(false)),
            pending_seek_secs: Rc::new(Cell::new(None)),
            paused_state: Rc::new(Cell::new(false)),
            rate_state: Rc::new(Cell::new(1.0)),
        }
    }

    pub(super) fn install_time_observer(
        &mut self,
        player: &Retained<AnyObject>,
        context: &ComponentContext,
    ) {
        self.remove_time_observer(player);
        let weak_player = Weak::from_retained(player);
        let ctx = context.clone();
        let failed = Rc::clone(&self.item_failed);
        let active = Rc::clone(&self.item_active);
        let ended = Rc::clone(&self.item_ended);
        let block = RcBlock::new(move |time: CMTime| {
            if !should_emit_progress(active.get(), ended.get()) {
                return;
            }
            let Some(player) = weak_player.load() else {
                return;
            };
            check_current_item_failure(&player, &ctx, &failed);
            let position_secs = cmtime_to_secs_u32(time);
            let duration_secs = current_item_duration_secs(&player);
            emit_event(
                &ctx,
                VideoPlayerEvent::Progress {
                    position_secs,
                    duration_secs,
                },
            );
        });
        let interval = CMTime::from_seconds(PROGRESS_INTERVAL_SECS);
        #[expect(
            unsafe_code,
            reason = "注册 AVPlayer 0.5 秒周期进度监听（nil queue 默认为主队列）"
        )]
        let observer: Option<Retained<AnyObject>> = unsafe {
            msg_send![
                &**player,
                addPeriodicTimeObserverForInterval: interval,
                queue: std::ptr::null::<AnyObject>(),
                usingBlock: &*block
            ]
        };
        self.time_observer = observer;
    }

    pub(super) fn remove_time_observer(&mut self, player: &AnyObject) {
        if let Some(token) = self.time_observer.take() {
            #[expect(
                unsafe_code,
                reason = "注销此前通过 addPeriodicTimeObserverForInterval: 添加的观察者"
            )]
            unsafe {
                let _: () = msg_send![player, removeTimeObserver: &*token];
            }
        }
    }

    pub(super) fn clear_item_observers(&mut self) {
        self.item_active.set(false);
        if let Some(timer) = self.load_timer.take() {
            timer.invalidate();
        }
        let center = NSNotificationCenter::defaultCenter();
        if let Some(observer) = self.end_observer.take() {
            #[expect(unsafe_code, reason = "从默认通知中心移除播放结束监听")]
            unsafe {
                center.removeObserver(observer.as_ref());
            }
        }
        if let Some(observer) = self.fail_observer.take() {
            #[expect(unsafe_code, reason = "从默认通知中心移除播放失败监听")]
            unsafe {
                center.removeObserver(observer.as_ref());
            }
        }
        self.item_ended.set(false);
        self.item_failed.set(false);
        self.pending_seek_secs.set(None);
    }

    pub(super) fn load_source(
        &mut self,
        player: &Retained<AnyObject>,
        raw_url: &str,
        props: &VideoPlayerProps,
        context: &ComponentContext,
    ) -> bool {
        self.clear_item_observers();
        let Some(url) = parse_nsurl(raw_url) else {
            emit_error(context, format!("无效的视频地址: {raw_url}"));
            return false;
        };
        let Some(item) = create_player_item(&url) else {
            emit_error(context, format!("无法初始化媒体播放项: {raw_url}"));
            return false;
        };

        self.item_active.set(true);
        self.item_ended.set(false);
        self.item_failed.set(false);
        self.sync_control_state(props);
        self.pending_seek_secs.set(None);

        self.install_item_notifications(&item, context);
        self.install_load_timer(player, context);

        #[expect(unsafe_code, reason = "向 AVPlayer 替换当前 AVPlayerItem 播放源")]
        unsafe {
            let _: () = msg_send![&**player, replaceCurrentItemWithPlayerItem: &*item];
        }
        sync_audio(player, props);
        sync_playback(player, props, &self.item_ended);
        true
    }

    fn install_item_notifications(&mut self, item: &AnyObject, context: &ComponentContext) {
        let center = NSNotificationCenter::defaultCenter();
        let main_queue = NSOperationQueue::mainQueue();

        let end_ctx = context.clone();
        let end_active = Rc::clone(&self.item_active);
        let end_flag = Rc::clone(&self.item_ended);
        let end_block = RcBlock::new(move |_: NonNull<NSNotification>| {
            if !end_active.get() {
                return;
            }
            end_flag.set(true);
            emit_event(&end_ctx, VideoPlayerEvent::Ended);
        });

        let fail_ctx = context.clone();
        let fail_active = Rc::clone(&self.item_active);
        let fail_flag = Rc::clone(&self.item_failed);
        let fail_block = RcBlock::new(move |notification: NonNull<NSNotification>| {
            if !fail_active.get() || fail_flag.replace(true) {
                return;
            }
            let message = extract_notification_error(notification)
                .unwrap_or_else(|| "视频播放失败".to_owned());
            emit_error(&fail_ctx, message);
        });

        #[expect(
            unsafe_code,
            reason = "向 NSNotificationCenter 注册当前 AVPlayerItem 的结束与失败通知"
        )]
        unsafe {
            self.end_observer = Some(center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str(
                    "AVPlayerItemDidPlayToEndTimeNotification",
                )),
                Some(item),
                Some(&main_queue),
                &end_block,
            ));
            self.fail_observer = Some(center.addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str(
                    "AVPlayerItemFailedToPlayToEndTimeNotification",
                )),
                Some(item),
                Some(&main_queue),
                &fail_block,
            ));
        }
    }

    fn install_load_timer(&mut self, player: &Retained<AnyObject>, context: &ComponentContext) {
        let weak_player = Weak::from_retained(player);
        let ctx = context.clone();
        let active_flag = Rc::clone(&self.item_active);
        let ended_flag = Rc::clone(&self.item_ended);
        let failed_flag = Rc::clone(&self.item_failed);
        let pending_seek = Rc::clone(&self.pending_seek_secs);
        let paused_state = Rc::clone(&self.paused_state);
        let rate_state = Rc::clone(&self.rate_state);
        let ticks = Cell::new(0_u32);
        let block = RcBlock::new(move |timer_ptr: NonNull<NSTimer>| {
            #[expect(unsafe_code, reason = "NSTimer 回调传入有效的非空主线程 NSTimer 引用")]
            let timer = unsafe { timer_ptr.as_ref() };
            let next_tick = ticks.get().saturating_add(1);
            ticks.set(next_tick);
            if !active_flag.get() || next_tick >= LOAD_POLL_MAX_TICKS {
                timer.invalidate();
                return;
            }
            let Some(player) = weak_player.load() else {
                timer.invalidate();
                return;
            };
            if let Some(status) = current_item_status(&player) {
                if status == AV_PLAYER_ITEM_STATUS_FAILED {
                    check_current_item_failure(&player, &ctx, &failed_flag);
                    timer.invalidate();
                } else if status == AV_PLAYER_ITEM_STATUS_READY {
                    timer.invalidate();
                    if let Some(target_secs) = pending_seek.take() {
                        ended_flag.set(false);
                        seek_to_secs(&player, target_secs);
                        sync_playback_state(
                            &player,
                            paused_state.get(),
                            rate_state.get(),
                            &ended_flag,
                        );
                    }
                }
            }
        });
        #[expect(
            unsafe_code,
            reason = "在主 RunLoop 创建 0.1s 周期 NSTimer 轮询 AVPlayerItem 初始加载状态"
        )]
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_repeats_block(
                LOAD_POLL_INTERVAL_SECS,
                true,
                &block,
            )
        };
        self.load_timer = Some(timer);
    }

    pub(super) fn sync_control_state(&self, props: &VideoPlayerProps) {
        self.paused_state.set(props.paused);
        self.rate_state.set(props.rate);
    }

    pub(super) fn seek(&self, player: &AnyObject, position_secs: u32, props: &VideoPlayerProps) {
        self.item_ended.set(false);
        self.sync_control_state(props);
        match resolve_seek_disposition(current_item_status(player)) {
            SeekDisposition::Immediate => {
                self.pending_seek_secs.set(None);
                seek_to_secs(player, position_secs);
                if !props.paused {
                    sync_playback(player, props, &self.item_ended);
                }
            }
            SeekDisposition::Deferred => {
                self.pending_seek_secs.set(Some(position_secs));
            }
        }
    }

    pub(super) fn is_ended(&self) -> &Cell<bool> {
        &self.item_ended
    }
}

pub(super) fn emit_event(context: &ComponentContext, event: VideoPlayerEvent) {
    let (name, data) = event.into_name_and_payload();
    context.emit(name, data);
}

pub(super) fn emit_error(context: &ComponentContext, message: String) {
    emit_event(context, VideoPlayerEvent::Error { message });
}

pub(super) fn sync_audio(player: &AnyObject, props: &VideoPlayerProps) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "props.volume 已限制在 [0.0, 1.0]，可安全转换为 f32"
    )]
    let volume = props.volume.clamp(0.0, 1.0) as f32;
    #[expect(unsafe_code, reason = "同步 AVPlayer 音量与静音状态")]
    unsafe {
        let _: () = msg_send![player, setVolume: volume];
        let _: () = msg_send![player, setMuted: Bool::new(props.muted)];
    }
}

pub(super) fn sync_playback(player: &AnyObject, props: &VideoPlayerProps, ended: &Cell<bool>) {
    sync_playback_state(player, props.paused, props.rate, ended);
}

fn sync_playback_state(player: &AnyObject, paused: bool, rate_f64: f64, ended: &Cell<bool>) {
    let rate = clamp_rate_f32(rate_f64);
    #[expect(unsafe_code, reason = "同步 AVPlayer 播放、暂停与倍速状态")]
    unsafe {
        let has_default_rate: Bool = msg_send![player, respondsToSelector: sel!(setDefaultRate:)];
        if has_default_rate.as_bool() {
            let _: () = msg_send![player, setDefaultRate: rate];
        }
        if paused {
            let _: () = msg_send![player, pause];
            return;
        }
        if ended.replace(false) {
            seek_to_secs(player, 0);
        }
        let has_play_at_rate: Bool =
            msg_send![player, respondsToSelector: sel!(playImmediatelyAtRate:)];
        if has_play_at_rate.as_bool() {
            let _: () = msg_send![player, playImmediatelyAtRate: rate];
        } else {
            let _: () = msg_send![player, setRate: rate];
        }
    }
}

pub(super) fn sync_rate(player: &AnyObject, props: &VideoPlayerProps) {
    let rate = clamp_rate_f32(props.rate);
    #[expect(unsafe_code, reason = "同步 AVPlayer 倍速设置")]
    unsafe {
        let has_default_rate: Bool = msg_send![player, respondsToSelector: sel!(setDefaultRate:)];
        if has_default_rate.as_bool() {
            let _: () = msg_send![player, setDefaultRate: rate];
        }
        if !props.paused {
            let _: () = msg_send![player, setRate: rate];
        }
    }
}

pub(super) fn seek_to_secs(player: &AnyObject, position_secs: u32) {
    let target = CMTime::from_seconds(f64::from(position_secs));
    let zero = CMTime::zero();
    #[expect(unsafe_code, reason = "调用 AVPlayer 精准 seekToTime: API")]
    unsafe {
        let _: () = msg_send![
            player,
            seekToTime: target,
            toleranceBefore: zero,
            toleranceAfter: zero
        ];
    }
}

fn clamp_rate_f32(rate: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rate 已限制在 [0.25, 4.0]，可安全转换为 f32"
    )]
    let clamped = rate.clamp(0.25, 4.0) as f32;
    clamped
}

fn create_player_item(
    url: &nsh_platform_macos::objc2_foundation::NSURL,
) -> Option<Retained<AnyObject>> {
    let asset_cls = class(c"AVURLAsset")?;
    let item_cls = class(c"AVPlayerItem")?;
    #[expect(unsafe_code, reason = "使用 AVURLAsset 与 AVPlayerItem 构造媒体播放项")]
    unsafe {
        let asset: Option<Retained<AnyObject>> = msg_send![
            asset_cls,
            URLAssetWithURL: url,
            options: std::ptr::null::<AnyObject>()
        ];
        let asset = asset?;
        msg_send![item_cls, playerItemWithAsset: &*asset]
    }
}

fn current_item_status(player: &AnyObject) -> Option<isize> {
    #[expect(unsafe_code, reason = "读取 AVPlayer currentItem.status")]
    unsafe {
        let item: Option<Retained<AnyObject>> = msg_send![player, currentItem];
        item.map(|it| msg_send![&*it, status])
    }
}

fn current_item_duration_secs(player: &AnyObject) -> u32 {
    #[expect(unsafe_code, reason = "读取 AVPlayer currentItem.duration")]
    unsafe {
        let item: Option<Retained<AnyObject>> = msg_send![player, currentItem];
        item.map_or(0, |it| {
            let duration: CMTime = msg_send![&*it, duration];
            cmtime_to_secs_u32(duration)
        })
    }
}

fn check_current_item_failure(player: &AnyObject, context: &ComponentContext, failed: &Cell<bool>) {
    if failed.get() {
        return;
    }
    #[expect(unsafe_code, reason = "检查 AVPlayerItem 失败状态与错误描述")]
    unsafe {
        let item: Option<Retained<AnyObject>> = msg_send![player, currentItem];
        let Some(item) = item else {
            return;
        };
        let status: isize = msg_send![&*item, status];
        if status == AV_PLAYER_ITEM_STATUS_FAILED && !failed.replace(true) {
            let err: Option<Retained<AnyObject>> = msg_send![&*item, error];
            let message = err
                .and_then(|e| error_description(&e))
                .unwrap_or_else(|| "视频加载失败".to_owned());
            emit_error(context, message);
        }
    }
}

fn extract_notification_error(notification: NonNull<NSNotification>) -> Option<String> {
    #[expect(
        unsafe_code,
        reason = "从 AVPlayerItemFailedToPlayToEndTimeNotification userInfo 读取 NSError"
    )]
    unsafe {
        let notif = notification.as_ref();
        let user_info = notif.userInfo()?;
        let key = NSString::from_str("AVPlayerItemFailedToPlayToEndTimeErrorKey");
        let err: Option<Retained<AnyObject>> = msg_send![&*user_info, objectForKey: &*key];
        err.and_then(|e| error_description(&e))
    }
}

fn error_description(err: &AnyObject) -> Option<String> {
    #[expect(unsafe_code, reason = "读取 NSError localizedDescription")]
    unsafe {
        let desc: Option<Retained<NSString>> = msg_send![err, localizedDescription];
        desc.map(|s| s.to_string())
    }
}
