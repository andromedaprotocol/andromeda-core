mod direct;
mod handler;
mod payable;
mod restricted;
mod utils;

use direct::DirectAttribute;
use handler::AttributeHandler;
use payable::NonPayableAttribute;
use proc_macro::TokenStream;
use quote::quote;
use restricted::RestrictedAttribute;
use syn::{parse_macro_input, DeriveInput};
use utils::process_variants_dynamic;

pub fn derive_execute_attrs(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match &input.data {
        syn::Data::Enum(data_enum) => {
            let name = &input.ident;

            // Define handlers
            let handlers: Vec<Box<dyn AttributeHandler>> = vec![
                Box::new(NonPayableAttribute),
                Box::new(RestrictedAttribute),
                Box::new(DirectAttribute),
            ];

            // Process variants and generate implementations for each handler
            let implementations = handlers
                .iter()
                .map(|handler| {
                    let variants = process_variants_dynamic(data_enum, handler.as_ref());
                    handler.generate_impl(data_enum, &variants)
                })
                .collect::<Vec<_>>();

            let expanded = quote! {
                impl #name {
                    #(#implementations)*
                }
            };

            TokenStream::from(expanded)
        }
        _ => panic!("Attrs can only be derived for enums"),
    }
}
