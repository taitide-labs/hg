use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;

use nsh_platform_windows::ComponentContext;
use nsh_platform_windows::windows::Win32::Foundation::{FreeLibrary, HMODULE};
use nsh_platform_windows::windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use nsh_platform_windows::windows::core::{PCSTR, PCWSTR, s, w};
use parking_lot::{Condvar, Mutex};

use crate::protocol::VideoPlayerEvent;

use super::commands::{DEFAULT_INIT_OPTIONS, MpvCommand, commands_for_recycle};
use super::layout::{MAX_RENDER_DIMENSION, clamp_render_dimensions};
use super::pool::{DEFAULT_POOL_CAPACITY, DoubleFrameBuffer, InstancePool};
use super::state::WindowsPlayerState;

const MPV_FORMAT_FLAG: c_int = 3;
const MPV_FORMAT_INT64: c_int = 4;
const MPV_FORMAT_DOUBLE: c_int = 5;

const MPV_EVENT_NONE: c_int = 0;
const MPV_EVENT_SHUTDOWN: c_int = 1;
const MPV_EVENT_END_FILE: c_int = 7;
const MPV_EVENT_VIDEO_RECONFIG: c_int = 17;
const MPV_EVENT_PROPERTY_CHANGE: c_int = 22;

const MPV_RENDER_PARAM_INVALID: c_int = 0;
const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
const MPV_RENDER_PARAM_SW_SIZE: c_int = 17;
const MPV_RENDER_PARAM_SW_FORMAT: c_int = 18;
const MPV_RENDER_PARAM_SW_STRIDE: c_int = 19;
const MPV_RENDER_PARAM_SW_POINTER: c_int = 20;

const MPV_RENDER_UPDATE_FRAME: u64 = 1;

const REPLY_TIME_POS: u64 = 1;
const REPLY_DURATION: u64 = 2;
const REPLY_DWIDTH: u64 = 3;
const REPLY_DHEIGHT: u64 = 4;

#[repr(C)]
pub struct MpvHandle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct MpvRenderContext {
    _private: [u8; 0],
}

#[repr(C)]
struct MpvEvent {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct MpvEventProperty {
    name: *const c_char,
    format: c_int,
    data: *mut c_void,
}

#[repr(C)]
struct MpvEventEndFile {
    reason: c_int,
    error: c_int,
}

#[repr(C)]
struct MpvRenderParam {
    kind: c_int,
    data: *mut c_void,
}

type MpvRenderUpdateFn = Option<unsafe extern "C" fn(*mut c_void)>;

struct MpvLibrary {
    create: unsafe extern "C" fn() -> *mut MpvHandle,
    initialize: unsafe extern "C" fn(*mut MpvHandle) -> c_int,
    terminate_destroy: unsafe extern "C" fn(*mut MpvHandle),
    set_option_string: unsafe extern "C" fn(*mut MpvHandle, *const c_char, *const c_char) -> c_int,
    set_property: unsafe extern "C" fn(*mut MpvHandle, *const c_char, c_int, *mut c_void) -> c_int,
    command: unsafe extern "C" fn(*mut MpvHandle, *const *const c_char) -> c_int,
    observe_property: unsafe extern "C" fn(*mut MpvHandle, u64, *const c_char, c_int) -> c_int,
    wait_event: unsafe extern "C" fn(*mut MpvHandle, f64) -> *const MpvEvent,
    wakeup: unsafe extern "C" fn(*mut MpvHandle),
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
    render_context_create: unsafe extern "C" fn(
        *mut *mut MpvRenderContext,
        *mut MpvHandle,
        *mut MpvRenderParam,
    ) -> c_int,
    render_context_set_update_callback:
        unsafe extern "C" fn(*mut MpvRenderContext, MpvRenderUpdateFn, *mut c_void),
    render_context_update: unsafe extern "C" fn(*mut MpvRenderContext) -> u64,
    render_context_render:
        unsafe extern "C" fn(*mut MpvRenderContext, *mut MpvRenderParam) -> c_int,
    render_context_free: unsafe extern "C" fn(*mut MpvRenderContext),
}

#[expect(
    unsafe_code,
    reason = "MpvLibrary 仅持有进程期内常驻 DLL 的 C 函数指针，可安全跨线程共享"
)]
unsafe impl Send for MpvLibrary {}
#[expect(
    unsafe_code,
    reason = "MpvLibrary 仅持有进程期内常驻 DLL 的 C 函数指针，可安全跨线程共享"
)]
unsafe impl Sync for MpvLibrary {}

static MPV_LIBRARY: OnceLock<Result<MpvLibrary, String>> = OnceLock::new();
static MPV_POOL: OnceLock<Mutex<InstancePool<MpvEngine>>> = OnceLock::new();

impl MpvLibrary {
    pub fn get() -> Result<&'static Self, String> {
        MPV_LIBRARY
            .get_or_init(Self::load_from_candidates)
            .as_ref()
            .map_err(Clone::clone)
    }

    fn load_from_candidates() -> Result<Self, String> {
        let candidates: [(&str, PCWSTR); 2] = [
            ("libmpv-2.dll", w!("libmpv-2.dll")),
            ("mpv-2.dll", w!("mpv-2.dll")),
        ];
        let mut errors = Vec::with_capacity(candidates.len());
        for (label, wide_name) in candidates {
            match Self::load_module(label, wide_name) {
                Ok(lib) => return Ok(lib),
                Err(err) => errors.push(err),
            }
        }
        Err(format!(
            "unable to load libmpv runtime ({})",
            errors.join("; ")
        ))
    }

    fn load_module(label: &str, wide_name: PCWSTR) -> Result<Self, String> {
        #[expect(
            unsafe_code,
            reason = "动态探测加载宿主环境中的 libmpv-2.dll 或 mpv-2.dll"
        )]
        let module = unsafe { LoadLibraryW(wide_name) }
            .map_err(|err| format!("{label}: LoadLibraryW failed ({err})"))?;

        match Self::resolve_all(label, module) {
            Ok(lib) => Ok(lib),
            Err(err) => {
                #[expect(unsafe_code, reason = "符号解析失败时释放未就绪的动态库句柄以避免泄漏")]
                let _freed = unsafe { FreeLibrary(module) };
                Err(err)
            }
        }
    }

    fn resolve_all(label: &str, module: HMODULE) -> Result<Self, String> {
        Ok(Self {
            create: resolve_fn(label, module, s!("mpv_create"))?,
            initialize: resolve_fn(label, module, s!("mpv_initialize"))?,
            terminate_destroy: resolve_fn(label, module, s!("mpv_terminate_destroy"))?,
            set_option_string: resolve_fn(label, module, s!("mpv_set_option_string"))?,
            set_property: resolve_fn(label, module, s!("mpv_set_property"))?,
            command: resolve_fn(label, module, s!("mpv_command"))?,
            observe_property: resolve_fn(label, module, s!("mpv_observe_property"))?,
            wait_event: resolve_fn(label, module, s!("mpv_wait_event"))?,
            wakeup: resolve_fn(label, module, s!("mpv_wakeup"))?,
            error_string: resolve_fn(label, module, s!("mpv_error_string"))?,
            render_context_create: resolve_fn(label, module, s!("mpv_render_context_create"))?,
            render_context_set_update_callback: resolve_fn(
                label,
                module,
                s!("mpv_render_context_set_update_callback"),
            )?,
            render_context_update: resolve_fn(label, module, s!("mpv_render_context_update"))?,
            render_context_render: resolve_fn(label, module, s!("mpv_render_context_render"))?,
            render_context_free: resolve_fn(label, module, s!("mpv_render_context_free"))?,
        })
    }

    fn format_error(&self, code: c_int) -> String {
        #[expect(unsafe_code, reason = "mpv_error_string 返回静态只读 C 字符串")]
        let ptr = unsafe { (self.error_string)(code) };
        if ptr.is_null() {
            return format!("mpv error {code}");
        }
        #[expect(
            unsafe_code,
            reason = "已校验非空后读取 mpv_error_string 静态 UTF-8 字符串"
        )]
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

fn resolve_fn<T>(label: &str, module: HMODULE, symbol: PCSTR) -> Result<T, String> {
    #[expect(
        unsafe_code,
        reason = "通过 GetProcAddress 获取已加载模块导出符号并按既定 C ABI 函数指针解释"
    )]
    let resolved = unsafe {
        let addr = GetProcAddress(module, symbol);
        addr.map(|proc_addr| std::mem::transmute_copy(&proc_addr))
    };
    resolved.ok_or_else(|| {
        #[expect(
            unsafe_code,
            reason = "symbol 来自 windows::core::s! 静态 C 字符串常量"
        )]
        let sym_name = unsafe { CStr::from_ptr(symbol.as_ptr().cast()) }.to_string_lossy();
        format!("{label}: missing symbol {sym_name}")
    })
}

#[derive(Clone, Copy)]
struct RawMpvHandle(*mut MpvHandle);

impl RawMpvHandle {
    const fn as_ptr(self) -> *mut MpvHandle {
        self.0
    }
}

#[expect(
    unsafe_code,
    reason = "libmpv 文档明确 mpv_handle 线程安全，允许工作线程与主线程并发调用"
)]
unsafe impl Send for RawMpvHandle {}
#[expect(
    unsafe_code,
    reason = "libmpv 文档明确 mpv_handle 线程安全，允许工作线程与主线程并发调用"
)]
unsafe impl Sync for RawMpvHandle {}

#[derive(Clone, Copy)]
struct RawRenderContext(*mut MpvRenderContext);

#[expect(
    unsafe_code,
    reason = "mpv_render_context 仅由专用渲染线程更新与渲染，并在 Drop 时安全释放"
)]
unsafe impl Send for RawRenderContext {}
#[expect(
    unsafe_code,
    reason = "mpv_render_context 仅由专用渲染线程更新与渲染，并在 Drop 时安全释放"
)]
unsafe impl Sync for RawRenderContext {}

#[derive(Default)]
struct RenderSignalState {
    dirty: bool,
    shutdown: bool,
}

struct RenderSignal {
    state: Mutex<RenderSignalState>,
    cv: Condvar,
}

impl RenderSignal {
    fn new() -> Self {
        Self {
            state: Mutex::new(RenderSignalState::default()),
            cv: Condvar::new(),
        }
    }

    fn mark_dirty(&self) {
        let mut guard = self.state.lock();
        guard.dirty = true;
        self.cv.notify_one();
    }

    fn signal_shutdown(&self) {
        let mut guard = self.state.lock();
        guard.shutdown = true;
        self.cv.notify_all();
    }

    fn wait_for_work(&self) -> bool {
        let mut guard = self.state.lock();
        while !guard.dirty && !guard.shutdown {
            self.cv.wait(&mut guard);
        }
        if guard.shutdown {
            return false;
        }
        guard.dirty = false;
        true
    }
}

#[derive(Default)]
struct RenderDimensions {
    video: (u32, u32),
    viewport: (f32, f32),
}

pub struct ActiveSink {
    pub context: ComponentContext,
    pub state: Arc<Mutex<WindowsPlayerState>>,
}

struct EngineShared {
    sink: Mutex<Option<ActiveSink>>,
    frame_buffer: Mutex<DoubleFrameBuffer>,
    dimensions: Mutex<RenderDimensions>,
    shutdown: AtomicBool,
}

pub struct MpvEngine {
    lib: &'static MpvLibrary,
    mpv: RawMpvHandle,
    render_ctx: RawRenderContext,
    shared: Arc<EngineShared>,
    render_signal_raw: *const RenderSignal,
    render_signal: Arc<RenderSignal>,
    render_thread: Option<JoinHandle<()>>,
    event_thread: Option<JoinHandle<()>>,
}

#[expect(
    unsafe_code,
    reason = "MpvEngine 持有的 render_signal_raw 指向同结构体管理的 Arc<RenderSignal>，可安全在线程池中转移所有权"
)]
unsafe impl Send for MpvEngine {}

impl MpvEngine {
    pub fn acquire_or_create() -> Result<Self, String> {
        let pool = MPV_POOL.get_or_init(|| Mutex::new(InstancePool::new(DEFAULT_POOL_CAPACITY)));
        if let Some(engine) = pool.lock().acquire() {
            return Ok(engine);
        }
        Self::create_new()
    }

    pub fn release_to_pool(mut self) {
        self.detach_sink();
        self.reset_for_pool();
        let pool = MPV_POOL.get_or_init(|| Mutex::new(InstancePool::new(DEFAULT_POOL_CAPACITY)));
        let overflow = pool.lock().release(self);
        drop(overflow);
    }

    pub fn attach_sink(&self, context: ComponentContext, state: Arc<Mutex<WindowsPlayerState>>) {
        *self.shared.sink.lock() = Some(ActiveSink { context, state });
        *self.shared.dimensions.lock() = RenderDimensions::default();
        self.shared.frame_buffer.lock().reset();
    }

    pub fn detach_sink(&self) {
        *self.shared.sink.lock() = None;
    }

    pub fn set_viewport_size(&self, size: (f32, f32)) {
        let mut dims = self.shared.dimensions.lock();
        dims.viewport = size;
    }

    pub fn with_front_frame<R>(
        &self,
        f: impl FnOnce(Option<&super::pool::FramePlane>, u64) -> R,
    ) -> R {
        let buffer = self.shared.frame_buffer.lock();
        f(buffer.front(), buffer.generation())
    }

    pub fn execute_command(&self, command: &MpvCommand) -> Result<(), String> {
        match command {
            MpvCommand::LoadFile { url } => {
                let c_url = CString::new(url.as_str())
                    .map_err(|_| "stream url contains interior nul byte".to_owned())?;
                run_mpv_command(
                    self.lib,
                    self.mpv,
                    &[c"loadfile".as_ptr(), c_url.as_ptr(), c"replace".as_ptr()],
                )
            }
            MpvCommand::Stop => {
                self.shared.frame_buffer.lock().reset();
                run_mpv_command(self.lib, self.mpv, &[c"stop".as_ptr()])
            }
            MpvCommand::SetPaused(paused) => {
                set_flag_property(self.lib, self.mpv, c"pause", *paused)
            }
            MpvCommand::SetRate(rate) => set_double_property(self.lib, self.mpv, c"speed", *rate),
            MpvCommand::SetVolume(volume) => {
                set_double_property(self.lib, self.mpv, c"volume", *volume)
            }
            MpvCommand::SetMuted(muted) => set_flag_property(self.lib, self.mpv, c"mute", *muted),
            MpvCommand::SeekAbsolute(position_secs) => {
                let pos_text = position_secs.to_string();
                let c_pos = CString::new(pos_text)
                    .map_err(|_| "seek position contains interior nul byte".to_owned())?;
                run_mpv_command(
                    self.lib,
                    self.mpv,
                    &[c"seek".as_ptr(), c_pos.as_ptr(), c"absolute".as_ptr()],
                )
            }
        }
    }

    fn reset_for_pool(&mut self) {
        for command in commands_for_recycle() {
            let _ignored = self.execute_command(&command);
        }
        *self.shared.dimensions.lock() = RenderDimensions::default();
        self.shared.frame_buffer.lock().reset();
    }

    fn create_new() -> Result<Self, String> {
        let lib = MpvLibrary::get()?;
        let mpv = create_configured_handle(lib)?;
        let render_ctx = match create_sw_render_context(lib, mpv) {
            Ok(ctx) => ctx,
            Err(err) => {
                #[expect(
                    unsafe_code,
                    reason = "渲染上下文创建失败时立即销毁刚创建的 mpv_handle"
                )]
                unsafe {
                    (lib.terminate_destroy)(mpv.0);
                }
                return Err(err);
            }
        };

        let render_signal = Arc::new(RenderSignal::new());
        let render_signal_raw = Arc::into_raw(Arc::clone(&render_signal));
        #[expect(
            unsafe_code,
            reason = "为 mpv_render_context 注册更新通知回调并传入独立持有的 Arc 指针"
        )]
        unsafe {
            (lib.render_context_set_update_callback)(
                render_ctx.0,
                Some(on_render_update),
                render_signal_raw.cast_mut().cast(),
            );
        }

        let shared = Arc::new(EngineShared {
            sink: Mutex::new(None),
            frame_buffer: Mutex::new(DoubleFrameBuffer::new()),
            dimensions: Mutex::new(RenderDimensions::default()),
            shutdown: AtomicBool::new(false),
        });

        let render_thread = spawn_render_worker(
            lib,
            render_ctx,
            Arc::clone(&render_signal),
            Arc::clone(&shared),
        );
        let event_thread =
            spawn_event_worker(lib, mpv, Arc::clone(&render_signal), Arc::clone(&shared));

        Ok(Self {
            lib,
            mpv,
            render_ctx,
            shared,
            render_signal_raw,
            render_signal,
            render_thread: Some(render_thread),
            event_thread: Some(event_thread),
        })
    }
}

impl Drop for MpvEngine {
    fn drop(&mut self) {
        self.detach_sink();
        self.shared.shutdown.store(true, Ordering::Release);
        self.render_signal.signal_shutdown();

        #[expect(
            unsafe_code,
            reason = "先清除更新回调并唤醒事件循环，确保工作线程可立即退出"
        )]
        unsafe {
            (self.lib.render_context_set_update_callback)(self.render_ctx.0, None, ptr::null_mut());
            (self.lib.wakeup)(self.mpv.0);
        }

        if let Some(handle) = self.render_thread.take() {
            let _joined = handle.join();
        }
        if let Some(handle) = self.event_thread.take() {
            let _joined = handle.join();
        }

        #[expect(
            unsafe_code,
            reason = "工作线程均已退出且已清零回调，按 libmpv 规范先释放 render_context 再销毁 mpv_handle，并回收回调 Arc"
        )]
        unsafe {
            (self.lib.render_context_free)(self.render_ctx.0);
            (self.lib.terminate_destroy)(self.mpv.0);
            drop(Arc::from_raw(self.render_signal_raw));
        }
    }
}

pub fn emit_video_event(context: &ComponentContext, event: VideoPlayerEvent) {
    let (name, payload) = event.into_name_and_payload();
    context.emit(name, payload);
}

#[expect(
    unsafe_code,
    reason = "libmpv 渲染更新回调入口，仅设置脏标记并唤醒渲染线程，不调用任何 mpv API"
)]
unsafe extern "C" fn on_render_update(cb_ctx: *mut c_void) {
    if cb_ctx.is_null() {
        return;
    }
    let signal = unsafe { &*(cb_ctx.cast::<RenderSignal>()) };
    signal.mark_dirty();
}

fn create_configured_handle(lib: &MpvLibrary) -> Result<RawMpvHandle, String> {
    #[expect(unsafe_code, reason = "调用 mpv_create 创建独立播放器实例句柄")]
    let raw = unsafe { (lib.create)() };
    if raw.is_null() {
        return Err("mpv_create returned null".to_owned());
    }
    let mpv = RawMpvHandle(raw);

    for option in DEFAULT_INIT_OPTIONS {
        let Ok(c_name) = CString::new(option.name) else {
            continue;
        };
        let Ok(c_val) = CString::new(option.value) else {
            continue;
        };
        #[expect(unsafe_code, reason = "在 mpv_initialize 之前写入已知有效的初始化选项")]
        let rc = unsafe { (lib.set_option_string)(mpv.0, c_name.as_ptr(), c_val.as_ptr()) };
        if rc < 0 {
            let msg = format!(
                "mpv option {}={} failed: {}",
                option.name,
                option.value,
                lib.format_error(rc)
            );
            #[expect(unsafe_code, reason = "配置失败时释放未初始化完成的 mpv_handle")]
            unsafe {
                (lib.terminate_destroy)(mpv.0);
            }
            return Err(msg);
        }
    }

    #[expect(unsafe_code, reason = "完成选项设置后调用 mpv_initialize 初始化实例")]
    let init_rc = unsafe { (lib.initialize)(mpv.0) };
    if init_rc < 0 {
        let msg = format!("mpv_initialize failed: {}", lib.format_error(init_rc));
        #[expect(unsafe_code, reason = "初始化失败时销毁 mpv_handle")]
        unsafe {
            (lib.terminate_destroy)(mpv.0);
        }
        return Err(msg);
    }

    observe_default_properties(lib, mpv);
    Ok(mpv)
}

fn observe_default_properties(lib: &MpvLibrary, mpv: RawMpvHandle) {
    let observed: [(u64, &CStr, c_int); 4] = [
        (REPLY_TIME_POS, c"time-pos", MPV_FORMAT_DOUBLE),
        (REPLY_DURATION, c"duration", MPV_FORMAT_DOUBLE),
        (REPLY_DWIDTH, c"dwidth", MPV_FORMAT_INT64),
        (REPLY_DHEIGHT, c"dheight", MPV_FORMAT_INT64),
    ];
    for (userdata, name, format) in observed {
        #[expect(unsafe_code, reason = "向已初始化 mpv_handle 注册静态属性观测")]
        let _ = unsafe { (lib.observe_property)(mpv.0, userdata, name.as_ptr(), format) };
    }
}

fn create_sw_render_context(
    lib: &MpvLibrary,
    mpv: RawMpvHandle,
) -> Result<RawRenderContext, String> {
    let api_type = c"sw";
    let mut params = [
        MpvRenderParam {
            kind: MPV_RENDER_PARAM_API_TYPE,
            data: api_type.as_ptr().cast_mut().cast(),
        },
        MpvRenderParam {
            kind: MPV_RENDER_PARAM_INVALID,
            data: ptr::null_mut(),
        },
    ];
    let mut ctx: *mut MpvRenderContext = ptr::null_mut();
    #[expect(unsafe_code, reason = "基于已初始化 mpv_handle 创建软件帧渲染上下文")]
    let rc = unsafe { (lib.render_context_create)(&mut ctx, mpv.0, params.as_mut_ptr()) };
    if rc < 0 || ctx.is_null() {
        return Err(format!(
            "mpv_render_context_create(sw) failed: {}",
            lib.format_error(rc)
        ));
    }
    Ok(RawRenderContext(ctx))
}

fn spawn_render_worker(
    lib: &'static MpvLibrary,
    render_ctx: RawRenderContext,
    signal: Arc<RenderSignal>,
    shared: Arc<EngineShared>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while signal.wait_for_work() {
            if shared.shutdown.load(Ordering::Acquire) {
                break;
            }
            #[expect(
                unsafe_code,
                reason = "专用渲染线程调用 mpv_render_context_update 检查新帧"
            )]
            let flags = unsafe { (lib.render_context_update)(render_ctx.0) };
            if (flags & MPV_RENDER_UPDATE_FRAME) == 0 {
                continue;
            }
            render_single_frame(lib, render_ctx, &shared);
        }
    })
}

fn render_single_frame(lib: &MpvLibrary, render_ctx: RawRenderContext, shared: &EngineShared) {
    let context = {
        let guard = shared.sink.lock();
        guard.as_ref().map(|sink| sink.context.clone())
    };
    let Some(context) = context else {
        return;
    };

    let (width, height) = resolve_render_dimensions(&shared.dimensions);
    let Some(mut target) = shared
        .frame_buffer
        .lock()
        .checkout_render_target(width, height)
    else {
        return;
    };

    let mut sw_size: [c_int; 2] = [
        i32::try_from(target.width).unwrap_or(i32::MAX),
        i32::try_from(target.height).unwrap_or(i32::MAX),
    ];
    let mut stride = usize::try_from(target.stride).unwrap_or(0);
    let sw_format = c"bgra";
    let mut params = [
        MpvRenderParam {
            kind: MPV_RENDER_PARAM_SW_SIZE,
            data: sw_size.as_mut_ptr().cast(),
        },
        MpvRenderParam {
            kind: MPV_RENDER_PARAM_SW_FORMAT,
            data: sw_format.as_ptr().cast_mut().cast(),
        },
        MpvRenderParam {
            kind: MPV_RENDER_PARAM_SW_STRIDE,
            data: (&raw mut stride).cast(),
        },
        MpvRenderParam {
            kind: MPV_RENDER_PARAM_SW_POINTER,
            data: target.pixels.as_mut_ptr().cast(),
        },
        MpvRenderParam {
            kind: MPV_RENDER_PARAM_INVALID,
            data: ptr::null_mut(),
        },
    ];

    #[expect(
        unsafe_code,
        reason = "将当前视频帧光栅化到独立 BGRA 缓冲并保证尺寸与跨度匹配"
    )]
    let rc = unsafe { (lib.render_context_render)(render_ctx.0, params.as_mut_ptr()) };
    if rc >= 0 {
        shared.frame_buffer.lock().publish_rendered_frame(target);
        context.redraw();
    } else {
        shared.frame_buffer.lock().return_unused_target(target);
    }
}

fn resolve_render_dimensions(dimensions: &Mutex<RenderDimensions>) -> (u32, u32) {
    let dims = dimensions.lock();
    if dims.video.0 > 0 && dims.video.1 > 0 {
        return clamp_render_dimensions(dims.video, MAX_RENDER_DIMENSION);
    }
    let vw = dims.viewport.0.round();
    let vh = dims.viewport.1.round();
    if vw.is_finite() && vh.is_finite() && vw >= 2.0 && vh >= 2.0 {
        return clamp_render_dimensions((vw as u32, vh as u32), MAX_RENDER_DIMENSION);
    }
    (1280, 720)
}

fn spawn_event_worker(
    lib: &'static MpvLibrary,
    mpv: RawMpvHandle,
    signal: Arc<RenderSignal>,
    shared: Arc<EngineShared>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mpv_handle = mpv;
        while !shared.shutdown.load(Ordering::Acquire) {
            #[expect(
                unsafe_code,
                reason = "事件线程轮询 mpv_wait_event，返回值指针在下一次调用前有效"
            )]
            let event_ptr = unsafe { (lib.wait_event)(mpv_handle.as_ptr(), 0.25) };
            if event_ptr.is_null() {
                continue;
            }
            #[expect(
                unsafe_code,
                reason = "已校验非空，按 MpvEvent C ABI 读取事件类型与载荷"
            )]
            let event = unsafe { &*event_ptr };
            match event.event_id {
                MPV_EVENT_NONE => continue,
                MPV_EVENT_SHUTDOWN => break,
                MPV_EVENT_VIDEO_RECONFIG => signal.mark_dirty(),
                MPV_EVENT_PROPERTY_CHANGE => {
                    handle_property_event(event, &signal, &shared);
                }
                MPV_EVENT_END_FILE => {
                    handle_end_file_event(lib, event, &shared);
                }
                _ => {}
            }
        }
    })
}

fn handle_property_event(event: &MpvEvent, signal: &RenderSignal, shared: &EngineShared) {
    if event.data.is_null() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "MPV_EVENT_PROPERTY_CHANGE 的 data 指向 MpvEventProperty"
    )]
    let prop = unsafe { &*(event.data.cast::<MpvEventProperty>()) };
    if prop.data.is_null() {
        return;
    }

    match (event.reply_userdata, prop.format) {
        (REPLY_TIME_POS, MPV_FORMAT_DOUBLE) => {
            #[expect(unsafe_code, reason = "MPV_FORMAT_DOUBLE 的 data 指向 f64")]
            let secs = unsafe { *(prop.data.cast::<f64>()) };
            dispatch_state_event(shared, |state| state.on_time_pos(secs));
        }
        (REPLY_DURATION, MPV_FORMAT_DOUBLE) => {
            #[expect(unsafe_code, reason = "MPV_FORMAT_DOUBLE 的 data 指向 f64")]
            let secs = unsafe { *(prop.data.cast::<f64>()) };
            dispatch_state_event(shared, |state| state.on_duration(secs));
        }
        (REPLY_DWIDTH, MPV_FORMAT_INT64) => {
            #[expect(unsafe_code, reason = "MPV_FORMAT_INT64 的 data 指向 i64")]
            let width = unsafe { *(prop.data.cast::<i64>()) };
            update_video_dimension(shared, signal, true, width);
        }
        (REPLY_DHEIGHT, MPV_FORMAT_INT64) => {
            #[expect(unsafe_code, reason = "MPV_FORMAT_INT64 的 data 指向 i64")]
            let height = unsafe { *(prop.data.cast::<i64>()) };
            update_video_dimension(shared, signal, false, height);
        }
        _ => {}
    }
}

fn update_video_dimension(
    shared: &EngineShared,
    signal: &RenderSignal,
    is_width: bool,
    raw_value: i64,
) {
    let clamped = u32::try_from(raw_value.max(0)).unwrap_or(u32::MAX);
    let changed = {
        let mut dims = shared.dimensions.lock();
        let target = if is_width {
            &mut dims.video.0
        } else {
            &mut dims.video.1
        };
        if *target == clamped {
            false
        } else {
            *target = clamped;
            true
        }
    };
    if changed {
        signal.mark_dirty();
    }
}

fn handle_end_file_event(lib: &MpvLibrary, event: &MpvEvent, shared: &EngineShared) {
    if event.data.is_null() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "MPV_EVENT_END_FILE 的 data 指向 MpvEventEndFile"
    )]
    let end_file = unsafe { &*(event.data.cast::<MpvEventEndFile>()) };
    let detail = if end_file.error < 0 {
        Some(lib.format_error(end_file.error))
    } else {
        None
    };
    dispatch_state_event(shared, |state| {
        state.on_end_file(end_file.reason, end_file.error, detail.as_deref())
    });
}

fn dispatch_state_event(
    shared: &EngineShared,
    update: impl FnOnce(&mut WindowsPlayerState) -> Option<VideoPlayerEvent>,
) {
    let emitted = {
        let sink_guard = shared.sink.lock();
        let Some(sink) = sink_guard.as_ref() else {
            return;
        };
        let mut state_guard = sink.state.lock();
        let event = update(&mut state_guard);
        event.map(|ev| (sink.context.clone(), ev))
    };
    if let Some((context, event)) = emitted {
        emit_video_event(&context, event);
    }
}

fn run_mpv_command(
    lib: &MpvLibrary,
    mpv: RawMpvHandle,
    args: &[*const c_char],
) -> Result<(), String> {
    let mut terminated = Vec::with_capacity(args.len() + 1);
    terminated.extend_from_slice(args);
    terminated.push(ptr::null());
    #[expect(
        unsafe_code,
        reason = "调用 mpv_command 并传入以 NULL 结尾的 C 字符串指针数组"
    )]
    let rc = unsafe { (lib.command)(mpv.0, terminated.as_ptr()) };
    if rc < 0 {
        return Err(lib.format_error(rc));
    }
    Ok(())
}

fn set_flag_property(
    lib: &MpvLibrary,
    mpv: RawMpvHandle,
    name: &CStr,
    value: bool,
) -> Result<(), String> {
    let mut flag: c_int = c_int::from(value);
    #[expect(
        unsafe_code,
        reason = "调用 mpv_set_property 写入 MPV_FORMAT_FLAG 属性"
    )]
    let rc = unsafe {
        (lib.set_property)(
            mpv.0,
            name.as_ptr(),
            MPV_FORMAT_FLAG,
            (&raw mut flag).cast(),
        )
    };
    if rc < 0 {
        return Err(lib.format_error(rc));
    }
    Ok(())
}

fn set_double_property(
    lib: &MpvLibrary,
    mpv: RawMpvHandle,
    name: &CStr,
    mut value: f64,
) -> Result<(), String> {
    #[expect(
        unsafe_code,
        reason = "调用 mpv_set_property 写入 MPV_FORMAT_DOUBLE 属性"
    )]
    let rc = unsafe {
        (lib.set_property)(
            mpv.0,
            name.as_ptr(),
            MPV_FORMAT_DOUBLE,
            (&raw mut value).cast(),
        )
    };
    if rc < 0 {
        return Err(lib.format_error(rc));
    }
    Ok(())
}
