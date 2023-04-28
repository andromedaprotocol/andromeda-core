use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::Parser, parse_macro_input, DeriveInput};

/// Taken from: https://github.com/DA0-DA0/dao-contracts/blob/74bd3881fdd86829e5e8b132b9952dd64f2d0737/packages/dao-macros/src/lib.rs#L9
/// Used to merge two enums together.
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
/// Attaches all relevant ADO messages to a set of Execute messages for a given contract.
///
/// Also derives the `AsRefStr` trait for the enum allowing the use of `as_ref_str` to get the string representation of the enum variant.
///
/// e.g. `ExecuteMsg::MyMessage{..}.as_ref_str()` will return `"MyMessage"`
///
/// **Must be placed before `#[cw_serde]`**
pub fn andr_exec(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let merged = merge_variants(
        metadata,
        input,
        quote! {
            enum Right {
                #[serde(rename="amp_receive")]
                AMPReceive(::andromeda_std::amp::messages::AMPPkt),
                UpdateOwner {
                    address: String,
                },
                UpdateOperators {
                    operators: Vec<String>,
                },
                UpdateAppContract {
                    address: String,
                },
                #[cfg(feature = "withdraw")]
                Withdraw {
                    recipient: Option<Recipient>,
                    tokens_to_withdraw: Option<Vec<Withdrawal>>,
                },
                #[cfg(feature = "modules")]
                RegisterModule {
                    module: ::andromeda_std::ado_base::Module,
                },
                #[cfg(feature = "modules")]
                DeregisterModule {
                    module_idx: Uint64,
                },
                #[cfg(feature = "modules")]
                AlterModule {
                    module_idx: Uint64,
                    module: ::andromeda_std::ado_base::Module,
                },
            }
        }
        .into(),
    );
    let input = parse_macro_input!(merged);
    TokenStream::from(andr_exec_derive(input).into_token_stream())
}

/// Derives the `AsRefStr` trait for a given enum allowing the use of `as_ref_str` to get the string representation of the enum.
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

/// Adjusted from https://users.rust-lang.org/t/solved-derive-and-proc-macro-add-field-to-an-existing-struct/52307/3
/// Adds all fields required to instantiate an ADO to a struct.
///
/// Includes:
/// 1. Kernel Address for interacting with aOS
/// 2. Owner of the ADO (optional, assumed to be sender otherwise)
/// 3. Modules (optional, requires `modules` feature)
#[proc_macro_attribute]
pub fn andr_instantiate(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
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
                    #[cfg(feature = "modules")]
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(quote! { pub modules: Option<Vec<::andromeda_std::ado_base::Module>> })
                            .unwrap(),
                    );
                }
                _ => (),
            }

            return quote! {
                #ast
            }
            .into();
        }
        _ => panic!("Macro only works with structs"),
    }
}

#[proc_macro_attribute]
/// Attaches all relevant ADO messages to a set of Query messages for a given contract.
///
/// **Must be placed before `#[cw_serde]`**
pub fn andr_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let merged = merge_variants(
        metadata,
        input,
        quote! {
            enum Right {
                #[returns(Option<::cosmwasm_std::Binary>)]
                Get(Option<Binary>),
                #[returns(andromeda_std::ado_base::ownership::ContractOwnerResponse)]
                Owner {},
                #[returns(andromeda_std::ado_base::operators::OperatorsResponse)]
                Operators {},
                #[returns(andromeda_std::ado_base::ado_type::TypeResponse)]
                Type {},
                #[returns(andromeda_std::ado_base::kernel_address::KernelAddressResponse)]
                KernelAddress {},
                #[returns(andromeda_std::ado_base::ownership::PublisherResponse)]
                OriginalPublisher {},
                #[returns(andromeda_std::ado_base::block_height::BlockHeightResponse)]
                BlockHeightUponCreation {},
                #[returns(andromeda_std::ado_base::operators::IsOperatorResponse)]
                IsOperator { address: String },
                #[cfg(feature = "modules")]
                #[returns(andromeda_std::ado_base::Module)]
                Module { id: ::cosmwasm_std::Uint64 },
                #[cfg(feature = "modules")]
                #[returns(Vec<String>)]
                ModuleIds {},
                #[returns(andromeda_std::ado_base::version::VersionResponse)]
                Version {},
            }
        }
        .into(),
    );
    merged
}
