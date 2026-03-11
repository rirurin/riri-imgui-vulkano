pub(crate) mod app;
pub(crate) mod clipboard;
pub(crate) mod color;
pub(crate) mod logger;
pub(crate) mod renderer;
pub(crate) mod result;
pub(crate) mod version;

fn main() {
    logger::Logger::init_task();
    app::App::execute();
}
