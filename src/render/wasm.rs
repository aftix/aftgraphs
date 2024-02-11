use super::Renderer;
use crate::input::Inputs;
use crate::prelude::InputValue;
use std::collections::HashMap;
use std::time::Duration;
use winit::{event::Event, window::Window};

impl Renderer {
    pub fn handle_event<T>(&mut self, _window: &Window, _event: &Event<T>) {}

    pub async fn prepare_ui(&mut self, _window: &Window) {}

    pub fn update_delta_time(&mut self, _duration: Duration) {}

    pub async fn draw_ui(
        &mut self,
        _window: &Window,
        _inputs: Inputs,
    ) -> Option<HashMap<String, InputValue>> {
        None
    }
}
