use crate::error::{LibError, Result};
use crate::resources::{HasAutoCommandBuffers, HasLogicalDevice, HasPhysicalDevice, HasQueue, HasRenderPass, HasStandardMemoryAllocator, HasSurface, HasSwapchain};
use glam::UVec2;
use vulkano::format::Format;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use riri_mod_tools_rt::logln;
use vulkano::command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageUsage};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo};
use vulkano::swapchain::{ColorSpace, PresentFuture, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::future::{FenceSignalFuture, JoinFuture};
use vulkano::sync::GpuFuture;
use vulkano::{sync, Validated, VulkanError};
use winit::window::Window;

pub type FenceFuture = PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>;

pub struct AcquireSwapchainImageResult {
    pub image_index: usize,
    pub suboptimal: bool,
    pub future: SwapchainAcquireFuture
}

impl AcquireSwapchainImageResult {
    pub(crate) fn new(image_index: usize, suboptimal: bool, acquire_future: SwapchainAcquireFuture) -> Self {
        Self { image_index, suboptimal, future: acquire_future }
    }
}

pub struct BaseSwapchain {
    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<Image>>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
    pub recreate: bool
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
        let image_format = context
            .physical_device()
            .surface_formats(context.surface().as_ref(), Default::default())?
            .iter().filter_map(|(f, s)| {
            match *s {
                ColorSpace::SrgbNonLinear => Some(*f),
                _ => None
            }
            })
            .min_by_key(|f| match *f {
                Format::B8G8R8A8_UNORM => 0,
                Format::R8G8B8A8_UNORM => 1,
                Format::B8G8R8A8_SRGB => 2,
                Format::R8G8B8A8_SRGB => 3,
                _ => 4
            }).ok_or(LibError::NoSuitableSwapchainImageFormat)?;
        logln!(Debug, "Selected swapchain image format: {:?}", image_format);
        let capabilities = context.physical_device().surface_capabilities(
            context.surface().as_ref(), Default::default())?;
        let dimensions = window.surface_size();
        let composite_alpha = capabilities.supported_composite_alpha
            .into_iter().next().ok_or(LibError::NoSurfaceCompositeAlpha)?;
        Ok(Swapchain::new(
            context.logical_device(),
            context.surface(),
            SwapchainCreateInfo {
                min_image_count: capabilities.min_image_count.max(2),
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                ..Default::default()
            }
        )?)
    }

    pub fn acquire_swapchain_image(&mut self) -> Option<AcquireSwapchainImageResult> {
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();
        match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => Some(AcquireSwapchainImageResult::new(r.0 as usize, r.1, r.2)),
            Err(Validated::Error(VulkanError::OutOfDate)) => None,
            Err(e) => panic!("Couldn't acquire image: {}", e)
        }
    }

    pub fn present<T>(&mut self, context: &T, buffer: Arc<PrimaryAutoCommandBuffer>, acquire: AcquireSwapchainImageResult) 
    where T: HasQueue + HasLogicalDevice {
        let future = self 
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire.future)
            .then_execute(context.queue(), buffer)
            .unwrap()
            .then_swapchain_present(
                context.queue(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), acquire.image_index as u32),
            )
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate = true;
                self.previous_frame_end = Some(sync::now(context.logical_device()).boxed());
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                self.previous_frame_end = Some(sync::now(context.logical_device()).boxed());
            }
        }
    }

    pub fn new<T>(context: &T, window: Arc<Box<dyn Window>>) -> Result<Self>
    where T: HasPhysicalDevice + HasSurface + HasLogicalDevice + HasStandardMemoryAllocator {
        let (swapchain, images)
            = BaseSwapchain::create_swapchain_and_images(context, window.clone())?;
        let framebuffers = vec![];
        // let fences = vec![None; images.len()];
        // let previous_fence = 0;
        Ok(Self {
            swapchain, images, framebuffers, // fences, previous_fence
            previous_frame_end: Some(sync::now(context.logical_device()).boxed()),
            recreate: false
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
        self.recreate = false;
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