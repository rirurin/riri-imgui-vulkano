use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Instant;
use glam::{UVec2, Vec2};
use imgui::DrawData;
use riri_mod_tools_rt::logln;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::format::ClearValue;
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport};
use winit::window::Window;
use riri_imgui_vulkano::commands::{DrawImgui, GpuCommandAllocator, GpuCommandBuilder, GpuCommandSet, GpuCommandUsageOnce};
use riri_imgui_vulkano::context::RendererContext;
use riri_imgui_vulkano::descriptors::LibDescriptorSets;
use riri_imgui_vulkano::viewport::{ScissorBuilder, ViewportBuilder};
// use riri_imgui_vulkano::commands::LibCommandBuffers;
use riri_imgui_vulkano::geometry::ImguiGeometry;
use riri_imgui_vulkano::pipeline::{ ImguiGraphicsPipeline, CreateGraphicsPipeline };
use riri_imgui_vulkano::render_pass::{ImguiRenderPass, LibRenderPass, RenderPassBuilder};
use riri_imgui_vulkano::resources::{HasAutoCommandBuffers, HasGraphicsPipeline, HasLogicalDevice, HasPhysicalDevice, HasQueue, HasRenderPass, HasStandardMemoryAllocator, HasSwapchain};
use riri_imgui_vulkano::shaders::{LibShaderRegistry, ShaderRegistry};
use riri_imgui_vulkano::swapchain::LibSwapchain;
use riri_imgui_vulkano::geometry::ImguiFontBuilder;
use crate::result::Result;

#[derive(Debug)]
pub struct AppGpuCommands {
    pub(crate) allocator: GpuCommandAllocator,
    pub(crate) buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
}

impl AppGpuCommands {
    pub fn new<C>(
        context: &C,
        viewport: &Viewport,
        swapchain: &LibSwapchain,
        pipeline: &ImguiGraphicsPipeline,
        geom_imgui: ImguiGeometry,
        clear_color: ClearValue,
        descriptors: &LibDescriptorSets
    ) -> Result<Self>
    where C: HasLogicalDevice + HasQueue {
        let allocator = GpuCommandAllocator::new(context);
        let buffers = swapchain.framebuffers.iter().map(|framebuffer| {
            let mut builder: GpuCommandBuilder<_, GpuCommandUsageOnce>
                = GpuCommandBuilder::new(&allocator, context)?;
            DrawImgui::new(
                clear_color,
                framebuffer.clone(),
                pipeline.graphics_pipeline(),
                &geom_imgui,
                viewport.clone(),
                descriptors
            )?.build(&mut builder)?;
            Ok(builder.build()?)
        }).collect::<Result<Vec<Arc<PrimaryAutoCommandBuffer>>>>()?;
        Ok(Self { allocator, buffers })
    }
}

impl HasAutoCommandBuffers for AppGpuCommands {
    fn buffer(&self, index: usize) -> Option<Arc<PrimaryAutoCommandBuffer>> {
        self.buffers.get(index).map(|v| v.clone())
    }
}

#[derive(Debug)]
pub struct VulkanContext {
    pub(crate) context: RendererContext,
    pub(crate) viewport: Viewport,
    pub(crate) swapchain: LibSwapchain,
    pub(crate) render_pass: LibRenderPass,
    pub(crate) descriptors: LibDescriptorSets,
    pub(crate) shaders: LibShaderRegistry,
    pub(crate) pipeline: ImguiGraphicsPipeline,
    pub(crate) gpu_commands: AppGpuCommands,
    pub(crate) clear_color: ClearValue,
}

impl VulkanContext {
    pub fn new(
        context: RendererContext,
        window: Arc<Box<dyn Window>>,
        imgui: &mut imgui::Context
    ) -> Result<Self> {
        let start = Instant::now();
        let ref_window= window.as_ref().as_ref();
        let (viewport, scissor) = (
            ViewportBuilder::from_window(ref_window),
            ScissorBuilder::from_window(ref_window)
        );

        let mut descriptors = LibDescriptorSets::new(&context)?;
        let mut swapchain = LibSwapchain::new(&context, window.clone())?;
        let render_pass = ImguiRenderPass::new(&context, swapchain.swapchain()).build()?;
        swapchain.set_framebuffers(&render_pass)?;

        // ImGui_ImplVulkan_CreateShaderModules
        let mut shaders = LibShaderRegistry::default();
        Self::create_shader_modules(&context, &mut shaders)?;

        let pipeline = ImguiGraphicsPipeline::new(
            &context, &viewport, &scissor, &shaders, &render_pass)?;
        let clear_color = ClearValue::Float([0.1, 0.1, 0.1, 1.]);

        let gpu_commands = AppGpuCommands::new(
            &context, &viewport, &swapchain, &pipeline, // None,
            ImguiGeometry::default(), clear_color.clone(), &descriptors)?;
        ImguiFontBuilder::build(
            &context, &shaders, &mut descriptors,
            &gpu_commands.allocator, imgui.fonts())?;

        // Completed
        let time_ms = Instant::now().duration_since(start).as_micros() as f64 / 1000.;
        logln!(Information, "Vulkan renderer initialized: {} ms", time_ms);
        let physical_device = context.physical_device();
        let physical_properties = physical_device.properties();
        logln!(Information, "Selected device is:");
        logln!(Information, "\tName: {}", physical_properties.device_name);
        logln!(Information, "\tDriver: {} (version 0x{:x})", physical_properties.driver_name
            .as_ref().map_or("No Name", |v| v.as_str()), physical_properties.driver_version);
        logln!(Information, "\tSupported Vulkan Version: {}", physical_properties.api_version);
        logln!(Information, "\tMaximum allocation: Size = 0x{:x}, Count = 0x{:x}",
            physical_properties.max_memory_allocation_size.unwrap_or(0),
            physical_properties.max_memory_allocation_count);
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
        })
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

    pub fn present(&mut self) -> Result<bool> {
        self.swapchain.present(&self.context, &self.gpu_commands)
    }

    pub(crate) fn create_shader_modules(
        context: &RendererContext,
        shaders: &mut LibShaderRegistry
    ) -> Result<()> {
        let exec_path = std::env::current_exe()?.parent().map(|v| v.to_owned()).unwrap();
        shaders.add_vertex_shader(context, exec_path.join("shaders/imgui.vs"))?;
        shaders.add_pixel_shader(context, exec_path.join("shaders/imgui.ps"))?;
        Ok(())
    }

    pub(crate) fn render_imgui(
        &mut self,
        draw_data: &DrawData,
    ) -> Result<()> {
        let imgui_geometry = ImguiGeometry::new(&self.context, draw_data)?;
        let framebuffer_size = Vec2::new(
            draw_data.framebuffer_scale[0] * draw_data.display_size[0],
            draw_data.framebuffer_scale[1] * draw_data.display_size[1],
        );
        self.viewport = ViewportBuilder::from_extent(framebuffer_size);
        self.gpu_commands = AppGpuCommands::new(
            &self.context, &self.viewport, &self.swapchain, &self.pipeline, // None,
             imgui_geometry, self.clear_color.clone(), &self.descriptors)?;
        Ok(())
    }
}