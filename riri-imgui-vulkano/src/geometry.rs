use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use glam::{Mat4, Vec2, Vec3, Vec4};
use imgui::{DrawCmd, DrawCmdParams, DrawData, DrawIdx, DrawList, DrawListIterator, DrawVert, TextureId};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo};
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::{sync, DeviceSize};
use vulkano::format::{ClearValue, Format};
use vulkano::image::{Image, ImageCreateFlags, ImageCreateInfo, ImageSubresourceRange, ImageTiling, ImageType, ImageUsage, SampleCount};
use vulkano::image::sampler::{BorderColor, Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode};
use vulkano::image::view::{ImageView, ImageViewCreateInfo, ImageViewType};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::depth_stencil::CompareOp;
use vulkano::sync::GpuFuture;
use crate::commands::{CopyBufferToImage, GpuCommandAllocator, GpuCommandBuilder, GpuCommandSet, GpuCommandUsageOnce};
use crate::descriptors::LibDescriptorSets;
use crate::error::Result;
use crate::resources::{HasLogicalDevice, HasQueue, HasStandardMemoryAllocator};
use crate::shaders::LibShaderRegistry;
use crate::vertex::{AppDrawData3D, AppDrawVert};

pub struct ImguiGeometryDraw<'a> {
    pub(crate) display_size: Vec2,
    pub(crate) display_pos: Vec2,
    pub(crate) clip_off: Vec2,
    pub(crate) clip_scale: Vec2,
    pub(crate) draw_lists: Vec<&'a DrawList>

}

impl<'a> Debug for ImguiGeometryDraw<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImguiGeometryDraw {{ display_size: {:?}, display_pos: {:?}, framebuffer_scale: {:?} }} ",
               self.display_size, self.display_pos, self.clip_scale)
    }
}

impl<'a> ImguiGeometryDraw<'a> {
    pub fn new(draw_data: &'a DrawData) -> Self {
        // Setup scale and translation:
        // Our visible imgui space lies from draw_data->DisplayPps (top left) to
        // draw_data->DisplayPos+data_data->DisplaySize (bottom right).
        // DisplayPos is (0,0) for single viewport apps.
        let display_size = Vec2::from(draw_data.display_size);
        let display_pos = Vec2::from(draw_data.display_pos);
        let clip_off = display_pos;
        let clip_scale = Vec2::from(draw_data.framebuffer_scale);
        Self {
            display_size,
            display_pos,
            clip_off,
            clip_scale,
            draw_lists: draw_data.draw_lists().collect()
        }
    }
}

#[derive(Debug)]
pub struct ImguiGeometry<'a> {
    pub(crate) vertex_buffer: Option<Subbuffer<[AppDrawVert]>>,
    pub(crate) index_buffer: Option<Subbuffer<[DrawIdx]>>,
    pub(crate) draw_data: Option<ImguiGeometryDraw<'a>>
}

impl<'a> ImguiGeometry<'a> {
    pub fn new<D>(
        dev: &D,
        draw_data: &'a DrawData
    ) -> Result<Self>
    where D: HasStandardMemoryAllocator {
        let mut vertices = Vec::with_capacity(draw_data.total_vtx_count as _);
        let mut indices = Vec::with_capacity(draw_data.total_idx_count as _);
        draw_data.draw_lists().for_each(|f| {
            let vtx_buffer = f.vtx_buffer();
            let app_vtx = unsafe { std::slice::from_raw_parts(
                vtx_buffer.as_ptr() as *const AppDrawVert, vtx_buffer.len()) };
            vertices.extend(app_vtx);
            indices.extend(f.idx_buffer());
        });

        let vertex_buffer = match vertices.len() > 0 {
            true => Some(Buffer::from_iter(
                dev.allocator(),
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                vertices
            )?),
            false => None
        };

        let index_buffer = match indices.len() > 0 {
            true => Some(Buffer::from_iter(
                dev.allocator(),
                BufferCreateInfo {
                    usage: BufferUsage::INDEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                indices
            )?),
            false => None
        };

        let draw_data = Some(ImguiGeometryDraw::new(draw_data));
        Ok(Self { vertex_buffer, index_buffer, draw_data })
    }
}

impl<'a> Default for ImguiGeometry<'a> {
    fn default() -> Self {
        Self { vertex_buffer: None, index_buffer: None, draw_data: None }
    }
}

#[derive(Debug)]
pub struct ImguiFontBuilder;

impl ImguiFontBuilder {

    fn build_font_image<
        T: HasStandardMemoryAllocator
    >(context: &T, extent: [u32; 2]) -> Result<Arc<Image>> {
        Ok(Image::new(
            context.allocator(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [extent[0], extent[1], 1],
                usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
                mip_levels: 1,
                array_layers: 1,
                samples: SampleCount::Sample1,
                tiling: ImageTiling::Optimal,
                flags: ImageCreateFlags::empty(),
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            }
        )?)
    }

    fn build_font_image_view(image: Arc<Image>) -> Result<Arc<ImageView>> {
        Ok(ImageView::new(
            image.clone(),
            ImageViewCreateInfo {
                view_type: ImageViewType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                subresource_range: ImageSubresourceRange::from_parameters(
                    Format::R8G8B8A8_UNORM,
                    image.mip_levels(),
                    image.array_layers()
                ),
                ..Default::default()
            }
        )?)
    }

    pub fn build<D>(
        context: &D,
        shaders: &LibShaderRegistry,
        descriptors: &mut LibDescriptorSets,
        command_allocator: &GpuCommandAllocator,
        fonts: &mut imgui::FontAtlas
    ) -> Result<()>
    where D: HasLogicalDevice + HasStandardMemoryAllocator + HasQueue {
        // ImGui_ImplVulkan_CreateFontSampler
        let font_sampler = Sampler::new(
            context.logical_device(),
            SamplerCreateInfo::simple_repeat_linear()
        )?;

        // ImGui_ImplVulkan_CreateFontsTexture
        let fa_tex = fonts.build_rgba32_texture();
        let font_data = unsafe { std::slice::from_raw_parts(fa_tex.data.as_ptr(), fa_tex.data.len()) };
        let font_image = Self::build_font_image(context, [fa_tex.width, fa_tex.height])?;
        let font_image_view = Self::build_font_image_view(font_image.clone())?;
        // layout(set=0, binding=0) uniform sampler2D sTexture;
        let font_id = descriptors.from_pipeline_shader_stage(
            context, shaders, 0,
            [WriteDescriptorSet::image_view_sampler(
                0, font_image_view.clone(), font_sampler.clone())],
            []
        )?;
        let mut builder: GpuCommandBuilder<_, GpuCommandUsageOnce>
            = GpuCommandBuilder::new(command_allocator, context)?;
        CopyBufferToImage::new(context, font_image.clone(), font_data)?.build(&mut builder)?;
        let command_buffer = builder.build()?;
        sync::now(context.logical_device())
            .then_execute(context.queue(), command_buffer)?
            .then_signal_fence_and_flush()?
            .wait(None)?;
        // Store our identifier
        fonts.tex_id = font_id;
        Ok(())
    }
}