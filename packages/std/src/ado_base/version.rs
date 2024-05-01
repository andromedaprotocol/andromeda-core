use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct VersionResponse {
    pub version: String,
}

#[cw_serde]
pub struct ADOBaseVersionResponse {
    // andromeda-std version in semver format
    pub version: String,
}
