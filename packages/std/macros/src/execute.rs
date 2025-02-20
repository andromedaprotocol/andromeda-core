use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, ItemFn};

use crate::utils::merge_variants;

pub fn enum_implementation(_args: TokenStream, input: TokenStream) -> TokenStream {
    #[allow(unused_mut)]
    let mut merged = merge_variants(
        input,
        quote! {
            enum Right {
                #[serde(rename="amp_receive")]
                AMPReceive(::andromeda_std::amp::messages::AMPPkt),
                Ownership(::andromeda_std::ado_base::ownership::OwnershipMessage),
                UpdateKernelAddress {
                    address: ::cosmwasm_std::Addr,
                },
                UpdateAppContract {
                    address: String,
                },
                Permissioning(::andromeda_std::ado_base::permissioning::PermissioningMessage),
            }
        }
        .into(),
    );

    #[cfg(feature = "rates")]
    {
        merged = merge_variants(
            merged,
            quote! {
                enum Right {
                    Rates(::andromeda_std::ado_base::rates::RatesMessage)
                }
            }
            .into(),
        )
    }

    let input = parse_macro_input!(merged as DeriveInput);
    let output = andr_exec_derive(input);

    quote! {
        #output
    }
    .into()
}

fn andr_exec_derive(input: DeriveInput) -> proc_macro2::TokenStream {
    match &input.data {
        syn::Data::Enum(_) => {
            parse_quote! {
                #[derive(::andromeda_std::AsRefStr, ::andromeda_std::ExecuteAttrs)]
                #input
            }
        }
        _ => panic!("unions are not supported"),
    }
}

pub(crate) fn fn_implementation(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let body = &input.block;

    let expanded = quote! {
        #[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
        pub fn execute(
            deps: ::cosmwasm_std::DepsMut,
            env: ::cosmwasm_std::Env,
            info: ::cosmwasm_std::MessageInfo,
            msg: ExecuteMsg,
        ) -> Result<::cosmwasm_std::Response, ContractError> {
            let (ctx, msg, resp) = ::andromeda_std::unwrap_amp_msg!(deps, info.clone(), env, msg);

            // Check if the message is restricted to the owner
            if msg.is_restricted() {
                let is_owner = ctx.contract.is_contract_owner(ctx.deps.storage, info.sender.as_str())?;
                ::cosmwasm_std::ensure!(
                    is_owner,
                    ::andromeda_std::error::ContractError::Unauthorized {}
                );
            }

            // Check if the message is payable
            if !msg.is_payable() {
                ::cosmwasm_std::ensure!(info.funds.is_empty(), ::andromeda_std::error::ContractError::Payment(::andromeda_std::error::PaymentError::NonPayable {}));
            }

            let res = execute_inner(ctx, msg)?;

            Ok(res
                .add_submessages(resp.messages)
                .add_attributes(resp.attributes)
                .add_events(resp.events))
        }

        #vis fn execute_inner(ctx: ::andromeda_std::common::context::ExecuteContext, msg: ExecuteMsg) -> Result<::cosmwasm_std::Response, ContractError> {
            #body
        }
    };

    TokenStream::from(expanded)
}
