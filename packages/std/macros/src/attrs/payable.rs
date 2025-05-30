use super::{handler::AttributeHandler, utils::generate_match_pattern};
use quote::quote;

const ATTR_KEY: &str = "nonpayable";
/**
 * NonPayableAttribute is used to indicate that a message can receive funds.
 *
 * Example usage:
 * ```rust
 * #[andr_exec]
 * enum ExecuteMsg {
 *     #[attrs(nonpayable)]
 *     MyMessage{..},
 * }
 * ```
 */
pub struct NonPayableAttribute;

impl AttributeHandler for NonPayableAttribute {
    fn check_attribute(&self, attr: &syn::Attribute) -> bool {
        if attr.path().is_ident("attrs") {
            let mut is_nonpayable = false;
            attr.parse_args_with(|input: syn::parse::ParseStream| {
                while !input.is_empty() {
                    let ident: syn::Ident = input.parse()?;
                    if ident == ATTR_KEY {
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

    fn generate_impl(
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
