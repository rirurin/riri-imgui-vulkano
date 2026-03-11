use std::error::Error;
use std::sync::Arc;
use std::time::Instant;
use imgui::{BackendFlags, ConfigFlags, Context as ImContext, FontConfig, FontSource};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use vulkano::format::ClearValue;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::icon::Icon;
#[cfg(target_os = "windows")]
use winit::platform::windows::WinIcon;
use winit::window::{Window, WindowAttributes, WindowId};
use riri_imgui_vulkano::context::RendererContext;
use crate::clipboard::ClipboardSupport;
use crate::color::ColorConverter;
use crate::renderer::VulkanContext;

#[derive(Debug)]
pub(crate) struct App {
    window: Option<Arc<Box<dyn Window>>>,
    platform: Option<WinitPlatform>,
    imgui: Option<ImContext>,
    renderer: Option<VulkanContext>,
    last_frame: Instant,
    count: usize,
}

impl App {
    pub(crate) fn execute() {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        let _ = event_loop.run_app(Box::new(App::new()));
    }

    fn new() -> Self {
        Self {
            window: None,
            platform: None,
            imgui: None,
            renderer: None,
            last_frame: Instant::now(),
            count: 0,
        }
    }

    pub fn get_name(&self) -> &str {
        "Sample App (winit + vulkano + imgui-rs)"
    }

    pub fn get_window(&self) -> Arc<Box<dyn Window>> {
        self.window.as_ref().unwrap().clone()
    }

    pub fn get_platform(&self) -> &WinitPlatform {
        self.platform.as_ref().unwrap()
    }

    pub fn get_platform_mut(&mut self) -> &mut WinitPlatform {
        self.platform.as_mut().unwrap()
    }

    pub fn get_imgui(&self) -> &ImContext {
        self.imgui.as_ref().unwrap()
    }

    pub fn get_imgui_mut(&mut self) -> &mut ImContext {
        self.imgui.as_mut().unwrap()
    }
}

#[cfg(target_os = "windows")]
struct IconLookupWin32;
#[cfg(target_os = "windows")]
impl IconLookupWin32 {
    fn get() -> Result<Icon, Box<dyn Error>> {
        Ok(WinIcon::from_resource(0x65, None)?.into())
    }
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let icon = if cfg!(target_os = "windows") {
            Some(IconLookupWin32::get().unwrap())
        } else {
            None
        };
        let attr = WindowAttributes::default()
            .with_visible(false)
            .with_title(self.get_name())
            .with_window_icon(icon)
            .with_surface_size(Size::Physical(PhysicalSize::new(1280, 720)))
            .with_position(Position::Physical(PhysicalPosition::new(100, 100)));
        self.window = Some(Arc::new(event_loop.create_window(attr).unwrap()));
        self.imgui = Some(ImContext::create());
        self.get_imgui_mut().io_mut().config_flags |= ConfigFlags::DOCKING_ENABLE;
        self.get_imgui_mut().set_ini_filename(None);
        self.get_imgui_mut().set_clipboard_backend(ClipboardSupport::new().unwrap());
        self.platform = Some(WinitPlatform::new(self.get_imgui_mut()));
        self.platform.as_mut().unwrap().attach_window(
            self.imgui.as_mut().unwrap().io_mut(),
            self.window.as_ref().unwrap().as_ref().as_ref(),
            HiDpiMode::Rounded
        );
        self.get_imgui_mut().io_mut().mouse_pos = [0., 0.];
        let hidpi_factor = self.platform.as_ref().unwrap().hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        self.get_imgui_mut().fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig { size_pixels: font_size, ..FontConfig::default() }),
        }]);
        self.get_imgui_mut().io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
        self.renderer = Some(VulkanContext::new(
            RendererContext::new(event_loop, self.get_window(), Some(self.get_name().to_string())).unwrap(),
            self.get_window(),
            self.get_imgui_mut()
        ).unwrap());
        // We can honor the ImDrawCmd::VtxOffset field, allowing for large meshes.
        self.get_imgui_mut().io_mut().backend_flags |= BackendFlags::RENDERER_HAS_VTX_OFFSET;
        self.window.as_ref().unwrap().set_visible(true);
    }

    fn about_to_wait(&mut self, _: &dyn ActiveEventLoop) {
        let imgui = self.imgui.as_mut().unwrap();
        let platform = self.platform.as_mut().unwrap();
        let io = imgui.io_mut();
        let window = self.window.as_ref().unwrap().as_ref().as_ref();
        platform.prepare_frame(io, window).unwrap();
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let window = self.window.as_ref().unwrap();
        let renderer = self.renderer.as_mut().unwrap();
        let platform = self.platform.as_mut().unwrap();
        let imgui = self.imgui.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => { event_loop.exit(); },
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                imgui.io_mut().update_delta_time(now - self.last_frame);
                self.last_frame = now;
                // Start draw UI
                // let delta = imgui.io_mut().delta_time;
                let ui = imgui.new_frame();
                let mut show = true;
                // println!("delta: {}", delta);
                self.count = self.count.overflowing_add(1).0;
                ui.show_demo_window(&mut show);
                let draw_data = imgui.render();
                let clear_color = ColorConverter::hsv_to_rgb(
                    (self.count as f32 / 300.) % 1., 0.25, 0.6);
                if let ClearValue::Float(v) = &mut renderer.clear_color {
                    *v = [clear_color.x, clear_color.y, clear_color.z, 1.];
                }
                renderer.render_imgui(draw_data).unwrap();
                if renderer.present().unwrap() {
                    renderer.refresh(window.clone()).unwrap();
                }
            },
            WindowEvent::SurfaceResized(_) => {
                let io = imgui.io_mut();
                platform.handle_window_event(io, window.as_ref().as_ref(), &event);
                renderer.refresh(window.clone()).unwrap();
            },
            _ => {
                let io = imgui.io_mut();
                platform.handle_window_event(io, window.as_ref().as_ref(), &event)
            }
        }
    }
}