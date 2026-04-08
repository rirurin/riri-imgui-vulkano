use std::sync::Arc;
use std::time::{Duration, Instant};
use glam::{UVec2, Vec2};
use imgui::DrawData;
use riri_mod_tools_rt::logln;
use vulkano::format::ClearValue;
use vulkano::pipeline::graphics::viewport::Viewport;
use winit::window::Window;
use riri_imgui_vulkano::context::RendererContext;
use riri_imgui_vulkano::descriptors::{Basic3dMVPUniform, ImguiFontBuilder, ImguiOrthoUniform, LibDescriptorSets};
use riri_imgui_vulkano::geometry::ImguiGeometry;
use riri_imgui_vulkano::viewport::{ScissorBuilder, ViewportBuilder};
use riri_imgui_vulkano::pipeline::{ImguiGraphicsPipeline, CreateGraphicsPipeline, Basic3dGraphicsPipeline};
use riri_imgui_vulkano::render_pass::{LibRenderPass, RenderPassBuilder};
use riri_imgui_vulkano::resources::{HasPhysicalDevice, HasSwapchain};
use riri_imgui_vulkano::shaders::{LibShaderRegistry, ShaderRegistry};
use riri_imgui_vulkano::swapchain::SwapchainImpl;
use riri_imgui_vulkano::vertex::AppDrawData3D;
use crate::camera::Camera;
use crate::renderer::commands::AppGpuCommands;
use crate::renderer::pipeline::AppPipeline;
use crate::renderer::render_pass::AppRenderPass;
use crate::renderer::swapchain::AppSwapchain;
use crate::result::Result;

#[derive(Debug)]
pub struct VulkanContext {
    pub(crate) context: RendererContext,
    pub(crate) viewport: Viewport,
    pub(crate) swapchain: AppSwapchain,
    pub(crate) render_pass: LibRenderPass,
    pub(crate) descriptors: LibDescriptorSets,
    pub(crate) shaders: LibShaderRegistry,
    pub(crate) pipeline: AppPipeline,
    pub(crate) gpu_commands: AppGpuCommands,
    pub(crate) clear_color: ClearValue,
    pub(crate) ortho_builder: ImguiOrthoUniform,
    pub(crate) basic3d_mvp: Basic3dMVPUniform,
}

impl VulkanContext {
    pub fn new(
        context: RendererContext,
        window: Arc<Box<dyn Window>>,
        imgui: &mut imgui::Context
    ) -> Result<Self> {
        let start = Instant::now();
        // Vulkan objects
        let ref_window= window.as_ref().as_ref();
        let (viewport, scissor) = (
            ViewportBuilder::from_window(ref_window),
            ScissorBuilder::from_window(ref_window)
        );
        let mut descriptors = LibDescriptorSets::new(&context)?;
        let mut swapchain = AppSwapchain::new(&context, window.clone())?;
        let render_pass = AppRenderPass::new(&context, swapchain.swapchain()).build()?;
        swapchain.set_framebuffers(&render_pass)?;
        let mut shaders = LibShaderRegistry::default();
        Self::create_shader_modules(&context, &mut shaders)?;
        let pipeline = AppPipeline::new(
            Basic3dGraphicsPipeline::<0>::new(
                &context, &viewport, &scissor, &shaders, &render_pass)?,
            ImguiGraphicsPipeline::<1>::new(
                &context, &viewport, &scissor, &shaders, &render_pass)?,
        );
        // App objects
        let clear_color = ClearValue::Float([0.1, 0.1, 0.1, 1.]);
        let ortho_builder = ImguiOrthoUniform::new();
        let basic3d_mvp = Basic3dMVPUniform::new();
        let gpu_commands = AppGpuCommands::new(&context);
        ImguiFontBuilder::build(
            &context, &pipeline.imgui, &mut descriptors,
            gpu_commands.allocator(), imgui.fonts())?;
        // Performance metrics
        Self::debug_print(&context, Instant::now().duration_since(start));
        Ok(Self {
            context,
            viewport,
            swapchain,
            render_pass,
            descriptors,
            shaders,
            pipeline,
            gpu_commands,
            clear_color,
            ortho_builder,
            basic3d_mvp
        })
    }

    pub(crate) fn debug_print<T>(context: &T, time: Duration)
    where T: HasPhysicalDevice {
        let time_ms = time.as_micros() as f64 / 1000.;
        logln!(Information, "Vulkan renderer initialized: {} ms", time_ms);
        let physical_device = context.physical_device();
        let physical_properties = physical_device.properties();
        logln!(Information, "Selected device is:");
        logln!(Information, "\tName: {}", physical_properties.device_name);
        logln!(Information, "\tType: {:?}", physical_properties.device_type);
        logln!(Information, "\tDriver: {} (version 0x{:x})", physical_properties.driver_name
            .as_ref().map_or("No Name", |v| v.as_str()), physical_properties.driver_version);
        logln!(Information, "\tSupported Vulkan Version: {}", physical_properties.api_version);
        logln!(Information, "\tMaximum allocation: Size = 0x{:x}, Count = 0x{:x}",
            physical_properties.max_memory_allocation_size.unwrap_or(0),
            physical_properties.max_memory_allocation_count);
    }

    pub fn refresh(&mut self, window: Arc<Box<dyn Window>>) -> Result<()> {
        if window.surface_size().width == 0 || window.surface_size().height == 0 {
            return Ok(());
        }
        let dims = UVec2::from_array(window.surface_size().into());
        self.swapchain.refresh(&self.context, &self.render_pass, dims)?;
        let dims = dims.as_vec2().to_array();
        if dims != self.viewport.extent {
            self.viewport.extent = dims;
        }
        Ok(())
    }

    pub(crate) fn create_shader_modules(
        context: &RendererContext,
        shaders: &mut LibShaderRegistry
    ) -> Result<()> {
        let exec_path = std::env::current_exe()?.parent().map(|v| v.to_owned()).unwrap();
        shaders.add_vertex_shader(context, exec_path.join("shaders/imgui.vs"))?;
        shaders.add_pixel_shader(context, exec_path.join("shaders/imgui.ps"))?;
        shaders.add_vertex_shader(context, exec_path.join("shaders/basic3d.vs"))?;
        shaders.add_pixel_shader(context, exec_path.join("shaders/basic3d.ps"))?;
        Ok(())
    }

    pub(crate) fn render(
        &mut self,
        draw_data: &DrawData,
        draw3d: &AppDrawData3D,
        camera: &Camera,
        time_elapsed: f32,
    ) -> Result<()> {
        let acquired = match self.swapchain.acquire_swapchain_image() {
            Some(v) => v,
            None => {
                self.swapchain.recreate = true;
                return Ok(());
            }
        };
        let imgui_geometry = ImguiGeometry::new(&self.context, draw_data)?;
        let framebuffer_size = Vec2::new(
            draw_data.framebuffer_scale[0] * draw_data.display_size[0],
            draw_data.framebuffer_scale[1] * draw_data.display_size[1],
        );
        self.viewport = ViewportBuilder::from_extent(framebuffer_size);
        let command_buffer = self.gpu_commands.create_gpu_commands(
            &self.context, &self.viewport, self.swapchain.framebuffers[acquired.image_index].clone(), 
            &self.pipeline,imgui_geometry, draw3d, self.clear_color.clone(),
            &mut self.descriptors, &mut self.ortho_builder, camera,
            &mut self.basic3d_mvp, time_elapsed)?;
        self.swapchain.present(&self.context, command_buffer, acquired);
        Ok(())
    }
}