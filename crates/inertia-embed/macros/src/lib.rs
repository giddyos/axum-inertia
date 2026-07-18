//! Proc macro implementation for `inertia-embed`.

#![forbid(unsafe_code)]

mod build;
mod generate;
mod input;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Embeds a validated Vite production output into a static frontend value.
#[proc_macro]
pub fn embed_frontend(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as input::EmbedInput);
    match build::build(&input).and_then(generate::expand) {
        Ok(expansion) => expansion.into(),
        Err(error) => error.into_compile_error().into(),
    }
}
