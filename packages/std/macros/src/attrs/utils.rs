use quote::quote;

use super::handler::AttributeHandler;

pub(crate) fn generate_match_pattern(
    data_enum: &syn::DataEnum,
    variant_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    let variant = data_enum
        .variants
        .iter()
        .find(|v| v.ident == *variant_name)
        .unwrap();

    match &variant.fields {
        syn::Fields::Named(_) => quote! { Self::#variant_name { .. } },
        syn::Fields::Unnamed(_) => quote! { Self::#variant_name(..) },
        syn::Fields::Unit => quote! { Self::#variant_name },
    }
}

pub(crate) fn process_variants_dynamic(
    data_enum: &syn::DataEnum,
    handler: &dyn AttributeHandler,
) -> Vec<(syn::Ident, bool)> {
    data_enum
        .variants
        .iter()
        .map(|variant| {
            let has_attr = variant
                .attrs
                .iter()
                .any(|attr| handler.check_attribute(attr));
            (variant.ident.clone(), has_attr)
        })
        .collect()
}
