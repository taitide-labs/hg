use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::Arc;

use nsh_core::JsonValue;
use nsh_platform_windows::windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_IGNORE, D2D1_PIXEL_FORMAT,
};
use nsh_platform_windows::windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_NONE, D2D1_BITMAP_PROPERTIES1, D2D1_INTERPOLATION_MODE_LINEAR,
    ID2D1Bitmap1, ID2D1DeviceContext,
};
use nsh_platform_windows::windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use nsh_platform_windows::windows::core::Interface;
use nsh_platform_windows::{ComponentContext, Components, NativeComponent};
use parking_lot::Mutex;

use crate::protocol::VIDEO_PLAYER_COMPONENT_NAME;

use super::commands::{MpvCommand, commands_for_attach};
use super::layout::compute_target_rect;
use super::mpv::{MpvEngine, emit_video_event};
use super::pool::FramePlane;
use super::state::WindowsPlayerState;

struct CachedBitmap {
    bitmap: ID2D1Bitmap1,
    device_ptr: usize,
    width: u32,
    height: u32,
    serial: u64,
}

struct WindowsVideoPlayer {
    context: ComponentContext,
    state: Arc<Mutex<WindowsPlayerState>>,
    engine: Option<MpvEngine>,
    cached_bitmap: RefCell<Option<CachedBitmap>>,
}

impl WindowsVideoPlayer {
    fn new(context: ComponentContext) -> Self {
        let state = Arc::new(Mutex::new(WindowsPlayerState::new()));
        let engine = match MpvEngine::acquire_or_create() {
            Ok(engine) => {
                engine.attach_sink(context.clone(), Arc::clone(&state));
                Some(engine)
            }
            Err(message) => {
                let initial_error = state.lock().record_error_once(message);
                if let Some(event) = initial_error {
                    emit_video_event(&context, event);
                }
                None
            }
        };

        Self {
            context,
            state,
            engine,
            cached_bitmap: RefCell::new(None),
        }
    }

    fn execute_commands(&self, commands: &[MpvCommand]) {
        let Some(engine) = &self.engine else {
            return;
        };
        for command in commands {
            if let Err(message) = engine.execute_command(command) {
                let event = self.state.lock().record_error_once(message);
                if let Some(ev) = event {
                    emit_video_event(&self.context, ev);
                }
            }
        }
    }

    fn ensure_engine_attached(&mut self) {
        if self.engine.is_some() {
            return;
        }
        if let Ok(engine) = MpvEngine::acquire_or_create() {
            engine.attach_sink(self.context.clone(), Arc::clone(&self.state));
            let attach_commands = commands_for_attach(self.state.lock().props());
            self.engine = Some(engine);
            self.execute_commands(&attach_commands);
        }
    }

    fn upload_or_reuse_bitmap(
        &self,
        dc: &ID2D1DeviceContext,
        plane: &FramePlane,
        serial: u64,
    ) -> Option<ID2D1Bitmap1> {
        let device_ptr = current_device_ptr(dc);
        let mut slot = self.cached_bitmap.borrow_mut();

        if let Some(cached) = slot.as_mut()
            && cached.device_ptr == device_ptr
            && cached.width == plane.width
            && cached.height == plane.height
        {
            if cached.serial == serial {
                return Some(cached.bitmap.clone());
            }
            #[expect(
                unsafe_code,
                reason = "pixels 大小经 FramePlane 校验匹配当前位图宽高与步长"
            )]
            let copy_res = unsafe {
                cached
                    .bitmap
                    .CopyFromMemory(None, plane.pixels.as_ptr().cast(), plane.stride)
            };
            if copy_res.is_ok() {
                cached.serial = serial;
                return Some(cached.bitmap.clone());
            }
        }

        *slot = None;
        let props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_IGNORE,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
            colorContext: ManuallyDrop::new(None),
        };
        let size = D2D_SIZE_U {
            width: plane.width,
            height: plane.height,
        };
        #[expect(
            unsafe_code,
            reason = "由 ID2D1DeviceContext 创建匹配 BGRA 帧尺寸的 Direct2D 位图"
        )]
        let created = unsafe {
            dc.CreateBitmap(
                size,
                Some(plane.pixels.as_ptr().cast()),
                plane.stride,
                &props,
            )
        };
        match created {
            Ok(bitmap) => {
                *slot = Some(CachedBitmap {
                    bitmap: bitmap.clone(),
                    device_ptr,
                    width: plane.width,
                    height: plane.height,
                    serial,
                });
                Some(bitmap)
            }
            Err(_) => None,
        }
    }
}

impl Drop for WindowsVideoPlayer {
    fn drop(&mut self) {
        self.cached_bitmap.borrow_mut().take();
        if let Some(engine) = self.engine.take() {
            engine.release_to_pool();
        }
    }
}

impl NativeComponent for WindowsVideoPlayer {
    fn draw(&self, dc: &ID2D1DeviceContext, size: (f32, f32)) {
        let Some(engine) = &self.engine else {
            return;
        };
        engine.set_viewport_size(size);

        let aspect = self.state.lock().aspect();
        let uploaded = engine.with_front_frame(|plane, serial| {
            let plane = plane?;
            let bitmap = self.upload_or_reuse_bitmap(dc, plane, serial)?;
            Some((bitmap, plane.width as f32, plane.height as f32))
        });

        let Some((bitmap, natural_w, natural_h)) = uploaded else {
            return;
        };

        let target = compute_target_rect(aspect, (natural_w, natural_h), size);
        if target.width <= 0.0 || target.height <= 0.0 {
            return;
        }
        let dest_rect = D2D_RECT_F {
            left: target.left,
            top: target.top,
            right: target.right(),
            bottom: target.bottom(),
        };
        #[expect(
            unsafe_code,
            reason = "在 NSH Direct2D 绘制上下文内输出同步自 libmpv 帧缓冲的位图"
        )]
        unsafe {
            dc.DrawBitmap(
                &bitmap,
                Some(&dest_rect),
                1.0,
                D2D1_INTERPOLATION_MODE_LINEAR,
                None,
                None,
            );
        }
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        let outcome = self.state.lock().apply_prop(name, value);
        if self.engine.is_none() && name == "src" {
            self.ensure_engine_attached();
        }
        if !outcome.commands.is_empty() {
            self.execute_commands(&outcome.commands);
        }
        if outcome.needs_redraw {
            self.context.redraw();
        }
    }
}

fn current_device_ptr(dc: &ID2D1DeviceContext) -> usize {
    #[expect(
        unsafe_code,
        reason = "读取当前 Direct2D Device 裸指针用于设备重建检测"
    )]
    let device = unsafe { dc.GetDevice() };
    device.map_or(0, |dev| dev.as_raw() as usize)
}

pub fn register(components: &Components) {
    components.register(
        VIDEO_PLAYER_COMPONENT_NAME,
        Rc::new(|context: ComponentContext| -> Box<dyn NativeComponent> {
            Box::new(WindowsVideoPlayer::new(context))
        }),
    );
}
