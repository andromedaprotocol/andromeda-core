// Trait for attribute handlers
pub trait AttributeHandler {
    fn check_attribute(&self, attr: &syn::Attribute) -> bool;
    fn generate_match_impl(
        &self,
        data_enum: &syn::DataEnum,
        variants: &[(syn::Ident, bool)],
    ) -> proc_macro2::TokenStream;
}
