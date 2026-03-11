use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use vulkano::format::Format;
use vulkano::render_pass::RenderPass;
use vulkano::swapchain::Swapchain;
use crate::error::Result;
use crate::resources::{HasLogicalDevice, HasRenderPass};

#[derive(Debug)]
#[repr(transparent)]
pub struct LibRenderPass(Arc<RenderPass>);

impl HasRenderPass for LibRenderPass {
    fn render_pass(&self) -> Arc<RenderPass> {
        self.0.clone()
    }
}

pub trait RenderPassBuilder {
    fn build(&self) -> Result<LibRenderPass>;
}

#[derive(Debug)]
pub struct GenericRenderPass<'a, T: Debug + HasLogicalDevice> {
    context: &'a T,
    swapchain: Arc<Swapchain>
}

impl<'a, T: Debug + HasLogicalDevice> GenericRenderPass<'a, T> {
    pub fn new(context: &'a T, swapchain: Arc<Swapchain>) -> Self {
        Self { context, swapchain }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ImguiRenderPass<'a, T: Debug + HasLogicalDevice>(GenericRenderPass<'a, T>);


impl<'a, T: Debug + HasLogicalDevice> Deref for ImguiRenderPass<'a, T> {
    type Target = GenericRenderPass<'a, T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: Debug + HasLogicalDevice> DerefMut for ImguiRenderPass<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: Debug + HasLogicalDevice> ImguiRenderPass<'a, T> {
    pub fn new(context: &'a T, swapchain: Arc<Swapchain>) -> Self {
        Self(GenericRenderPass::new(context, swapchain))
    }
}

impl<'a, T: Debug + HasLogicalDevice> RenderPassBuilder for ImguiRenderPass<'a, T> {
    fn build(&self) -> Result<LibRenderPass> {
        Ok(LibRenderPass(vulkano::single_pass_renderpass!(
            self.context.logical_device(),
            attachments: {
                color: {
                    format: self.swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )?))
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct DepthRenderPass<'a, T: Debug + HasLogicalDevice>(GenericRenderPass<'a, T>);

impl<'a, T: Debug + HasLogicalDevice> Deref for DepthRenderPass<'a, T> {
    type Target = GenericRenderPass<'a, T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: Debug + HasLogicalDevice> DerefMut for DepthRenderPass<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: Debug + HasLogicalDevice> DepthRenderPass<'a, T> {
    pub fn new(context: &'a T, swapchain: Arc<Swapchain>) -> Self {
        Self(GenericRenderPass::new(context, swapchain))
    }
}

impl<'a, T: Debug + HasLogicalDevice> RenderPassBuilder for DepthRenderPass<'a, T> {
    fn build(&self) -> Result<LibRenderPass> {
        Ok(LibRenderPass(vulkano::single_pass_renderpass!(
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
                    store_op: DontCare,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )?))
    }
}