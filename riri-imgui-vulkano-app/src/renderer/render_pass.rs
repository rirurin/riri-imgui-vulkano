use crate::result::Result;
use riri_imgui_vulkano::render_pass::{BaseRenderPass, ImguiRenderPass, LibRenderPass, RenderPassBuilder};
use riri_imgui_vulkano::resources::{HasLogicalDevice, HasRenderPass, HasSwapchain};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use vulkano::format::Format;
use vulkano::swapchain::Swapchain;

#[derive(Debug)]
#[repr(transparent)]
pub struct AppRenderPass<'a, T: Debug + HasLogicalDevice>(BaseRenderPass<'a, T>);

impl<'a, T: Debug + HasLogicalDevice> Deref for AppRenderPass<'a, T> {
    type Target = BaseRenderPass<'a, T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: Debug + HasLogicalDevice> DerefMut for AppRenderPass<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: Debug + HasLogicalDevice> AppRenderPass<'a, T> {
    pub fn new(context: &'a T, swapchain: Arc<Swapchain>) -> Self {
        Self(BaseRenderPass::new(context, swapchain))
    }
}

impl<'a, T: Debug + HasLogicalDevice> RenderPassBuilder for AppRenderPass<'a, T> {
    fn build(&self) -> Result<LibRenderPass> {
        Ok(vulkano::ordered_passes_renderpass!(
            self.context.logical_device(),
            attachments: {
                color: {
                    format: self.swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                depth: {
                    format: Format::D16_UNORM,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare
                }
            },
            passes: [
                {
                    color: [color],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [color],
                    depth_stencil: {},
                    input: []
                },
            ]
        )?.into())
    }
}