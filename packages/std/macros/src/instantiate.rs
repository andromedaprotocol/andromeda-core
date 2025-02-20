use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parser, parse_macro_input, DeriveInput};

pub(crate) fn enum_implementation(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            if let syn::Fields::Named(fields) = &mut struct_data.fields {
                fields.named.push(
                    syn::Field::parse_named
                        .parse2(quote! { pub kernel_address: String })
                        .unwrap(),
                );
                fields.named.push(
                    syn::Field::parse_named
                        .parse2(quote! { pub owner: Option<String> })
                        .unwrap(),
                );
            }

            quote! {
                #ast
            }
            .into()
        }
        _ => panic!("Macro only works with structs"),
    }
}
