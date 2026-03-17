use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use glam::{UVec2, Vec2};
use vulkano::command_buffer::CommandBufferExecFuture;
use vulkano::format::Format;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::image::view::ImageView;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo};
use vulkano::swapchain::{PresentFuture, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::future::{FenceSignalFuture, JoinFuture};
use vulkano::sync::GpuFuture;
use vulkano::{sync, Validated, VulkanError};
use winit::window::Window;
use crate::error::{ LibError, Result };
use crate::resources::{HasAutoCommandBuffers, HasLogicalDevice, HasPhysicalDevice, HasQueue, HasRenderPass, HasStandardMemoryAllocator, HasSurface, HasSwapchain};

pub type FenceFuture = PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>;

pub struct BaseSwapchain {
    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<Image>>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub fences: Vec<Option<Arc<FenceSignalFuture<FenceFuture>>>>,
    pub previous_fence: usize,
}

impl HasSwapchain for BaseSwapchain {
    fn swapchain(&self) -> Arc<Swapchain> {
        self.swapchain.clone()
    }
}

impl Debug for BaseSwapchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VulkanSwapchainData {{ swapchain: {:?}, images: {:?}, framebuffers: {:?} }}"
               , self.swapchain, self.images, self.framebuffers)
    }
}

impl BaseSwapchain {
    pub fn create_swapchain_and_images<T>(
        context: &T,
        window: Arc<Box<dyn Window>>
    ) -> Result<(Arc<Swapchain>, Vec<Arc<Image>>)>
    where T: HasPhysicalDevice + HasSurface + HasLogicalDevice + HasStandardMemoryAllocator {
        let image_format = context.physical_device().surface_formats(
            context.surface().as_ref(), Default::default())?[0].0;
        let capabilities = context.physical_device().surface_capabilities(
            context.surface().as_ref(), Default::default())?;
        let dimensions = window.surface_size();
        let composite_alpha = capabilities.supported_composite_alpha
            .into_iter().next().ok_or(LibError::NoSurfaceCompositeAlpha)?;
        Ok(Swapchain::new(
            context.logical_device(),
            context.surface(),
            SwapchainCreateInfo {
                min_image_count: capabilities.min_image_count,
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                ..Default::default()
            }
        )?)
    }

    fn present_inner<
        T0: HasLogicalDevice + HasQueue,
        T1: HasAutoCommandBuffers
    >(&mut self, device: &T0, buffers: &T1) -> Result<bool> {
        let (image_index, mut recreate_swapchain, future) =
            match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(Validated::Error(VulkanError::OutOfDate)) => return Ok(true),
                Err(e) => panic!("Couldn't acquire image: {}", e)
            };
        let image_index = image_index as usize;
        if let Some(image_fence) = &self.fences[image_index] {
            image_fence.wait(None)?; // wait for GPU to finish
        }
        let prev_fut = match self.fences[self.previous_fence].clone() {
            None => { // Create a NowFuture
                let mut now = sync::now(device.logical_device());
                now.cleanup_finished();
                now.boxed()
            }, // Use the existing FenceSignalFuture
            Some(fence) => fence.boxed()
        };
        let execution = prev_fut
            .join(future)
            .then_execute(device.queue(), buffers.buffer(image_index).ok_or(LibError::NoCommandBufferAtIndex(image_index))?)?
            .then_swapchain_present(device.queue(), SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index as _))
            .then_signal_fence_and_flush();
        self.fences[image_index] = match execution.map_err(Validated::unwrap) {
            Ok(fut) => Some(Arc::new(fut)),
            Err(VulkanError::OutOfDate) => {
                recreate_swapchain = true;
                None
            },
            Err(e) => {
                println!("Failed to flush future: {}", e);
                None
            }
        };
        self.previous_fence = image_index;
        Ok(recreate_swapchain)
    }

    pub fn new<T>(context: &T, window: Arc<Box<dyn Window>>) -> Result<Self>
    where T: HasPhysicalDevice + HasSurface + HasLogicalDevice + HasStandardMemoryAllocator {
        let (swapchain, images)
            = BaseSwapchain::create_swapchain_and_images(context, window.clone())?;
        let framebuffers = vec![];
        let fences = vec![None; images.len()];
        let previous_fence = 0;
        Ok(Self {
            swapchain, images, framebuffers, fences, previous_fence
        })
    }
}

pub trait SwapchainImpl: Deref<Target = BaseSwapchain> + DerefMut<Target = BaseSwapchain> {
    fn make_framebuffer<R: HasRenderPass>(&self, image: Arc<Image>, render_pass: &R) -> Result<Arc<Framebuffer>>;
    fn set_framebuffers<R: HasRenderPass>(&mut self, render_pass: &R) -> Result<()> {
        self.framebuffers = self.images.iter()
            .map(|image| self.make_framebuffer(image.clone(), render_pass))
            .collect::<Result<Vec<Arc<Framebuffer>>>>()?;
        Ok(())
    }
    fn present<
        T0: HasLogicalDevice + HasQueue,
        T1: HasAutoCommandBuffers
    >(&mut self, device: &T0, buffers: &T1) -> Result<bool> {
        self.present_inner(device, buffers)
    }
    fn refresh<
        T0: HasStandardMemoryAllocator,
        T1: HasRenderPass
    >(&mut self, context: &T0, render_pass: &T1, extent: UVec2) -> Result<()>;
}

#[derive(Debug)]
pub struct LibSwapchain {
    pub base: BaseSwapchain
}

impl HasSwapchain for LibSwapchain {
    fn swapchain(&self) -> Arc<Swapchain> {
        self.base.swapchain.clone()
    }
}

impl Deref for LibSwapchain {
    type Target = BaseSwapchain;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for LibSwapchain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl SwapchainImpl for LibSwapchain {
    fn make_framebuffer<R: HasRenderPass>(&self, image: Arc<Image>, render_pass: &R) -> Result<Arc<Framebuffer>> {
        let color = ImageView::new_default(image.clone())?;
        Ok(Framebuffer::new(
            render_pass.render_pass(),
            FramebufferCreateInfo {
                attachments: vec![color],
                ..Default::default()
            },
        )?)
    }

    fn refresh<
        T0: HasStandardMemoryAllocator,
        T1: HasRenderPass
    >(&mut self, _: &T0, render_pass: &T1, extent: UVec2) -> Result<()> {
        (self.swapchain, self.images) = self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: extent.to_array(), ..self.swapchain.create_info()
        })?;
        self.set_framebuffers(render_pass)?;
        Ok(())
    }
}

impl LibSwapchain {
    pub fn new<T>(context: &T, window: Arc<Box<dyn Window>>) -> Result<Self>
    where T: HasPhysicalDevice + HasSurface + HasLogicalDevice + HasStandardMemoryAllocator {
        Ok(Self {
            base: BaseSwapchain::new(context, window)?
        })
    }
}