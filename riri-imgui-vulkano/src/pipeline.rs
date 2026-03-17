use crate::error::{LibError, Result};
use crate::resources::{HasGraphicsPipeline, HasLogicalDevice, HasPipelineLayout, HasRenderPass};
use crate::shaders::{AppShader, ShaderRegistry};
use crate::vertex::{AppDrawVert, AppVertex3D};
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::graphics::depth_stencil::{DepthState, DepthStencilState};
use vulkano::render_pass::Subpass;
use crate::try_get_vertex_pixel;

#[derive(Debug)]
#[repr(transparent)]
pub struct PipelineLayoutBuilder<'a, const N: usize>([&'a AppShader; N]);

impl<'a, const N: usize> PipelineLayoutBuilder<'a, N> {
    pub fn new(shaders: [&'a AppShader; N]) -> Self {
        Self(shaders)
    }
}

impl<'a> PipelineLayoutBuilder<'a, 2> {
    pub fn from_vertex_pixel(shaders: (&'a AppShader, &'a AppShader)) -> Self {
        shaders.into()
    }
}

impl<'a> From<(&'a AppShader, &'a AppShader)> for PipelineLayoutBuilder<'a, 2> {
    fn from(value: (&'a AppShader, &'a AppShader)) -> Self {
        Self([value.0, value.1])
    }
}

impl<'a, const N: usize> Deref for PipelineLayoutBuilder<'a, N> {
    type Target = [&'a AppShader; N];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, const N: usize> PipelineLayoutBuilder<'a, N> {
    pub fn get_stages(&self) -> [PipelineShaderStageCreateInfo; N] {
        std::array::from_fn::<_, N, _>(|i|
            PipelineShaderStageCreateInfo::new(self[i].entry_point()))
    }

    pub fn build<T: HasLogicalDevice>(self, context: &T) -> Result<Arc<PipelineLayout>> {
        let stages = self.get_stages();
        Ok(PipelineLayout::new(
            context.logical_device(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(context.logical_device())?)?)
    }
}

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

#[derive(Debug)]
#[repr(transparent)]
pub struct ImguiGraphicsPipeline<const I: usize>(Arc<GraphicsPipeline>);

impl<const I: usize> HasGraphicsPipeline for ImguiGraphicsPipeline<I> {
    fn graphics_pipeline(&self) -> Arc<GraphicsPipeline> {
        self.0.clone()
    }
}

impl<const I: usize> HasPipelineLayout for ImguiGraphicsPipeline<I> {
    fn layout(&self) -> Arc<PipelineLayout> {
        self.0.layout().clone()
    }
}

impl<const I: usize> CreateGraphicsPipeline for ImguiGraphicsPipeline<I> {
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
        let (vertex_shader, pixel_shader) = try_get_vertex_pixel!(shaders, "imgui")?;
        let layout = PipelineLayoutBuilder::from_vertex_pixel(
            (vertex_shader, pixel_shader));
        let stages = layout.get_stages();
        let vertex_input_state = AppDrawVert::per_vertex()
            .definition(&vertex_shader.entry_point())?;
        let layout = layout.build(context)?;
        let subpass = Subpass::from(render_pass.render_pass(), I as u32)
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

#[derive(Debug)]
#[repr(transparent)]
pub struct Basic3dGraphicsPipeline<const I: usize>(Arc<GraphicsPipeline>);

impl<const I: usize> HasGraphicsPipeline for Basic3dGraphicsPipeline<I> {
    fn graphics_pipeline(&self) -> Arc<GraphicsPipeline> {
        self.0.clone()
    }
}

impl<const I: usize> HasPipelineLayout for Basic3dGraphicsPipeline<I> {
    fn layout(&self) -> Arc<PipelineLayout> {
        self.0.layout().clone()
    }
}

impl<const I: usize> CreateGraphicsPipeline for Basic3dGraphicsPipeline<I> {
    fn new<
        T0: HasLogicalDevice,
        T1: ShaderRegistry,
        T2: HasRenderPass
    >(
        context: &T0,
        viewport: &Viewport,
        _: &Scissor,
        shaders: &T1,
        render_pass: &T2
    ) -> Result<Self> {
        let (vertex_shader, pixel_shader) = try_get_vertex_pixel!(shaders, "basic3d")?;
        let layout = PipelineLayoutBuilder::from_vertex_pixel(
            (vertex_shader, pixel_shader));
        let stages = layout.get_stages();
        let vertex_input_state = AppVertex3D::per_vertex()
            .definition(&vertex_shader.entry_point())?;
        let layout = layout.build(context)?;
        let render_pass = Subpass::from(render_pass.render_pass(), I as u32)
            .ok_or(LibError::FailToGetSubBuffer)?;
        let mut dynamic_state = HashSet::default();
        dynamic_state.insert(DynamicState::Viewport);
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
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    render_pass.num_color_attachments(),
                    ColorBlendAttachmentState {
                        blend: Some(AttachmentBlend::alpha()),
                        ..Default::default()
                    }
                )),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                subpass: Some(render_pass.into()),
                dynamic_state,
                ..GraphicsPipelineCreateInfo::layout(layout)
            }
        )?))
    }
}