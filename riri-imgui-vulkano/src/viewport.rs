use glam::{UVec2, Vec2};
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport};
use winit::window::Window;

pub struct ViewportBuilder;
impl ViewportBuilder {
    pub fn from_window(window: &dyn Window) -> Viewport {
        Viewport {
            offset: Vec2::ZERO.into(),
            extent: window.surface_size().into(),
            depth_range: 0f32..=1.
        }
    }

    pub fn from_extent(extent: Vec2) -> Viewport {
        Viewport {
            offset: Vec2::ZERO.into(),
            extent: [extent.x, extent.y],
            depth_range: 0f32..=1.,
        }
    }
}

pub struct ScissorBuilder;
impl ScissorBuilder {
    pub fn from_window(window: &dyn Window) -> Scissor {
        Scissor {
            offset: UVec2::ZERO.into(),
            extent: window.surface_size().into()
        }
    }
}