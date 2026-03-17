mod app;
mod camera;
mod color;
mod logger;
mod renderer {
    pub(crate) mod commands;
    pub(crate) mod context;
    pub(crate) mod pipeline;
    pub(crate) mod render_pass;
    pub(crate) mod swapchain;
}
mod result;
mod version;

fn main() {
    logger::Logger::init_task();
    app::App::execute();
}
