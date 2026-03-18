use crate::commands::{CopyBufferToImage, GpuCommandAllocator, GpuCommandBuilder, GpuCommandSet, GpuCommandUsageOnce};
use crate::error::{LibError, Result};
use crate::geometry::GeometryBufferBuilder;
use crate::pipeline::PipelineLayoutBuilder;
use crate::resources::{HasLogicalDevice, HasQueue, HasStandardDescriptorSetAllocator, HasStandardMemoryAllocator};
use crate::shaders::{AppShader, ShaderRegistry};
use crate::try_get_vertex_pixel;
use glam::Mat4;
use imgui::TextureId;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use vulkano::buffer::{BufferContents, BufferUsage};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{CopyDescriptorSet, DescriptorSet, WriteDescriptorSet};
use vulkano::format::Format;
use vulkano::image::sampler::{Sampler, SamplerCreateInfo};
use vulkano::image::view::{ImageView, ImageViewCreateInfo, ImageViewType};
use vulkano::image::{Image, ImageCreateFlags, ImageCreateInfo, ImageSubresourceRange, ImageTiling, ImageType, ImageUsage, SampleCount};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::sync;
use vulkano::sync::GpuFuture;

pub trait DescriptorSetRegistry {
    /// Tries to get the descriptor set that matches the given key.
    fn get(&self, key: TextureId) -> Result<Weak<DescriptorSet>>;
    /// Removes the descriptor set from the registry.
    fn remove(&mut self, key: TextureId);
    /// Creates a descriptor set using a pipeline shader stage
    fn from_pipeline_layout<T0>(
        &mut self,
        context: &T0,
        shaders: (&AppShader, &AppShader),
        set: usize,
        descriptor_writes: impl IntoIterator<Item = WriteDescriptorSet>,
        descriptor_copies: impl IntoIterator<Item = CopyDescriptorSet>
    ) -> Result<TextureId>
    where T0: HasLogicalDevice;
}

#[derive(Debug)]
pub struct LibDescriptorSets {
    allocator: Arc<StandardDescriptorSetAllocator>,
    descriptor_sets: HashMap<TextureId, Arc<DescriptorSet>>
}

impl LibDescriptorSets {
    /// Create a new descriptor set collection. Contains the dedicated allocator and a descriptor
    /// set registry.
    pub fn new<D: HasLogicalDevice>(context: &D) -> Result<Self> {
        let allocator = Arc::new(
            StandardDescriptorSetAllocator::new(context.logical_device(), Default::default()));
        Ok(Self { allocator, descriptor_sets: HashMap::new() })
    }
}

impl DescriptorSetRegistry for LibDescriptorSets {
    fn get(&self, key: TextureId) -> Result<Weak<DescriptorSet>> {
        self.descriptor_sets.get(&key)
            .map(|v| Arc::downgrade(v))
            .ok_or(Box::new(LibError::MissingDescriptorSet(key)))
    }

    fn remove(&mut self, key: TextureId) {
        let _ = self.descriptor_sets.remove(&key);
    }

    fn from_pipeline_layout<T0>(
        &mut self,
        context: &T0,
        shaders: (&AppShader, &AppShader),
        set: usize,
        descriptor_writes: impl IntoIterator<Item = WriteDescriptorSet>,
        descriptor_copies: impl IntoIterator<Item = CopyDescriptorSet>
    ) -> Result<TextureId>
    where T0: HasLogicalDevice {
        let layout = PipelineLayoutBuilder::from_vertex_pixel(shaders).build(context)?;
        let descriptor_set_layout = layout.set_layouts()
            .get(set).ok_or(LibError::FailToGetDescriptorSetLayout(layout.clone()))?;
        let descriptor_set = DescriptorSet::new(
            self.allocator.clone(),
            descriptor_set_layout.clone(),
            descriptor_writes,
            descriptor_copies
        )?;
        let key = TextureId::new(&raw const *descriptor_set.as_ref() as usize);
        self.descriptor_sets.insert(key, descriptor_set.clone());
        Ok(key)
    }
}

impl HasStandardDescriptorSetAllocator for LibDescriptorSets {
    fn allocator(&self) -> Arc<StandardDescriptorSetAllocator> {
        self.allocator.clone()
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

    pub fn build<T0, T1>(
        context: &T0,
        shaders: &T1,
        descriptors: &mut LibDescriptorSets,
        command_allocator: &GpuCommandAllocator,
        fonts: &mut imgui::FontAtlas
    ) -> Result<()>
    where T0: HasLogicalDevice + HasStandardMemoryAllocator + HasQueue,
          T1: ShaderRegistry {
        // remove the old font texture
        if fonts.tex_id.id() as *const u8 != std::ptr::null() {
            descriptors.remove(fonts.tex_id);
        }
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
        let font_id = descriptors.from_pipeline_layout(
            context, try_get_vertex_pixel!(shaders, "imgui")?, 0,
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

#[derive(Debug)]
#[repr(transparent)]
pub struct ImguiOrthoUniform(TextureId);

impl ImguiOrthoUniform {
    pub fn new() -> Self { Self(std::ptr::null::<usize>().into()) }

    pub fn get(&self) -> TextureId {
        self.0
    }

    pub fn create_descriptor_set<T0, T1>(
        &mut self,
        context: &T0,
        shaders: &T1,
        descriptors: &mut LibDescriptorSets,
        projection: Mat4,
    ) -> Result<()>
    where T0: HasLogicalDevice + HasStandardMemoryAllocator,
          T1: ShaderRegistry {
        if self.0.id() as *const u8 != std::ptr::null() {
            descriptors.remove(self.0);
        }
        let buffer = GeometryBufferBuilder::from_iter_generic(
            projection.to_cols_array(), context, BufferUsage::UNIFORM_BUFFER)?.unwrap();
        self.0 = descriptors.from_pipeline_layout(
            context, try_get_vertex_pixel!(shaders, "imgui")?, 1,
            [WriteDescriptorSet::buffer(0, buffer.clone())],
            []
        )?;
        Ok(())
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Basic3dMVPUniform(TextureId);

impl Basic3dMVPUniform {
    pub fn new() -> Self { Self(std::ptr::null::<usize>().into()) }

    pub fn get(&self) -> TextureId {
        self.0
    }

    pub fn create_descriptor_set<T0, T1>(
        &mut self,
        context: &T0,
        shaders: &T1,
        descriptors: &mut LibDescriptorSets,
        view_projection: Mat4,
        model: Mat4
    ) -> Result<()>
    where T0: HasLogicalDevice + HasStandardMemoryAllocator,
          T1: ShaderRegistry {
        if self.0.id() as *const u8 != std::ptr::null() {
            descriptors.remove(self.0);
        }
        let camera_mvp = CameraMVP {
            view_projection: view_projection.to_cols_array(),
            model: model.to_cols_array()
        };
        let buffer = GeometryBufferBuilder::from_data(
            camera_mvp, context, BufferUsage::UNIFORM_BUFFER)?.unwrap();
        self.0 = descriptors.from_pipeline_layout(
            context, try_get_vertex_pixel!(shaders, "basic3d")?, 1,
            [WriteDescriptorSet::buffer(0, buffer.clone())],
            []
        )?;
        Ok(())
    }
}

#[repr(C)]
#[derive(Debug, BufferContents)]
pub struct CameraMVP {
    pub(crate) view_projection: [f32; 16],
    pub(crate) model: [f32; 16]
}