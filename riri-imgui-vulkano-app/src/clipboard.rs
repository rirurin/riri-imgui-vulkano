// from imgui-rs examples

use std::error::Error;
use copypasta::{ClipboardContext, ClipboardProvider};
use imgui::ClipboardBackend;

#[repr(transparent)]
pub struct ClipboardSupport(pub ClipboardContext);

impl ClipboardSupport {
    pub fn new() -> Result<ClipboardSupport, Box<dyn Error + Send + Sync + 'static>> {
        ClipboardContext::new().map(ClipboardSupport)
    }
}

impl ClipboardBackend for ClipboardSupport {
    fn get(&mut self) -> Option<String> {
        self.0.get_contents().ok()
    }
    fn set(&mut self, text: &str) {
        // ignore errors?
        let _ = self.0.set_contents(text.to_owned());
    }
}