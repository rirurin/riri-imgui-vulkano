use std::sync::Arc;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::device::{Device, Queue};
use vulkano::device::physical::PhysicalDevice;
use vulkano::instance::Instance;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout};
use vulkano::render_pass::RenderPass;
use vulkano::swapchain::{Surface, Swapchain};

pub trait HasInstance {
    fn instance(&self) -> Arc<Instance>;
}

pub trait HasSurface {
    fn surface(&self) -> Arc<Surface>;
}

pub trait HasPhysicalDevice {
    fn physical_device(&self) -> Arc<PhysicalDevice>;
}

pub trait HasLogicalDevice {
    fn logical_device(&self) -> Arc<Device>;
}

pub trait HasQueue {
    fn queue(&self) -> Arc<Queue>;
}

pub trait HasStandardMemoryAllocator {
    fn allocator(&self) -> Arc<StandardMemoryAllocator>;
}

pub trait HasSwapchain {
    fn swapchain(&self) -> Arc<Swapchain>;
}

pub trait HasRenderPass {
    fn render_pass(&self) -> Arc<RenderPass>;
}

pub trait HasGraphicsPipeline {
    fn graphics_pipeline(&self) -> Arc<GraphicsPipeline>;
}

pub trait HasPipelineLayout {
    fn layout(&self) -> Arc<PipelineLayout>;
}

pub trait HasCommandBufferAllocator {
    fn allocator(&self) -> Arc<StandardCommandBufferAllocator>;
}

pub trait HasAutoCommandBuffers {
    fn buffer(&self, index: usize) -> Option<Arc<PrimaryAutoCommandBuffer>>;
}