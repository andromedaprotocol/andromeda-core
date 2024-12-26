use super::{handler::AttributeHandler, utils::generate_match_pattern};
use quote::quote;

pub struct PayableAttribute;

impl AttributeHandler for PayableAttribute {
    fn check_attribute(&self, attr: &syn::Attribute) -> bool {
        if attr.path().is_ident("attrs") {
            let mut is_nonpayable = false;
            attr.parse_args_with(|input: syn::parse::ParseStream| {
                while !input.is_empty() {
                    let ident: syn::Ident = input.parse()?;
                    if ident == "nonpayable" {
                        is_nonpayable = true;
                    }
                    if !input.is_empty() {
                        input.parse::<syn::Token![,]>()?;
                    }
                }
                Ok(())
            })
            .unwrap_or(());
            return is_nonpayable;
        }
        false
    }

    fn generate_match_impl(
        &self,
        data_enum: &syn::DataEnum,
        variants: &[(syn::Ident, bool)],
    ) -> proc_macro2::TokenStream {
        let match_arms = variants.iter().map(|(variant_name, is_nonpayable)| {
            let pattern = generate_match_pattern(data_enum, variant_name);
            quote! { #pattern => !#is_nonpayable }
        });

        quote! {
            #[inline]
            pub fn is_payable(&self) -> bool {
                match self {
                    #(#match_arms,)*
                }
            }
        }
    }
}
