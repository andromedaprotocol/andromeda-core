#[cfg(test)]
mod app;

#[cfg(test)]
mod marketplace_app;

#[cfg(test)]
mod auction_app;

#[cfg(test)]
mod ibc_registry;

#[cfg(test)]
mod kernel;

#[cfg(test)]
#[path = "./tests-ibc/interchain.rs"]
mod interchain;

#[cfg(test)]
#[path = "./tests-ibc/fixed_amount_splitter_ibc.rs"]
mod fixed_amount_splitter_ibc;

#[cfg(test)]
#[path = "./tests-ibc/splitter_ibc.rs"]
mod splitter_ibc;

#[cfg(test)]
mod cw20_staking;

#[cfg(test)]
mod lockdrop;

#[cfg(test)]
mod validator_staking;

#[cfg(test)]
mod cw20_app;

#[cfg(test)]
mod fixed_amount_splitter;

#[cfg(test)]
mod shunting;
