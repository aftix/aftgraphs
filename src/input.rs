use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Input {
    // An input slider: name, [lower bound, upper bound]
    SLIDER(f64, f64),
    CHECKBOX,
    #[serde(untagged)]
    GROUP(HashMap<String, Input>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputBlock {
    #[serde(flatten)]
    pub inputs: HashMap<String, Input>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMetadata {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn input_empty_string() {
        assert_eq!(None, Inputs::new("").ok())
    }

    #[test]
    fn input_no_blocks() {
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
    fn input_description() {
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
    fn input_block() {
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
                blocks: vec![InputBlock { inputs: block_map }],
            },
            result
        );
    }
}
