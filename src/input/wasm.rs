use super::*;
use crate::ui::{Ui, UiFrame};
use lazy_static::lazy_static;
use std::collections::hash_map::Entry;
use wasm_bindgen::JsCast;
use web_sys::{
    self, Element, HtmlFieldSetElement, HtmlFormElement, HtmlInputElement, HtmlLabelElement,
    HtmlLegendElement, Node,
};

lazy_static! {
    static ref INPUT_STATE: Mutex<HashMap<String, InputValue>> = Mutex::new(HashMap::new());
}

impl Inputs {
    fn create_input((name, input): (&str, &Input), scope: &str, ui: &mut Ui) -> Element {
        let input_name = format!("{}-{}", scope, name);
        let sanitized_name = input_name.replace(' ', "_");

        match input {
            Input::CHECKBOX => {
                let label_elem = ui.document.create_element("label").unwrap();
                let label_elem: HtmlLabelElement = label_elem.dyn_into().unwrap();
                label_elem.set_html_for(sanitized_name.as_str());
                label_elem.set_inner_text(name);

                let input_elem = ui.document.create_element("input").unwrap();
                let input_elem: HtmlInputElement = input_elem.dyn_into().unwrap();
                input_elem.set_id(sanitized_name.as_str());
                input_elem.set_type("checkbox");

                let div = ui.document.create_element("div").unwrap();
                div.set_class_name("inputset");

                div.append_child(&input_elem).unwrap();
                div.append_child(&label_elem).unwrap();
                div.append_child(&ui.document.create_element("br").unwrap())
                    .unwrap();

                div
            }
            Input::SLIDER(lower, upper) => {
                let label_elem = ui.document.create_element("label").unwrap();
                let label_elem: HtmlLabelElement = label_elem.dyn_into().unwrap();
                label_elem.set_html_for(sanitized_name.as_str());
                label_elem.set_inner_text(name);

                let input_elem = ui.document.create_element("input").unwrap();
                let input_elem: HtmlInputElement = input_elem.dyn_into().unwrap();
                input_elem.set_id(sanitized_name.as_str());
                input_elem.set_type("range");
                input_elem
                    .set_attribute("min", &ToString::to_string(&lower))
                    .unwrap();
                input_elem
                    .set_attribute("max", &ToString::to_string(&upper))
                    .unwrap();
                input_elem.set_attribute("step", "any").unwrap();
                input_elem.set_value_as_number(*lower);

                let div = ui.document.create_element("div").unwrap();
                div.set_class_name("inputset");

                div.append_child(&input_elem).unwrap();
                div.append_child(&label_elem).unwrap();
                div.append_child(&ui.document.create_element("br").unwrap())
                    .unwrap();

                div
            }
            Input::GROUP(inputs) => {
                let scope = sanitized_name;

                let mut inputs: Vec<_> = inputs
                    .iter()
                    .map(|(name, input)| (name.as_str(), input))
                    .collect();
                inputs.sort_by_key(|&(name, _)| name);

                let fieldset_elem = ui.document.create_element("fieldset").unwrap();
                let fieldset_elem: HtmlFieldSetElement = fieldset_elem.dyn_into().unwrap();
                fieldset_elem.set_id(scope.as_str());
                fieldset_elem.set_name(scope.as_str());

                let legend_elem = ui.document.create_element("legend").unwrap();
                let legend_elem: HtmlLegendElement = legend_elem.dyn_into().unwrap();
                legend_elem.set_inner_text(name);
                fieldset_elem.append_child(&legend_elem).unwrap();

                for input in inputs {
                    let child = Self::create_input(input, scope.as_str(), ui);
                    fieldset_elem.append_child(&child).unwrap();
                }

                fieldset_elem.dyn_into().unwrap()
            }
        }
    }

    fn create_inputs(&self, ui: &mut Ui) {
        let form_elem = ui.document.create_element("form").unwrap();
        let form_elem: HtmlFormElement = form_elem.dyn_into().unwrap();

        for (idx, block) in self.blocks.iter().enumerate() {
            let default_block_title = format!("Input block {}", idx);
            let block_title = if let Some(ref title) = block.name {
                title.as_str()
            } else {
                default_block_title.as_ref()
            };
            let scope = if let Some(ref title) = block.name {
                title.clone()
            } else {
                format!("{}", idx)
            };
            let scope = scope.replace(' ', "_");

            let block_fieldset = ui.document.create_element("fieldset").unwrap();
            let block_fieldset: HtmlFieldSetElement = block_fieldset.dyn_into().unwrap();
            block_fieldset.set_id(scope.as_str());

            let block_legend = ui.document.create_element("legend").unwrap();
            let block_legend: HtmlLegendElement = block_legend.dyn_into().unwrap();
            block_legend.set_inner_text(block_title);
            block_fieldset.append_child(&block_legend).unwrap();

            let mut inputs: Vec<_> = block
                .inputs
                .iter()
                .map(|(name, input)| (name.as_str(), input))
                .collect();
            inputs.sort_by_key(|&(name, _)| name);

            for input in inputs {
                let child = Self::create_input(input, scope.as_ref(), ui);
                block_fieldset.append_child(&child).unwrap();
            }

            form_elem.append_child(&block_fieldset).unwrap();
        }

        let canvas_list = ui.document.get_elements_by_name("canvas");
        let body_node: &Node = &ui.body;
        body_node
            .insert_before(&form_elem, canvas_list.get(0).as_ref())
            .unwrap();

        ui.input_forms_created = true;
    }

    pub fn get_input(
        (name, input): (&str, &Input),
        scope: &str,
        ui: &mut Ui,
        state: &mut HashMap<String, InputValue>,
        old_state: &mut HashMap<String, InputValue>,
    ) {
        let input_name = format!("{}-{}", scope, name);
        let sanitized_name = input_name.replace(' ', "_");

        match input {
            Input::CHECKBOX => {
                if let Some(checkbox) = ui.document.get_element_by_id(sanitized_name.as_str()) {
                    if let Ok(checkbox) = checkbox.dyn_into::<HtmlInputElement>() {
                        let val = checkbox.checked();
                        let key = sanitized_name.replace('_', " ").replace('-', ".");

                        let old_entry = old_state.entry(key.clone());
                        let state_val = state.insert(key.clone(), InputValue::CHECKBOX(val));
                        if let Some(InputValue::CHECKBOX(state_val)) = state_val {
                            match &old_entry {
                                Entry::Occupied(old_entry) => {
                                    if *old_entry.get() != InputValue::CHECKBOX(state_val) {
                                        checkbox.set_checked(state_val);
                                        state.insert(key, InputValue::CHECKBOX(state_val));
                                    }
                                }
                                Entry::Vacant(_) => {
                                    checkbox.set_checked(state_val);
                                    state.insert(key, InputValue::CHECKBOX(state_val));
                                }
                            }
                        }
                    } else {
                        log::error!("Element for id {} is not input element", sanitized_name);
                    }
                } else {
                    log::error!("Could not find element for id {}", sanitized_name);
                }
            }
            Input::SLIDER(_, _) => {
                if let Some(range) = ui.document.get_element_by_id(sanitized_name.as_str()) {
                    if let Ok(range) = range.dyn_into::<HtmlInputElement>() {
                        let val = range.value_as_number();
                        let key = sanitized_name.replace('_', " ").replace('-', ".");

                        let old_entry = old_state.entry(key.clone());
                        let state_val = state.insert(key.clone(), InputValue::SLIDER(val));
                        if let Some(InputValue::SLIDER(state_val)) = state_val {
                            match &old_entry {
                                Entry::Occupied(old_entry) => {
                                    if *old_entry.get() != InputValue::SLIDER(state_val) {
                                        range.set_value_as_number(state_val);
                                        state.insert(key, InputValue::SLIDER(state_val));
                                    }
                                }
                                Entry::Vacant(_) => {
                                    range.set_value_as_number(state_val);
                                    state.insert(key, InputValue::SLIDER(state_val));
                                }
                            }
                        }
                    } else {
                        log::error!("Element for id {} is not input element", sanitized_name);
                    }
                } else {
                    log::error!("Could not find element for id {}", sanitized_name);
                }
            }
            Input::GROUP(inputs) => {
                let scope = sanitized_name;

                for input in inputs.iter().map(|(k, v)| (k.as_str(), v)) {
                    Self::get_input(input, scope.as_str(), ui, state, old_state);
                }
            }
        }
    }

    pub fn get_inputs(
        &self,
        ui: &mut Ui,
        state: &mut HashMap<String, InputValue>,
        old_state: &mut HashMap<String, InputValue>,
    ) {
        for (idx, block) in self.blocks.iter().enumerate() {
            let scope = if let Some(ref title) = block.name {
                title.clone()
            } else {
                format!("{}", idx)
            };

            for input in block.inputs.iter().map(|(k, v)| (k.as_str(), v)) {
                Self::get_input(input, scope.as_str(), ui, state, old_state);
            }
        }
    }

    pub async fn render<'a>(&'a self, ui: UiFrame<'a>, state: InputState) {
        if !ui.input_forms_created {
            self.create_inputs(ui);
        }

        let mut values = state.lock().await;
        let mut old_values = INPUT_STATE.lock().await;
        self.get_inputs(ui, &mut values.guard, &mut old_values);
        *old_values = values.guard.clone();
    }
}
