use super::*;
use imgui::{Condition, Ui};
use std::collections::HashMap;

impl Inputs {
    fn render_input(
        ui: &Ui,
        (name, input): (&str, &Input),
        scope: &str,
        map: &mut HashMap<String, InputValue>,
    ) -> Option<()> {
        let input_name = format!("{}.{}", scope, name);
        match input {
            &Input::CHECKBOX => {
                let entry = map
                    .entry(input_name)
                    .or_insert_with(|| InputValue::CHECKBOX(false));
                match entry {
                    &mut InputValue::CHECKBOX(ref mut checked) => {
                        ui.checkbox(name, checked);
                    }
                    _ => {
                        *entry = InputValue::CHECKBOX(false);
                        if let &mut InputValue::CHECKBOX(ref mut checked) = entry {
                            ui.checkbox(name, checked);
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
            &Input::SLIDER(lower, upper) => {
                let entry = map
                    .entry(input_name)
                    .or_insert_with(|| InputValue::SLIDER(lower));
                match entry {
                    &mut InputValue::SLIDER(ref mut value) => {
                        ui.slider(name, lower, upper, value);
                    }
                    _ => {
                        *entry = InputValue::SLIDER(lower);
                        if let &mut InputValue::SLIDER(ref mut value) = entry {
                            ui.slider(name, lower, upper, value);
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
            Input::GROUP(inputs) => {
                let scope = input_name;

                let mut inputs: Vec<_> = inputs
                    .iter()
                    .map(|(name, input)| (name.as_str(), input))
                    .collect();
                inputs.sort_by_key(|&(name, _)| name);

                for input in inputs {
                    Self::render_input(ui, input, scope.as_str(), map)?;
                }
            }
        }

        Some(())
    }

    pub async fn render(&self, ui: &mut imgui::Ui, values: InputState) {
        let mut values = values.lock().await;

        for (idx, block) in self.blocks.iter().enumerate() {
            let default_window_title = format!("Input block {}", idx);
            let window_title = if let Some(ref title) = block.name {
                title.as_str()
            } else {
                default_window_title.as_str()
            };
            let scope = if let Some(ref title) = block.name {
                title.clone()
            } else {
                format!("{}", idx)
            };

            let mut ui_window = ui.window(window_title);
            if let Some(size) = block.size {
                ui_window = ui_window.size(size, Condition::Always);
            }

            let mut run = false;
            ui_window = ui_window.opened(&mut run).movable(true).resizable(true);

            let mut inputs: Vec<_> = block
                .inputs
                .iter()
                .map(|(name, input)| (name.as_str(), input))
                .collect();
            inputs.sort_by_key(|&(name, _)| name);

            ui_window.build(|| {
                for input in inputs {
                    if Self::render_input(ui, input, scope.as_str(), values.as_mut()).is_none() {
                        log::error!("aftgraphs::input::render failed to render inputs");
                    }
                }
            });
        }
    }
}
