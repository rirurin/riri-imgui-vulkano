use std::collections::HashMap;
use std::sync::{ Arc, Weak };
use imgui::TextureId;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{CopyDescriptorSet, DescriptorSet, WriteDescriptorSet};
use vulkano::pipeline::{PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use crate::resources::HasLogicalDevice;
use crate::error::{LibError, Result};
use crate::shaders::LibShaderRegistry;

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

    /// Tries to get the descriptor set that matches the given key.
    pub fn get(&self, key: TextureId) -> Result<Weak<DescriptorSet>> {
        self.descriptor_sets.get(&key)
            .map(|v| Arc::downgrade(v))
            .ok_or(Box::new(LibError::MissingDescriptorSet(key)))
    }

    /// Removes the descriptor set from the registry.
    pub fn remove(&mut self, key: TextureId) {
        let _ = self.descriptor_sets.remove(&key);
    }

    /// Creates a descriptor set using a pipeline shader stage
    pub fn from_pipeline_shader_stage<D>(
        &mut self,
        context: &D,
        shaders: &LibShaderRegistry,
        set: usize,
        descriptor_writes: impl IntoIterator<Item = WriteDescriptorSet>,
        descriptor_copies: impl IntoIterator<Item = CopyDescriptorSet>
    // ) -> Result<Arc<DescriptorSet>>
    ) -> Result<TextureId>
    where D: HasLogicalDevice {
        let vertex_shader = shaders.get("imgui.vs").unwrap();
        let pixel_shader = shaders.get("imgui.ps").unwrap();
        // imgui_impl_vulkan.cpp 809:818
        let stages = [
            PipelineShaderStageCreateInfo::new(vertex_shader.entry_point()),
            PipelineShaderStageCreateInfo::new(pixel_shader.entry_point()),
        ];
        // ImGui_ImplVulkan_CreatePipelineLayout
        let layout = PipelineLayout::new(
            context.logical_device(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(context.logical_device())?)?;
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
        // Ok(descriptor_set)
    }
}