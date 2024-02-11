use crate::prelude::{Arc, Mutex};
use async_mutex::MutexGuard;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Input {
    // An input slider: name, [lower bound, upper bound]
    SLIDER(f64, f64),
    #[default]
    CHECKBOX,
    #[serde(untagged)]
    GROUP(HashMap<String, Input>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputValue {
    SLIDER(f64),
    CHECKBOX(bool),
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
    values: Arc<Mutex<HashMap<String, InputValue>>>,
}

#[derive(Debug)]
pub struct InputStateGuard<'a> {
    guard: MutexGuard<'a, HashMap<String, InputValue>>,
}

impl InputState {
    pub async fn lock(&self) -> InputStateGuard {
        InputStateGuard {
            guard: self.values.lock().await,
        }
    }
}

impl InputStateGuard<'_> {
    pub fn get(&self, name: &str) -> Option<&InputValue> {
        self.guard.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut InputValue> {
        self.guard.get_mut(name)
    }

    pub fn as_ref(&self) -> &HashMap<String, InputValue> {
        &self.guard
    }

    pub fn as_mut(&mut self) -> &mut HashMap<String, InputValue> {
        &mut self.guard
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InputBlock {
    #[serde(rename = "_name")]
    pub name: Option<String>,
    #[serde(rename = "_size")]
    pub size: Option<[f32; 2]>,
    #[serde(flatten)]
    pub inputs: HashMap<String, Input>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InputMetadata {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Inputs {
    pub simulation: InputMetadata,
    #[serde(rename = "block", default)]
    pub blocks: Vec<InputBlock>,
}

impl Inputs {
    pub fn new(data: impl AsRef<str>) -> anyhow::Result<Self> {
        toml::from_str(data.as_ref()).map_err(|err| anyhow::anyhow!("Inputs::new: {}", err))
    }

    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let data = read_to_string(path)
            .map_err(|err| anyhow::anyhow!("Inputs::from_file: failed to read file: {}", err))?;
        Self::new(data).map_err(|err| anyhow::anyhow!("Inputs::from_file: {}", err))
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod linux;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_string() {
        assert_eq!(None, Inputs::new("").ok())
    }

    #[test]
    fn no_blocks() {
        let document = r#"
            [simulation]
            name = "test"
        "#;

        let result = Inputs::new(document).unwrap();

        assert_eq!(
            Inputs {
                simulation: InputMetadata {
                    name: "test".to_owned(),
                    description: None,
                },
                blocks: vec![],
            },
            result
        );
    }

    #[test]
    fn description() {
        let document = r#"
            [simulation]
            name = "test"
            description = "testing"
        "#;

        let result = Inputs::new(document).unwrap();

        assert_eq!(
            Inputs {
                simulation: InputMetadata {
                    name: "test".to_owned(),
                    description: Some("testing".to_owned()),
                },
                blocks: vec![],
            },
            result
        );
    }

    #[test]
    fn block() {
        let document = r#"
            [simulation]
            name = "test"

            [[block]]
            slider = { SLIDER = [0.0, 1.0] }
            checkbox = "CHECKBOX"

            [block.group]
            inner_slider = { SLIDER = [1.0, 2.0] }
            inner_checkbox = "CHECKBOX"
        "#;

        let result = Inputs::new(document).unwrap();

        let simulation = InputMetadata {
            name: "test".to_owned(),
            description: None,
        };

        let inner_block_map: HashMap<String, Input> = [
            ("inner_slider".to_owned(), Input::SLIDER(1.0, 2.0)),
            ("inner_checkbox".to_owned(), Input::CHECKBOX),
        ]
        .into_iter()
        .collect();

        let block_map: HashMap<String, Input> = [
            ("slider".to_owned(), Input::SLIDER(0.0, 1.0)),
            ("checkbox".to_owned(), Input::CHECKBOX),
            ("group".to_owned(), Input::GROUP(inner_block_map)),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            Inputs {
                simulation,
                blocks: vec![InputBlock {
                    inputs: block_map,
                    ..Default::default()
                }],
            },
            result
        );
    }

    #[test]
    fn block_name() {
        let document = r#"
            [simulation]
            name = "test"

            [[block]]
            _name = "test block"
            _size = [400.0, 400.0]
            slider = { SLIDER = [0.0, 1.0] }
            checkbox = "CHECKBOX"

            [block.group]
            inner_slider = { SLIDER = [1.0, 2.0] }
            inner_checkbox = "CHECKBOX"
        "#;

        let result = Inputs::new(document).unwrap();

        let simulation = InputMetadata {
            name: "test".to_owned(),
            description: None,
        };

        let inner_block_map: HashMap<String, Input> = [
            ("inner_slider".to_owned(), Input::SLIDER(1.0, 2.0)),
            ("inner_checkbox".to_owned(), Input::CHECKBOX),
        ]
        .into_iter()
        .collect();

        let block_map: HashMap<String, Input> = [
            ("slider".to_owned(), Input::SLIDER(0.0, 1.0)),
            ("checkbox".to_owned(), Input::CHECKBOX),
            ("group".to_owned(), Input::GROUP(inner_block_map)),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            Inputs {
                simulation,
                blocks: vec![InputBlock {
                    name: Some("test block".to_owned()),
                    size: Some([400.0, 400.0]),
                    inputs: block_map
                }],
            },
            result
        );
    }
}
