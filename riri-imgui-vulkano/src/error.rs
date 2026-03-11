use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use imgui::TextureId;
use vulkano::pipeline::PipelineLayout;

#[derive(Debug)]
pub enum LibError {
    NoPhysicalDevice,
    NoGraphicsQueue,
    FailToGetDescriptorSetLayout(Arc<PipelineLayout>),
    FailToMakeImageBuffer,
    FailToGetSubBuffer,
    NoSuitablePhysicalDevice,
    NoSurfaceCompositeAlpha,
    InvalidFileSizeForSpirvBytecode(usize),
    NoCommandBufferAtIndex(usize),
    CouldNotFindShader(String),
    MissingDescriptorSet(TextureId),
    NoFileExtensionOnShader
}

impl Error for LibError {}

// TODO: Make user friendly errors
impl Display for LibError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;