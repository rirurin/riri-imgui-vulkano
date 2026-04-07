use crate::camera::Camera;
use crate::renderer::pipeline::AppPipeline;
use crate::renderer::swapchain::AppSwapchain;
use crate::result::Result;
use riri_imgui_vulkano::commands::{DrawBasic3d, DrawImgui, EndRenderPass, GpuCommandAllocator, GpuCommandBuilder, GpuCommandSet, GpuCommandUsageOnce, NextSubpass, StartRenderPass};
use riri_imgui_vulkano::descriptors::{Basic3dMVPUniform, ImguiOrthoUniform, LibDescriptorSets};
use riri_imgui_vulkano::geometry::{BasicDrawGeometry, ImguiGeometry};
use riri_imgui_vulkano::resources::{HasAutoCommandBuffers, HasGraphicsPipeline, HasLogicalDevice, HasQueue, HasStandardMemoryAllocator};
use riri_imgui_vulkano::vertex::AppDrawData3D;
use vulkano::render_pass::Framebuffer;
use std::sync::Arc;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::format::ClearValue;
use vulkano::pipeline::graphics::viewport::Viewport;

#[derive(Debug)]
pub struct AppGpuCommands;

impl AppGpuCommands {
    pub fn create_command_buffer<C>(
        context: &C,
        viewport: &Viewport,
        framebuffer: Arc<Framebuffer>,
        // swapchain: &AppSwapchain,
        pipelines: &AppPipeline,
        geom_imgui: ImguiGeometry,
        draw3d: &AppDrawData3D,
        clear_color: ClearValue,
        descriptors: &mut LibDescriptorSets,
        ortho_uniform: &mut ImguiOrthoUniform,
        camera: &Camera,
        basic3d_mvp: &mut Basic3dMVPUniform,
        time_elapsed: f32,
    ) -> Result<Arc<PrimaryAutoCommandBuffer>>
    where C: HasLogicalDevice + HasStandardMemoryAllocator + HasQueue {
        let allocator = GpuCommandAllocator::new(context);
        let (vp, model) = camera.calculate_mvp(viewport, time_elapsed);
        basic3d_mvp.create_descriptor_set(
            context, &pipelines.basic3d, descriptors, vp, model)?;
        ortho_uniform.create_descriptor_set(
            context, &pipelines.imgui, descriptors, geom_imgui.get_orthographic_projection())?;
        let geom_draw3d = BasicDrawGeometry::new(context, draw3d)?;
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
    }
}