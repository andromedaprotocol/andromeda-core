mod attrs;
mod execute;
mod instantiate;
mod query;
mod utils;

use proc_macro::TokenStream;

#[proc_macro_attribute]
/// Attaches all relevant ADO messages to a set of Execute messages for a given contract.
///
/// Also derives the `AsRefStr` trait for the enum allowing the use of `as_ref_str` to get the string representation of the enum variant.
///
/// e.g. `ExecuteMsg::MyMessage{..}.as_ref_str()` will return `"MyMessage"`
///
/// **Must be placed before `#[cw_serde]`**
pub fn andr_exec(args: TokenStream, input: TokenStream) -> TokenStream {
    execute::enum_implementation(args, input)
}

/// Modifies the execute entrypoint to unwrap AMP messages if they are received.
///
/// It also constructs the `ExecuteCtx` and passes it to the function.
///
/// **Please note that this will replace `ctx.info.sender` with the `origin` of an AMP packet if it is received. If you wish to use
/// the original `info.sender` use `ctx.raw_info.sender`.**
///
/// Example usage:
/// ```rust
/// #[andr_execute_fn]
/// pub fn execute(ctx: ExecuteCtx, msg: ExecuteMsg) -> Result<Response, ContractError> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn andr_execute_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    execute::fn_implementation(_attr, item)
}

/// Adjusted from https://users.rust-lang.org/t/solved-derive-and-proc-macro-add-field-to-an-existing-struct/52307/3
/// Adds all fields required to instantiate an ADO to a struct.
///
/// Includes:
/// 1. Kernel Address for interacting with aOS
/// 2. Owner of the ADO (optional, assumed to be sender otherwise)
#[proc_macro_attribute]
pub fn andr_instantiate(args: TokenStream, input: TokenStream) -> TokenStream {
    instantiate::enum_implementation(args, input)
}

#[proc_macro_attribute]
/// Attaches all relevant ADO messages to a set of Query messages for a given contract.
///
/// **Must be placed before `#[cw_serde]`**
pub fn andr_query(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    query::enum_implementation(_metadata, input)
}

/// Derives the `ExecuteAttrs` trait for a given enum.
///
/// This trait is used to allow attributes to be attached to the enum variants.
///
/// Example usage:
/// ```rust
/// #[andr_exec]
/// enum ExecuteMsg {
///     #[attrs(nonpayable, restricted)]
///     MyMessage{..},
/// }
/// ```
#[proc_macro_derive(ExecuteAttrs, attributes(attrs))]
pub fn derive_execute_attrs(input: TokenStream) -> TokenStream {
    attrs::derive_execute_attrs(input)
}
