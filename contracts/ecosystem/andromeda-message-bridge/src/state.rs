use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Item;
use serde::Deserialize;

/// The address of the user that's authorized to use this bridge
pub const AUTHORIZED_USER: Item<Addr> = Item::new("authorized_user");

#[derive(Deserialize)]
pub struct UniversalNftInfoResponse {
    pub token_uri: Option<String>,

    #[serde(skip_deserializing)]
    #[allow(dead_code)]
    extension: Empty,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_binary, to_binary, Coin, Empty};

    use super::UniversalNftInfoResponse;

    #[test]
    fn test_universal_deserialize() {
        let start = cw721::NftInfoResponse::<Coin> {
            token_uri: None,
            extension: Coin::new(100, "ujuno"),
        };
        let start = to_binary(&start).unwrap();
        let end: UniversalNftInfoResponse = from_binary(&start).unwrap();
        assert_eq!(end.token_uri, None);
        assert_eq!(end.extension, Empty::default())
    }
}
