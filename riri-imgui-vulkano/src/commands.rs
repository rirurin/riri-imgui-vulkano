use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use glam::{Mat4, Vec2, Vec4};
use imgui::{DrawCmd, DrawCmdParams, TextureId};
use imgui::internal::RawWrapper;
use riri_mod_tools_rt::logln;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, CopyImageToBufferInfo, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo};
use vulkano::format::{ClearValue, Format};
use vulkano::image::Image;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Framebuffer;
use crate::descriptors::{DescriptorSetRegistry, LibDescriptorSets};
use crate::error::Result;
use crate::geometry::{BasicDrawGeometry, ImguiGeometry};
use crate::resources::{HasCommandBufferAllocator, HasLogicalDevice, HasQueue, HasStandardMemoryAllocator};

#[derive(Debug)]
pub struct GpuCommandAllocator(Arc<StandardCommandBufferAllocator>);

impl Deref for GpuCommandAllocator {
    type Target = Arc<StandardCommandBufferAllocator>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GpuCommandAllocator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl HasCommandBufferAllocator for GpuCommandAllocator {
    fn allocator(&self) -> Arc<StandardCommandBufferAllocator> {
        (*self).clone()
    }
}

pub trait GpuCommandUsage {
    fn usage() -> CommandBufferUsage;
}

impl GpuCommandAllocator {
    pub fn new<C: HasLogicalDevice>(context: &C) -> Self {
        Self(Arc::new(StandardCommandBufferAllocator::new(
            context.logical_device(), StandardCommandBufferAllocatorCreateInfo::default())))
    }
}

#[derive(Debug)]
pub struct GpuCommandUsageOnce;
impl GpuCommandUsage for GpuCommandUsageOnce {
    fn usage() -> CommandBufferUsage {
        CommandBufferUsage::OneTimeSubmit
    }
}

#[derive(Debug)]
pub struct GpuCommandUsageMultiple;
impl GpuCommandUsage for GpuCommandUsageMultiple {
    fn usage() -> CommandBufferUsage {
        CommandBufferUsage::MultipleSubmit
    }
}

#[derive(Debug)]
pub struct GpuCommandUsageAsync;
impl GpuCommandUsage for GpuCommandUsageAsync {
    fn usage() -> CommandBufferUsage {
        CommandBufferUsage::SimultaneousUse
    }
}

type GpuBuilder = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>;

pub struct GpuCommandBuilder<'a, A: HasCommandBufferAllocator, U: GpuCommandUsage> {
    allocator: &'a A,
    builder: GpuBuilder,
    _usage: PhantomData<U>
}

impl<'a, A, U> Debug for GpuCommandBuilder<'a, A, U>
where A: HasCommandBufferAllocator,
      U: GpuCommandUsage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GpuCommandBuilder {{ }}")
    }
}

impl<'a, A, U> GpuCommandBuilder<'a, A, U>
where A: HasCommandBufferAllocator,
      U: GpuCommandUsage {
    pub fn new<C: HasQueue>(allocator: &'a A, context: &C) -> Result<Self> {
        let builder = AutoCommandBufferBuilder::primary(
            allocator.allocator(),
            context.queue().queue_family_index(),
            U::usage()
        )?;
        Ok(Self {
            allocator, builder, _usage: PhantomData::default()
        })
    }

    pub fn build(self) -> Result<Arc<PrimaryAutoCommandBuffer>> {
        Ok(self.builder.build()?)
    }
}

impl<'a, A, U> Deref for GpuCommandBuilder<'a, A, U>
where A: HasCommandBufferAllocator,
      U: GpuCommandUsage {
    type Target = GpuBuilder;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}

impl<'a, A, U> DerefMut for GpuCommandBuilder<'a, A, U>
where A: HasCommandBufferAllocator,
      U: GpuCommandUsage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.builder
    }
}

pub trait GpuCommandSet {
    fn build(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> Result<()>;
}

#[derive(Debug)]
pub struct CopyBufferToImage {
    image: Arc<Image>,
    upload_buffer: Subbuffer<[u8]>
}
impl CopyBufferToImage {
    pub fn new<
        A: HasStandardMemoryAllocator
    >(
        allocator: &A,
        image: Arc<Image>,
        image_data: &[u8]
    ) -> Result<Self> {
        let upload_buffer = Buffer::from_iter(
            allocator.allocator(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            (0..image_data.len() as u32).map(|_| 0u8)
        )?;
        let mut upload_data = upload_buffer.write()?;
        upload_data.copy_from_slice(image_data);
        drop(upload_data);
        Ok(Self {
            image,
            upload_buffer
        })
    }
}

impl GpuCommandSet for CopyBufferToImage {
    fn build(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> Result<()> {
        builder.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            self.upload_buffer.clone(), self.image.clone()))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct CopyImageToBuffer {
    image: Arc<Image>,
    buffer: Subbuffer<[u8]>
}

impl CopyImageToBuffer {
    pub fn new<
        A: HasStandardMemoryAllocator
    >(
        allocator: &A,
        image: Arc<Image>,
    ) -> Result<Self> {
        let extent = image.extent();
        let size = extent[0] * extent[1] * image.format().block_size() as u32;
        let buffer = Buffer::from_iter(
            allocator.allocator(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            (0..size).map(|_| 0u8)
        )?;
        Ok(Self { buffer, image })
    }
}

impl GpuCommandSet for CopyImageToBuffer {
    fn build(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> Result<()> {
        builder.copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
            self.image.clone(), self.buffer.clone()))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct StartRenderPass {
    framebuffer: Arc<Framebuffer>,
    clear_values: Vec<Option<ClearValue>>,
}

impl StartRenderPass {
    pub fn new(
        framebuffer: Arc<Framebuffer>,
        clear_values: Vec<Option<ClearValue>>,
    ) -> Self {
        Self { framebuffer, clear_values }
    }
}

impl GpuCommandSet for StartRenderPass {
    fn build(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> Result<()> {
        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: self.clear_values.clone(),
                    ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                }
            )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct NextSubpass;

impl NextSubpass {
    pub fn new() -> Self { Self }
}

impl GpuCommandSet for NextSubpass {
    fn build(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> Result<()> {
        builder.next_subpass(
            SubpassEndInfo::default(),
            SubpassBeginInfo {
                contents: SubpassContents::Inline,
                ..Default::default()
            }
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EndRenderPass;

impl EndRenderPass {
    pub fn new() -> Self { Self }
}

impl GpuCommandSet for EndRenderPass {
    fn build(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> Result<()> {
        builder.end_render_pass(SubpassEndInfo::default())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DrawImgui<'a> {
    pipeline: Arc<GraphicsPipeline>,
    geometry: &'a ImguiGeometry<'a>,
    viewport: Viewport,
    descriptors: &'a LibDescriptorSets,
    ortho: TextureId,
}

impl<'a> DrawImgui<'a> {
    pub fn new(
        pipeline: Arc<GraphicsPipeline>,
        geometry: &'a ImguiGeometry<'a>,
        viewport: Viewport,
        descriptors: &'a LibDescriptorSets,
        ortho: TextureId,
    ) -> Result<Self> {
        Ok(Self { pipeline, geometry, viewport, descriptors, ortho })
    }
}

impl<'a> GpuCommandSet for DrawImgui<'a> {
    fn build(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> Result<()> {
        // ImGui_ImplVulkan_SetupRenderState
        builder.bind_pipeline_graphics(self.pipeline.clone())?;
        if let Some(draw_data) = &self.geometry.draw_data {
            let viewport_extent = Vec2::from(self.viewport.extent);
            let ortho = self.descriptors.get(self.ortho)?.clone().upgrade().unwrap();
            builder
                .set_viewport(0, [self.viewport.clone()].into_iter().collect())?
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.pipeline.layout().clone(),
                    1, ortho
                )?
            ;
            if let Some(vertex_buffer) = &self.geometry.vertex_buffer {
                builder.bind_vertex_buffers(0, vertex_buffer.clone())?;
            }
            if let Some(index_buffer) = &self.geometry.index_buffer {
                builder.bind_index_buffer(index_buffer.clone())?;
            }
            let (mut global_index_offset, mut global_vertex_offset) = (0, 0);
            let mut current_texture_id : Option<TextureId> = None;
            for draw_list in &draw_data.draw_lists {
                for command in draw_list.commands() {
                    match command {
                        DrawCmd::Elements {
                            count,
                            cmd_params: DrawCmdParams {
                                clip_rect,
                                texture_id,
                                vtx_offset,
                                idx_offset
                            }
                        } => {
                            // Project scissor/clipping rectangles into framebuffer space
                            // Clamp to viewport as vkCmdSetScissor() won't accept values that are off bounds
                            let clip_rect = Vec4::from(clip_rect);
                            let clip_min = Vec2::new(
                                (clip_rect.x - draw_data.clip_off.x) * draw_data.clip_scale.x,
                                (clip_rect.y - draw_data.clip_off.y) * draw_data.clip_scale.y
                            ).max(Vec2::ZERO);
                            let clip_max = Vec2::new(
                                (clip_rect.z - draw_data.clip_off.x) * draw_data.clip_scale.x,
                                (clip_rect.w - draw_data.clip_off.y) * draw_data.clip_scale.y
                            ).min(viewport_extent);
                            if clip_min.x >= clip_max.x || clip_min.y >= clip_max.y {
                                continue;
                            }
                            // Apply scissor/clipping rectangle
                            builder
                                .set_scissor(0, [Scissor {
                                    offset: clip_min.as_uvec2().to_array(),
                                    extent: (clip_max - clip_min).as_uvec2().to_array()
                                }].into_iter().collect())?;
                            // Bind DescriptorSet with font or user texture
                            if current_texture_id.map_or(
                                true, |id| texture_id != id) {
                                if let Ok(desc) = self.descriptors.get(texture_id) {
                                    builder
                                        // layout(set=0, binding=0) uniform sampler2D sTexture;
                                        .bind_descriptor_sets(
                                            PipelineBindPoint::Graphics,
                                            self.pipeline.layout().clone(),
                                            0, desc.clone().upgrade().unwrap()
                                        )?;
                                    current_texture_id = Some(texture_id);
                                }
                            }
                            // Draw
                            unsafe { builder.draw_indexed(
                                count as u32, 1,
                                (idx_offset + global_index_offset) as u32,
                                (vtx_offset + global_vertex_offset) as i32,
                                0
                            )? };
                        },
                        DrawCmd::ResetRenderState => {
                            logln!(Warning, "DrawCmd::ResetRenderState not implemented");
                        },
                        DrawCmd::RawCallback {
                            callback,
                            raw_cmd
                        } => {
                            unsafe { callback(draw_list.raw(), raw_cmd ) }
                        }
                    }
                }
                global_index_offset += draw_list.idx_buffer().len();
                global_vertex_offset += draw_list.vtx_buffer().len();
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct DrawBasic3d<'a> {
    pipeline: Arc<GraphicsPipeline>,
    geometry: &'a BasicDrawGeometry,
    viewport: Viewport,
    descriptors: &'a LibDescriptorSets,
    mvp: TextureId
}

impl<'a> DrawBasic3d<'a> {
    pub fn new(
        pipeline: Arc<GraphicsPipeline>,
        geometry: &'a BasicDrawGeometry,
        viewport: Viewport,
        descriptors: &'a LibDescriptorSets,
        mvp: TextureId,
    ) -> Result<Self> {
        Ok(Self { pipeline, geometry, viewport, descriptors, mvp })
    }
}

impl<'a> GpuCommandSet for DrawBasic3d<'a> {
    fn build(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> Result<()> {
        let mvp = self.descriptors.get(self.mvp)?.clone().upgrade().unwrap();
        builder
            .bind_pipeline_graphics(self.pipeline.clone())?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                1, mvp
            )?;
        if let Some(vertex_buffer) = &self.geometry.vertex_buffer {
            builder.bind_vertex_buffers(0, vertex_buffer.clone())?;
        }
        if let Some(index_buffer) = &self.geometry.index_buffer {
            let index_count = index_buffer.len() as u32;
            builder.bind_index_buffer(index_buffer.clone())?;
            // Draw
            unsafe { builder.draw_indexed(
                 index_count, 1,
                0,
                0,
                0
            )? };
        }
        Ok(())
    }
}