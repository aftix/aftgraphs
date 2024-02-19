use super::Renderer;
use crate::ui::UiPlatform;
use web_time::Duration;
use winit::{event::Event, window::Window};

impl<P: UiPlatform> Renderer<P> {
    pub fn handle_event<T>(&mut self, _window: &Window, _event: &Event<T>) {}

    pub async fn prepare_ui(&mut self, _window: &Window) {}

    pub fn update_delta_time(&mut self, _duration: Duration) {}
}
