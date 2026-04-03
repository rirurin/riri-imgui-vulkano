use crate::result::Result;
use glam::UVec2;
use riri_imgui_vulkano::resources::{HasLogicalDevice, HasPhysicalDevice, HasRenderPass, HasStandardMemoryAllocator, HasSurface, HasSwapchain};
use riri_imgui_vulkano::swapchain::{BaseSwapchain, SwapchainImpl};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo};
use vulkano::swapchain::{Swapchain, SwapchainCreateInfo};
use winit::window::Window;

#[derive(Debug)]
pub struct AppSwapchain {
    pub base: BaseSwapchain,
    pub depth_stencil: Arc<ImageView>,
}

impl Deref for AppSwapchain {
    type Target = BaseSwapchain;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for AppSwapchain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl HasSwapchain for AppSwapchain {
    fn swapchain(&self) -> Arc<Swapchain> {
        self.base.swapchain.clone()
    }
}

impl SwapchainImpl for AppSwapchain {
    fn make_framebuffer<R: HasRenderPass>(&self, image: Arc<Image>, render_pass: &R) -> Result<Arc<Framebuffer>> {
        Ok(Framebuffer::new(
            render_pass.render_pass(),
            FramebufferCreateInfo {
                attachments: vec![ImageView::new_default(image)?, self.depth_stencil.clone()],
                ..Default::default()
            },
        )?)
    }

    fn refresh<
        T0: HasStandardMemoryAllocator,
        T1: HasRenderPass
    >(&mut self, context: &T0, render_pass: &T1, extent: UVec2) -> Result<()> {
        self.depth_stencil = Self::make_depth_stencil(context, extent.to_array())?;
        (self.swapchain, self.images) = self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: extent.to_array(), ..self.swapchain.create_info()
        })?;
        self.set_framebuffers(render_pass)?;
        Ok(())
    }
}

impl AppSwapchain {
    fn make_depth_stencil<
        T: HasStandardMemoryAllocator
    >(context: &T, extent: [u32; 2]) -> Result<Arc<ImageView>> {
        let image = Image::new(
            context.allocator(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D16_UNORM,
                extent: [extent[0], extent[1], 1],
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            })?;
        Ok(ImageView::new_default(image.clone())?)
    }

    pub fn new<T>(context: &T, window: Arc<Box<dyn Window>>) -> Result<Self>
    where T: HasPhysicalDevice + HasSurface + HasLogicalDevice + HasStandardMemoryAllocator {
        let base = BaseSwapchain::new(context, window.clone())?;
        let depth_stencil = Self::make_depth_stencil(
            context, window.surface_size().into())?;
        Ok(Self { base, depth_stencil })
    }
}