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
    path: String,
}

impl Parse for SimMain {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;
        let _comma: Comma = input.parse()?;
        let id = input.parse()?;

        Ok(Self {
            id,
            path: path.value(),
        })
    }
}

fn sim_main_impl(input: TokenStream) -> TokenStream {
    let SimMain { id, path } = parse2(input).expect("did not encounter Ident");

    quote! {
        #[cfg(target_arch = "wasm32")]
        use wasm_bindgen::prelude::*;

        #[cfg(target_arch = "wasm32")]
        #[wasm_bindgen(js_name = "simMain")]
        pub fn sim_main() {
            aftgraphs::sim_main(
                include_str!(#path),
                #id::default(),
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        pub fn sim_main() {
            aftgraphs::sim_main(
                include_str!(#path),
                #id::default(),
            );
        }
    }
}

#[proc_macro]
pub fn sim_main(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sim_main_impl(input.into()).into()
}
