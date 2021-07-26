// use cosmwasm_std::{
//     log, Api, Env, Extern, HandleResponse, HumanAddr, Querier, StdError, StdResult, Storage,
//     Uint128,
// };
// use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};

// use crate::extension::Extension;

// use crate::{
//     log::WhitelistLog,
//     msg::WhitelistedResponse,
//     require::require,
//     token::collection::{collection, collection_read, TokenCollection},
// };

// pub trait Whitelist {
//     fn get_whitelist_settings(&self) -> StdResult<Extension>;
//     fn extends_whitelist(&self) -> StdResult<bool>;
//     fn is_moderator(&self, addr: &HumanAddr) -> StdResult<bool>;
//     fn is_whitelisted<S: Storage>(&self, storage: &S, addr: &HumanAddr) -> StdResult<bool>;
//     fn add_whitelisted<S: Storage>(&self, storage: &mut S, addr: &HumanAddr) -> StdResult<bool>;
//     fn remove_whitelisted<S: Storage>(&self, storage: &mut S, addr: &HumanAddr) -> StdResult<bool>;
// }

// impl Whitelist for TokenCollection {
//     fn get_whitelist_settings(&self) -> StdResult<Extension> {
//         let extension: Option<Extension> = self.extensions.iter().find_map(|d| match d {
//             Extension::WhiteListExtension { moderators } => Some(Extension::WhiteListExtension {
//                 moderators: moderators.to_vec(),
//             }),
//             _ => None,
//         });

//         match extension {
//             Some(e) => Ok(e),
//             None => Err(StdError::generic_err(
//                 "Collection does not implement the 'whitelist' extension",
//             )),
//         }
//     }
//     fn extends_whitelist(&self) -> StdResult<bool> {
//         let has_whitelist = self.extensions.iter().any(|e| match e {
//             Extension::WhiteListExtension { .. } => true,
//             _ => false,
//         });

//         require(
//             has_whitelist,
//             StdError::generic_err("Collection does not implement the 'whitelist' extension"),
//         )
//     }
//     fn is_moderator(&self, addr: &HumanAddr) -> StdResult<bool> {
//         self.extends_whitelist()?;
//         if self.is_creator(addr) {
//             Ok(true)
//         } else {
//             let settings = self.get_whitelist_settings()?;
//             match settings {
//                 Extension::WhiteListExtension { moderators } => {
//                     Ok(moderators.iter().any(|moderator| moderator.eq(&addr)))
//                 }
//                 _ => Ok(false),
//             }
//         }
//     }
//     fn is_whitelisted<S: Storage>(&self, storage: &S, addr: &HumanAddr) -> StdResult<bool> {
//         self.extends_whitelist()?;

//         let whitelist_bucket = whitelist_read(storage, String::from(&self.symbol));
//         match whitelist_bucket.load(&addr.to_string().as_bytes()) {
//             Ok(a) => Ok(a),
//             _ => Ok(false),
//         }
//     }
//     fn add_whitelisted<S: Storage>(&self, storage: &mut S, addr: &HumanAddr) -> StdResult<bool> {
//         self.extends_whitelist()?;

//         if self.is_whitelisted(storage, &addr)? {
//             Ok(true)
//         } else {
//             let mut whitelist_bucket = whitelist(storage, String::from(&self.symbol));
//             whitelist_bucket.save(addr.to_string().as_bytes(), &true)?;
//             Ok(true)
//         }
//     }
//     fn remove_whitelisted<S: Storage>(&self, storage: &mut S, addr: &HumanAddr) -> StdResult<bool> {
//         self.extends_whitelist()?;

//         let mut whitelist_bucket = whitelist(storage, String::from(&self.symbol));
//         whitelist_bucket.remove(addr.to_string().as_bytes());
//         Ok(true)
//     }
// }

// pub fn whitelist<S: Storage>(storage: &mut S, collection_symbol: String) -> Bucket<S, bool> {
//     let whitelist_string = format!("whitelist::{}", &collection_symbol);
//     let whitelist_ns = whitelist_string.as_bytes();
//     bucket(whitelist_ns, storage)
// }

// pub fn whitelist_read<S: Storage>(
//     storage: &S,
//     collection_symbol: String,
// ) -> ReadonlyBucket<S, bool> {
//     let whitelist_string = format!("whitelist::{}", &collection_symbol);
//     let whitelist_ns = whitelist_string.as_bytes();
//     bucket_read(whitelist_ns, storage)
// }

// pub fn pre_mint<S: Storage>(storage: &S, env: &Env, collection_symbol: &String) -> StdResult<bool> {
//     let collection = collection_read(storage).load(collection_symbol.as_bytes())?;

//     require(
//         collection.is_whitelisted(storage, &env.message.sender)?,
//         StdError::unauthorized(),
//     )?;
//     Ok(true)
// }

// pub fn pre_transfer<S: Storage>(
//     storage: &S,
//     env: &Env,
//     collection_symbol: &String,
//     to: &HumanAddr,
// ) -> StdResult<bool> {
//     let collection = collection_read(storage).load(collection_symbol.as_bytes())?;

//     require(
//         collection.is_whitelisted(storage, &to)?,
//         StdError::unauthorized(),
//     )?;
//     require(
//         collection.is_whitelisted(storage, &env.message.sender)?,
//         StdError::unauthorized(),
//     )?;
//     Ok(true)
// }

// pub fn pre_burn<S: Storage>(storage: &S, env: &Env, collection_symbol: &String) -> StdResult<bool> {
//     let collection = collection_read(storage).load(collection_symbol.as_bytes())?;

//     require(
//         collection.is_whitelisted(storage, &env.message.sender)?,
//         StdError::unauthorized(),
//     )?;
//     Ok(true)
// }

// pub fn pre_archive<S: Storage>(
//     storage: &S,
//     env: &Env,
//     collection_symbol: &String,
// ) -> StdResult<bool> {
//     let collection = collection_read(storage).load(collection_symbol.as_bytes())?;

//     require(
//         collection.is_whitelisted(storage, &env.message.sender)?,
//         StdError::unauthorized(),
//     )?;
//     Ok(true)
// }

// pub fn pre_transfer_agreement<S: Storage>(
//     storage: &S,
//     env: &Env,
//     collection_symbol: &String,
//     _denom: &String,
//     _amount: &Uint128,
//     purchaser: &HumanAddr,
// ) -> StdResult<bool> {
//     let collection = collection_read(storage).load(collection_symbol.as_bytes())?;

//     require(
//         collection.is_whitelisted(storage, &env.message.sender)?,
//         StdError::unauthorized(),
//     )?;
//     require(
//         collection.is_whitelisted(storage, purchaser)?,
//         StdError::unauthorized(),
//     )?;

//     Ok(true)
// }

// pub fn query_whitelisted<S: Storage, A: Api, Q: Querier>(
//     deps: &Extern<S, A, Q>,
//     collection_symbol: String,
//     addr: HumanAddr,
// ) -> StdResult<WhitelistedResponse> {
//     let collection = collection_read(&deps.storage).load(&collection_symbol.as_bytes())?;
//     let whitelisted = collection.is_whitelisted(&deps.storage, &addr)?;

//     Ok(WhitelistedResponse { whitelisted })
// }

// pub fn try_whitelist<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env,
//     collection_symbol: String,
//     addr: HumanAddr,
// ) -> StdResult<HandleResponse> {
//     let collection = collection(&mut deps.storage).load(collection_symbol.as_bytes())?;

//     require(
//         collection.is_moderator(&env.message.sender)?,
//         StdError::unauthorized(),
//     )?;
//     require(
//         !collection.is_whitelisted(&deps.storage, &addr)?,
//         StdError::generic_err("Address is already whitelisted"),
//     )?;

//     collection.add_whitelisted(&mut deps.storage, &addr)?;

//     let whitelisted_addr = addr.clone();
//     Ok(HandleResponse {
//         messages: vec![],
//         log: vec![log(
//             "whitelist",
//             WhitelistLog {
//                 whitelisted: true,
//                 whitelister: env.message.sender,
//                 address: whitelisted_addr,
//             },
//         )],
//         data: None,
//     })
// }

// pub fn try_dewhitelist<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env,
//     collection_symbol: String,
//     addr: HumanAddr,
// ) -> StdResult<HandleResponse> {
//     let collection = collection(&mut deps.storage).load(collection_symbol.as_bytes())?;

//     require(
//         collection.is_moderator(&env.message.sender)?,
//         StdError::unauthorized(),
//     )?;
//     require(
//         collection.is_whitelisted(&deps.storage, &addr)?,
//         StdError::generic_err("Address is not currently whitelisted"),
//     )?;

//     collection.remove_whitelisted(&mut deps.storage, &addr)?;

//     let whitelisted_addr = addr.clone();
//     Ok(HandleResponse {
//         messages: vec![],
//         log: vec![log(
//             "whitelist",
//             WhitelistLog {
//                 whitelisted: false,
//                 whitelister: env.message.sender,
//                 address: whitelisted_addr,
//             },
//         )],
//         data: None,
//     })
// }
