use super::{handler::AttributeHandler, utils::generate_match_pattern};
use quote::quote;

const ATTR_KEY: &str = "permissionless";
/**
 * PermissionlessAttribute is used to indicate that a message can not be permissioned.
 *
 * Example usage:
 * ```rust
 * #[andr_exec]
 * enum ExecuteMsg {
 *     #[attrs(permissionless)]
 *     MyMessage{..},
 * }
 * ```
 */
pub struct PermissionlessAttribute;

impl AttributeHandler for PermissionlessAttribute {
    fn check_attribute(&self, attr: &syn::Attribute) -> bool {
        if attr.path().is_ident("attrs") {
            let mut is_permissionless = false;
            attr.parse_args_with(|input: syn::parse::ParseStream| {
                while !input.is_empty() {
                    let ident: syn::Ident = input.parse()?;
                    if ident == ATTR_KEY {
                        is_permissionless = true;
                    }
                    if !input.is_empty() {
                        input.parse::<syn::Token![,]>()?;
                    }
                }
                Ok(())
            })
            .unwrap_or(());
            return is_permissionless;
        }
        false
    }

    fn generate_impl(
        &self,
        data_enum: &syn::DataEnum,
        variants: &[(syn::Ident, bool)],
    ) -> proc_macro2::TokenStream {
        let match_arms = variants.iter().map(|(variant_name, is_permissionless)| {
            let pattern = generate_match_pattern(data_enum, variant_name);
            quote! { #pattern => #is_permissionless }
        });

        quote! {
            #[inline]
            pub fn is_permissionless(&self) -> bool {
                match self {
                    #(#match_arms,)*
                }
            }
        }
    }
}
