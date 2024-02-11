use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse2,
    token::Comma,
    Ident, LitStr, Result,
};

struct SimMain {
    id: Ident,
    shader_path: String,
    inputs_path: String,
}

impl Parse for SimMain {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;
        let _comma: Comma = input.parse()?;
        let inputs_path: LitStr = input.parse()?;
        let _comma: Comma = input.parse()?;
        let id = input.parse()?;

        Ok(Self {
            id,
            shader_path: path.value(),
            inputs_path: inputs_path.value(),
        })
    }
}

fn sim_main_impl(input: TokenStream) -> TokenStream {
    let SimMain {
        id,
        shader_path,
        inputs_path,
    } = parse2(input).expect("did not encounter Ident");

    quote! {
        #[cfg(target_arch = "wasm32")]
        use wasm_bindgen::prelude::*;

        #[cfg(target_arch = "wasm32")]
        #[wasm_bindgen(js_name = "simMain")]
        pub fn sim_main() {
            let inputs_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), #inputs_path));
            let inputs = aftgraphs::input::Inputs::new(inputs_src).unwrap();
            aftgraphs::sim_main(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), #shader_path)),
                inputs,
                #id::default(),
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        pub fn sim_main() {
            let inputs_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), #inputs_path));
            let inputs = aftgraphs::input::Inputs::new(inputs_src).unwrap();
            aftgraphs::sim_main(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), #shader_path)),
                inputs,
                #id::default(),
            );
        }
    }
}

// Macro parameters:
//   str literal containing path to wgsl (concat'd to CARGO_MANIFEST_DIR)
//   str literal containing path to simulation TOML (concat'd to CARGO_MANIFEST_DIR)
//   identifier literal which is the name of the simulation struct type
#[proc_macro]
pub fn sim_main(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sim_main_impl(input.into()).into()
}
