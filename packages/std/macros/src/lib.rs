use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};

/// Taken from: https://github.com/DA0-DA0/dao-contracts/blob/74bd3881fdd86829e5e8b132b9952dd64f2d0737/packages/dao-macros/src/lib.rs#L9
fn merge_variants(metadata: TokenStream, left: TokenStream, right: TokenStream) -> TokenStream {
    use syn::Data::Enum;
    use syn::{AttributeArgs, DataEnum};

    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "macro takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut left: DeriveInput = parse_macro_input!(left);
    let right: DeriveInput = parse_macro_input!(right);

    if let (
        Enum(DataEnum { variants, .. }),
        Enum(DataEnum {
            variants: to_add, ..
        }),
    ) = (&mut left.data, right.data)
    {
        variants.extend(to_add.into_iter());

        quote! { #left }.into()
    } else {
        syn::Error::new(left.ident.span(), "variants may only be added for enums")
            .to_compile_error()
            .into()
    }
}

#[proc_macro_attribute]
pub fn andr_exec(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let merged = merge_variants(
        metadata,
        input,
        quote! {
            enum Right {
                AMPReceive(::andromeda_std::amp::messages::AMPPkt),
            }
        }
        .into(),
    );
    let input = parse_macro_input!(merged);
    TokenStream::from(andr_exec_derive(input).into_token_stream())
}

fn andr_exec_derive(input: DeriveInput) -> DeriveInput {
    use syn::parse_quote;

    match input.data {
        syn::Data::Enum(_) => parse_quote! {
            #[derive(::andromeda_std::AsRefStr)]
            #input
        },
        _ => panic!("unions are not supported"),
    }
}
