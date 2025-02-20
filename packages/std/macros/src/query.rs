use crate::utils::merge_variants;
use proc_macro::TokenStream;
use quote::quote;

pub fn enum_implementation(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    #[allow(unused_mut)]
    let mut merged = merge_variants(
        input,
        quote! {
            enum Right {
                #[returns(andromeda_std::ado_base::ownership::ContractOwnerResponse)]
                Owner {},
                #[returns(andromeda_std::ado_base::ownership::ContractPotentialOwnerResponse)]
                OwnershipRequest {},
                #[returns(andromeda_std::ado_base::ado_type::TypeResponse)]
                Type {},
                #[returns(andromeda_std::ado_base::kernel_address::KernelAddressResponse)]
                KernelAddress {},
                #[returns(andromeda_std::ado_base::app_contract::AppContractResponse)]
                AppContract {},
                #[returns(andromeda_std::ado_base::ownership::PublisherResponse)]
                OriginalPublisher {},
                #[returns(andromeda_std::ado_base::block_height::BlockHeightResponse)]
                BlockHeightUponCreation {},
                #[returns(andromeda_std::ado_base::version::VersionResponse)]
                Version {},
                #[returns(andromeda_std::ado_base::version::ADOBaseVersionResponse)]
                #[schemars(example = "andromeda_std::ado_base::version::base_crate_version")]
                ADOBaseVersion {},
                #[returns(Vec<::andromeda_std::ado_base::permissioning::PermissionInfo>)]
                Permissions { actor: String, limit: Option<u32>, start_after: Option<String> },
                #[returns(Vec<String>)]
                PermissionedActions { },
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
                    #[returns(Option<::andromeda_std::ado_base::rates::Rate>)]
                    Rates {action: String},
                    #[returns(::andromeda_std::ado_base::rates::AllRatesResponse)]
                    AllRates {}
                }
            }
            .into(),
        )
    }
    merged
}
