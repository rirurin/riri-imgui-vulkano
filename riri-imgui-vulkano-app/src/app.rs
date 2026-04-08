use crate::camera::Camera;
use crate::color::ColorConverter;
use crate::renderer::context::VulkanContext;
use crate::result::Result;
use gilrs_imgui_support::debug::GamepadVisualDebug;
use gilrs_imgui_support::state::{GamepadBuilder, GamepadState};
use glam::{U8Vec4, Vec2, Vec3};
use image::ImageFormat;
use imgui::{BackendFlags, ConfigFlags, Context as ImContext, FontGlyphRanges, FontId, ImColor32};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use riri_imgui_vulkano::context::RendererContext;
use riri_imgui_vulkano::vertex::{AppDrawData3D, AppVertex3D};
use riri_inspector_components::clipboard::ClipboardSupport;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Instant;
use vulkano::format::ClearValue;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::icon::RgbaIcon;
#[cfg(target_os = "windows")]
use winit::platform::windows::WinIcon;
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Debug)]
pub(crate) struct App {
    window: Option<Arc<Box<dyn Window>>>,
    platform: Option<WinitPlatform>,
    imgui: Option<ImContext>,
    renderer: Option<VulkanContext>,
    fonts: HashMap<String, FontId>,

    camera: Camera,
    data3d: AppDrawData3D,
    gamepad: GamepadState,

    last_frame: Instant,
    time_elapsed: f32,
    count: usize,
}

impl App {
    pub(crate) fn execute() {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        let _ = event_loop.run_app(Box::new(App::new().unwrap()));
    }

    fn new() -> Result<Self> {
        Ok(Self {
            window: None,
            platform: None,
            imgui: None,
            renderer: None,
            fonts: HashMap::new(),
            last_frame: Instant::now(),
            camera: Camera::new(),
            data3d: Self::get_sample_data(),
            gamepad: GamepadBuilder::new()
                .set_axis_to_btn(0.5, 0.4)
                // Invert the inverse_y setting to account for flipped y axis clip space.
                // This is not necessary with OpenGL or DirectX
                .invert_y(true)
                .build()?,
            time_elapsed: 0.,
            count: 0,
        })
    }

    pub fn get_name(&self) -> &str {
        "Sample App (winit + vulkano + imgui-rs)"
    }

    pub fn get_window(&self) -> Arc<Box<dyn Window>> {
        self.window.as_ref().unwrap().clone()
    }

    #[allow(dead_code)]
    pub fn get_platform(&self) -> &WinitPlatform {
        self.platform.as_ref().unwrap()
    }

    #[allow(dead_code)]
    pub fn get_platform_mut(&mut self) -> &mut WinitPlatform {
        self.platform.as_mut().unwrap()
    }

    #[allow(dead_code)]
    pub fn get_imgui(&self) -> &ImContext {
        self.imgui.as_ref().unwrap()
    }

    pub fn get_imgui_mut(&mut self) -> &mut ImContext {
        self.imgui.as_mut().unwrap()
    }

    pub(crate) fn get_sample_data() -> AppDrawData3D {
        let vertices = vec![
            // Front
            AppVertex3D::pos_color(Vec3::new( -0.5,  -0.5, 0.5 ), U8Vec4::new(0xff, 0x00, 0x00, 0xff)),
            AppVertex3D::pos_color(Vec3::new(  0.5,  -0.5, 0.5 ), U8Vec4::new(0x00, 0xff, 0x00, 0xff)),
            AppVertex3D::pos_color(Vec3::new( -0.5,   0.5, 0.5 ), U8Vec4::new(0x00, 0x00, 0xff, 0xff)),
            AppVertex3D::pos_color(Vec3::new(  0.5,   0.5, 0.5 ), U8Vec4::new(0xff, 0xff, 0x00, 0xff)),

            // Back
            AppVertex3D::pos_color(Vec3::new( -0.5,  -0.5, -0.5 ), U8Vec4::new(0x00, 0xff, 0xff, 0xff)),
            AppVertex3D::pos_color(Vec3::new(  0.5,  -0.5, -0.5 ), U8Vec4::new(0xff, 0x00, 0xff, 0xff)),
            AppVertex3D::pos_color(Vec3::new( -0.5,   0.5, -0.5 ), U8Vec4::new(0x00, 0x00, 0x00, 0xff)),
            AppVertex3D::pos_color(Vec3::new(  0.5,   0.5, -0.5 ), U8Vec4::new(0xff, 0xff, 0xff, 0xff)),
        ];

        let indices = vec![
            //Top
            7u32, 6, 2,
            2, 3, 7,

            //Bottom
            0, 4, 5,
            5, 1, 0,

            //Left
            0, 2, 6,
            6, 4, 0,

            //Right
            7, 3, 1,
            1, 5, 7,

            //Front
            3, 2, 0,
            0, 1, 3,

            //Back
            4, 6, 7,
            7, 5, 4
        ];
        AppDrawData3D::new(vertices, indices)
    }
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        #[cfg(target_os = "windows")]
        let icon = Some(WinIcon::from_resource(0x65, None).unwrap().into());
        #[cfg(not(target_os = "windows"))]
        let icon = {
            let icon_path = std::env::current_exe().unwrap().parent().unwrap().join("appicon.png");
            let app_icon = image::ImageReader::with_format(BufReader::new(File::open(icon_path).unwrap()), ImageFormat::Png);
            let app_icon= app_icon.decode().unwrap();
            Some(RgbaIcon::new(app_icon.as_rgba8().unwrap().to_vec(), app_icon.width(), app_icon.height()).unwrap().into())
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
        self.add_font("NotoSansCJKjp-Medium.otf", FontGlyphRanges::japanese(), 15.).unwrap();
        self.add_font("LibreBodoni-Bold.ttf", FontGlyphRanges::japanese(), 60.).unwrap();
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
                let delta_time = imgui.io().delta_time;
                self.time_elapsed += delta_time;
                self.last_frame = now;
                self.count = self.count.overflowing_add(1).0;
                self.gamepad.update(imgui);
                // Start draw UI
                let ui = imgui.new_frame();
                self.camera.update(ui, delta_time);
                let mut show = true;
                if let Some(main) = ui.begin_main_menu_bar() {
                    main.end()
                }
                AppDebugInfo::new(&self.fonts, ui, &self.camera, window.clone()).draw();
                GamepadVisualDebug::new(&self.gamepad)
                    .top_left(Vec2::new(10., 20.))
                    .build(ui);
                ui.show_demo_window(&mut show);
                let draw_data = imgui.render();
                let clear_color = ColorConverter::hsv_to_rgb(
                    (self.count as f32 / 300.) % 1., 0.25, 0.35);
                if let ClearValue::Float(v) = &mut renderer.clear_color {
                    *v = [clear_color.x, clear_color.y, clear_color.z, 1.];
                }
                renderer.render(
                    draw_data, &self.data3d, &self.camera, self.time_elapsed).unwrap();
                renderer.refresh(window.clone()).unwrap();
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

impl App {
    pub fn add_font(&mut self, name: &str, range: FontGlyphRanges, size: f32)
    -> Result<()> {
        let font_path = std::env::current_exe().unwrap()
            .parent().unwrap().join("data");
        let key =  name.rsplit_once(".")
            .map_or_else(|| name, |(name, _)| name).to_owned();
        self.fonts.insert(
            key,
            riri_inspector_components::font::load_font(
                self.imgui.as_mut().unwrap(),
                font_path.join(name),
                range,
                size
            )?
        );
        Ok(())
    }
}

#[derive(Debug)]
pub struct AppDebugInfo<'a> {
    fonts: &'a HashMap<String, FontId>,
    ui: &'a mut imgui::Ui,
    camera: &'a Camera,
    window: Arc<Box<dyn Window>>
}

impl<'a> AppDebugInfo<'a> {
    pub(crate) fn new(
        fonts: &'a HashMap<String, FontId>,
        ui: &'a mut imgui::Ui,
        camera: &'a Camera,
        window: Arc<Box<dyn Window>>
    ) -> Self {
        Self { fonts, ui, camera, window }
    }

    pub(crate) fn draw(&self) {
        let tf_id = *self.fonts.get("LibreBodoni-Bold").unwrap();
        let title_font = self.ui.fonts().get_font(tf_id).unwrap();
        let m_id = *self.fonts.get("NotoSansCJKjp-Medium").unwrap();
        let main_font = self.ui.fonts().get_font(m_id).unwrap();

        let debug_title = "Vulkano Test App";
        let debug_info = format!(
            "Version {}, Git Commit {}, Build Date {}",
            crate::version::RELOADED_VERSION,
            crate::version::COMMIT_HASH,
            crate::version::COMPILE_DATE
        );

        let position_info = format!(
            "Lookat: {} -> {}, Pan {}, Pitch {}, Roll {}",
            self.camera.eye,
            self.camera.lookat,
            self.camera.pan,
            self.camera.pitch,
            self.camera.roll
        );

        let window_dims = Vec2::from_array(self.window.surface_size().into());
        let title_length = debug_title.chars().map(|c| title_font.get_glyph(c).advance_x).sum::<f32>();
        let title_pos = [window_dims.x - (title_length + 20.), window_dims.y - (title_font.font_size + main_font.font_size * 2.)];
        let info_length = debug_info.chars().map(|c| main_font.get_glyph(c).advance_x).sum::<f32>();
        let info_pos = [ window_dims.x - (info_length + 20.), window_dims.y - (main_font.font_size + main_font.font_size / 2.)];

        let position_length = position_info.chars().map(|c| main_font.get_glyph(c).advance_x).sum::<f32>();
        let position_pos = [ window_dims[0] - (position_length + (main_font.font_size * 2.)), 0.];

        let debug_subtitle = ImColor32::from_rgba(255, 255, 255, 127);
        let title_token = self.ui.push_font(tf_id);
        self.ui.get_background_draw_list().add_text(title_pos, debug_subtitle, debug_title);
        title_token.pop();
        let body_token = self.ui.push_font(m_id);
        self.ui.get_background_draw_list().add_text(info_pos, debug_subtitle, debug_info);
        self.ui.get_foreground_draw_list().add_text(position_pos, ImColor32::WHITE, position_info);
        body_token.pop();
    }
}