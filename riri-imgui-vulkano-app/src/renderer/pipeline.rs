use riri_imgui_vulkano::pipeline::{Basic3dGraphicsPipeline, ImguiGraphicsPipeline};

#[derive(Debug)]
pub struct AppPipeline {
    pub(crate) basic3d: Basic3dGraphicsPipeline<0>,
    pub(crate) imgui: ImguiGraphicsPipeline<1>,
}

impl AppPipeline {
    pub fn new(basic3d: Basic3dGraphicsPipeline<0>, imgui: ImguiGraphicsPipeline<1>) -> Self {
        Self { basic3d, imgui }
    }
}

