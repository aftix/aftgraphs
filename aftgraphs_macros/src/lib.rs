use quote::quote;

#[cfg(not(target_family = "wasm"))]
#[proc_macro]
pub fn sim_main(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}

#[cfg(target_family = "wasm")]
pub fn sim_main(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}
