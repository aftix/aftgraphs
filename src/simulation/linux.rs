use super::*;

impl ExtraEvent {
    pub async fn handle(self) -> (PhysicalPosition<f64>, PhysicalSize<f64>) {
        let mut renderer = self.renderer.lock().await;
        let window = self.window.lock().await;
        if let Some(window) = window.as_ref() {
            renderer.handle_event(window, &self.event);
        }
        (self.cursor_position, self.window_size)
    }
}
