use super::Renderer;
use web_time::Duration;
use winit::{event::Event, window::Window};

impl Renderer {
    pub fn handle_event<T>(&mut self, window: &Window, event: &Event<T>) {
        self.platform.handle_event(&mut self.ui, window, event);
    }

    pub async fn prepare_ui(&mut self, window: &Window) {
        self.platform.prepare_frame(&mut self.ui, window);
    }

    pub fn update_delta_time(&mut self, duration: Duration) {
        self.ui.context_mut().io_mut().update_delta_time(duration)
    }
}
