use super::*;
use winit::event::DeviceEvent;

impl ExtraEvent {
    pub async fn handle(mut self) -> (PhysicalPosition<f64>, PhysicalSize<f64>) {
        if let DeviceEvent::MouseMotion { delta } = self.event {
            self.cursor_position = PhysicalPosition::new(
                self.cursor_position.0 + delta.0,
                self.cursor_position.1 + delta.1,
            );
        } else if let DeviceEvent::B

        let mut renderer = self.renderer.lock().await;
        let window = self.window.lock().await;
        if let Some(window) = window.as_ref() {
            renderer.handle_event(window, &self.event);
        }
        (self.cursor_position, self.window_size)
    }
}
