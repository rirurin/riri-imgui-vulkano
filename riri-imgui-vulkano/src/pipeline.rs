use std::collections::HashSet;
use std::sync::Arc;
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport, ViewportState};
use vulkano::pipeline::{DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::render_pass::Subpass;
use crate::error::{LibError, Result};
use crate::vertex::AppDrawVert;
use crate::resources::{HasGraphicsPipeline, HasLogicalDevice, HasPipelineLayout, HasRenderPass};
use crate::shaders::ShaderRegistry;

#[derive(Debug)]
#[repr(transparent)]
pub struct ImguiGraphicsPipeline(Arc<GraphicsPipeline>);

pub trait CreateGraphicsPipeline where Self: Sized {
    fn new<
        T0: HasLogicalDevice,
        T1: ShaderRegistry,
        T2: HasRenderPass
    >(
        device: &T0,
        viewport: &Viewport,
        scissor: &Scissor,
        shaders: &T1,
        render_pass: &T2
    ) -> Result<Self>;
}

impl HasGraphicsPipeline for ImguiGraphicsPipeline {
    fn graphics_pipeline(&self) -> Arc<GraphicsPipeline> {
        self.0.clone()
    }
}

impl HasPipelineLayout for ImguiGraphicsPipeline {
    fn layout(&self) -> Arc<PipelineLayout> {
        self.0.layout().clone()
    }
}

impl CreateGraphicsPipeline for ImguiGraphicsPipeline {
    fn new<
        T0: HasLogicalDevice,
        T1: ShaderRegistry,
        T2: HasRenderPass
    >(
        context: &T0,
        viewport: &Viewport,
        scissor: &Scissor,
        shaders: &T1,
        render_pass: &T2
    ) -> Result<Self> {
        let vertex_shader = shaders.get("imgui.vs").unwrap();
        let pixel_shader = shaders.get("imgui.ps").unwrap();
        // imgui_impl_vulkan.cpp 809:818
        let stages = [
            PipelineShaderStageCreateInfo::new(vertex_shader.entry_point()),
            PipelineShaderStageCreateInfo::new(pixel_shader.entry_point()),
        ];
        // imgui_impl_vulkan.cpp 819:842
        let vertex_input_state = AppDrawVert::per_vertex()
            .definition(&vertex_shader.entry_point())?;
        // ImGui_ImplVulkan_CreatePipelineLayout
        let layout = PipelineLayout::new(
            context.logical_device(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(context.logical_device())?)?;
        let subpass = Subpass::from(render_pass.render_pass(), 0)
            .ok_or(LibError::FailToGetSubBuffer)?;
        let mut dynamic_state = HashSet::default();
        dynamic_state.insert(DynamicState::Viewport);
        dynamic_state.insert(DynamicState::Scissor);
        Ok(Self(GraphicsPipeline::new(
            context.logical_device(),
            None,
            GraphicsPipelineCreateInfo {
                // The stages of the pipeline
                stages: stages.into_iter().collect(),
                // Describe the layout of the vertex input and how it should behave
                vertex_input_state: Some(vertex_input_state),
                // Indicate the type of the primitives, a list of triangles by default
                input_assembly_state: Some(InputAssemblyState::default()),
                // Set the viewport
                viewport_state: Some(ViewportState {
                    viewports: [viewport.clone()].into_iter().collect(),
                    scissors: [scissor.clone()].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState {
                        blend: Some(AttachmentBlend::alpha()),
                        ..Default::default()
                    }
                )),
                subpass: Some(subpass.into()),
                dynamic_state,
                ..GraphicsPipelineCreateInfo::layout(layout)
            }
        )?))
    }
}