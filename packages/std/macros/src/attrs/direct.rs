use super::{handler::AttributeHandler, utils::generate_match_pattern};
use quote::quote;

/**
 * DirectAttribute is used to indicate that a message cannot be received via an AMP packet.
 *
 * Example usage:
 * ```rust
 * #[andr_exec]
 * enum ExecuteMsg {
 *     #[attrs(direct)]
 *     MyMessage{..},
 * }
 * ```
 */
pub struct DirectAttribute;

impl AttributeHandler for DirectAttribute {
    fn check_attribute(&self, attr: &syn::Attribute) -> bool {
        if attr.path().is_ident("attrs") {
            let mut is_direct = false;
            attr.parse_args_with(|input: syn::parse::ParseStream| {
                while !input.is_empty() {
                    let ident: syn::Ident = input.parse()?;
                    if ident == "direct" {
                        is_direct = true;
                    }
                    if !input.is_empty() {
                        input.parse::<syn::Token![,]>()?;
                    }
                }
                Ok(())
            })
            .unwrap_or(());
            return is_direct;
        }
        false
    }

    fn generate_match_impl(
        &self,
        data_enum: &syn::DataEnum,
        variants: &[(syn::Ident, bool)],
    ) -> proc_macro2::TokenStream {
        let match_arms = variants.iter().map(|(variant_name, is_direct)| {
            let pattern = generate_match_pattern(data_enum, variant_name);
            quote! { #pattern => #is_direct }
        });

        quote! {
            #[inline]
            pub fn must_be_direct(&self) -> bool {
                match self {
                    #(#match_arms,)*
                }
            }
        }
    }
}
