// Trait for attribute handlers
pub trait AttributeHandler {
    /// Checks if the attribute is present
    fn check_attribute(&self, attr: &syn::Attribute) -> bool;

    /// Generates the implementation for the attribute
    fn generate_impl(
        &self,
        data_enum: &syn::DataEnum,
        variants: &[(syn::Ident, bool)],
    ) -> proc_macro2::TokenStream;
}
