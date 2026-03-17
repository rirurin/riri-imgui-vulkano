use crate::renderer::pipeline::AppPipeline;
use crate::result::Result;
use riri_imgui_vulkano::commands::{DrawBasic3d, DrawImgui, EndRenderPass, GpuCommandAllocator, GpuCommandBuilder, GpuCommandSet, GpuCommandUsageOnce, NextSubpass, StartRenderPass};
use riri_imgui_vulkano::descriptors::{Basic3dMVPUniform, ImguiOrthoUniform, LibDescriptorSets};
use riri_imgui_vulkano::geometry::{BasicDrawGeometry, ImguiGeometry};
use riri_imgui_vulkano::pipeline::CreateGraphicsPipeline;
use riri_imgui_vulkano::render_pass::RenderPassBuilder;
use riri_imgui_vulkano::resources::{HasAutoCommandBuffers, HasGraphicsPipeline, HasLogicalDevice, HasQueue, HasRenderPass, HasStandardMemoryAllocator, HasSwapchain};
use riri_imgui_vulkano::shaders::{LibShaderRegistry, ShaderRegistry};
use riri_imgui_vulkano::swapchain::LibSwapchain;
use std::sync::Arc;
use glam::Mat4;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::format::ClearValue;
use vulkano::pipeline::graphics::viewport::Viewport;
use riri_imgui_vulkano::vertex::AppDrawData3D;
use crate::camera::Camera;
use crate::renderer::swapchain::AppSwapchain;

#[derive(Debug)]
pub struct AppGpuCommands {
    pub(crate) allocator: GpuCommandAllocator,
    pub(crate) buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
}

impl AppGpuCommands {
    pub fn new<C>(
        context: &C,
        viewport: &Viewport,
        swapchain: &AppSwapchain,
        pipelines: &AppPipeline,
        geom_imgui: ImguiGeometry,
        draw3d: &AppDrawData3D,
        clear_color: ClearValue,
        shaders: &LibShaderRegistry,
        descriptors: &mut LibDescriptorSets,
        ortho_uniform: &mut ImguiOrthoUniform,
        camera: &Camera,
        basic3d_mvp: &mut Basic3dMVPUniform,
    ) -> Result<Self>
    where C: HasLogicalDevice + HasStandardMemoryAllocator + HasQueue {
        let allocator = GpuCommandAllocator::new(context);
        let (vp, model) = camera.calculate_mvp(viewport);
        basic3d_mvp.create_descriptor_set(context, shaders, descriptors, vp, model)?;
       ortho_uniform.create_descriptor_set(
           context, shaders, descriptors, geom_imgui.get_orthographic_projection())?;
        let geom_draw3d = BasicDrawGeometry::new(context, draw3d)?;
        let buffers = swapchain.framebuffers.iter().map(|framebuffer| {
            let mut builder: GpuCommandBuilder<_, GpuCommandUsageOnce>
                = GpuCommandBuilder::new(&allocator, context)?;
            let clear_values = vec![Some(clear_color), Some(ClearValue::Depth(1.))];
            StartRenderPass::new(framebuffer.clone(), clear_values).build(&mut builder)?;
            DrawBasic3d::new(
                pipelines.basic3d.graphics_pipeline(),
                &geom_draw3d,
                viewport.clone(),
                descriptors,
                basic3d_mvp.get()
            )?.build(&mut builder)?;
            NextSubpass::new().build(&mut builder)?;
            DrawImgui::new(
                pipelines.imgui.graphics_pipeline(),
                &geom_imgui,
                viewport.clone(),
                descriptors,
                ortho_uniform.get()
            )?.build(&mut builder)?;
            EndRenderPass::new().build(&mut builder)?;
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