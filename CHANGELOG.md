# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased
- feat: Added neutron testnet to deploy script [(##875)](https://github.com/andromedaprotocol/andromeda-core/pull/875)

### Added

- CW20 support for kernel routing [(#723)](https://github.com/andromedaprotocol/andromeda-core/pull/723)
- feat: Handle username on IBC packets [(#791)](https://github.com/andromedaprotocol/andromeda-core/pull/791)
- Added Time Gate ADO [(#529)](https://github.com/andromedaprotocol/andromeda-core/pull/529)
- feat: Add previous hops to AMP packets [(#796)](https://github.com/andromedaprotocol/andromeda-core/pull/796)
- feat: Permissionless macro attribute [(#812)](https://github.com/andromedaprotocol/andromeda-core/pull/812)
- feat: Add ID to AMP packets [(#813)](https://github.com/andromedaprotocol/andromeda-core/pull/813)
- feat: Batch send NFTs [(#816)](https://github.com/andromedaprotocol/andromeda-core/pull/816)
- feat: Added Permissionless attribute to relevant messages [(#825)](https://github.com/andromedaprotocol/andromeda-core/pull/825)
- feat: Apply Serde default to previous_hops [(#829)](https://github.com/andromedaprotocol/andromeda-core/pull/829)
- feat: Frequency in Permissioning [(#855)](https://github.com/andromedaprotocol/andromeda-core/pull/855)
- feat: CW20 Redeem ADO [(#842)](https://github.com/andromedaprotocol/andromeda-core/pull/842)
- feat: Update to CosmWasm 2.2 from 1.5 [(#850)](https://github.com/andromedaprotocol/andromeda-core/pull/850)
- feat: CW20 Exchange V2, delete CW20 Redeem [(#874)](https://github.com/andromedaprotocol/andromeda-core/pull/874)
- feat: Added Astroport socket with LP and swap functionality [(#885)](https://github.com/andromedaprotocol/andromeda-core/pull/885)
- feat: Create pool with funds and withdraw from pool in osmosis socket [(#886)](https://github.com/andromedaprotocol/andromeda-core/pull/886)
- feat: Osmosis Token Factory ADO [(#929)](https://github.com/andromedaprotocol/andromeda-core/pull/929)
- feat: Wildcard in Permissions [(#949)](https://github.com/andromedaprotocol/andromeda-core/pull/949)
- feat: Additional Permission Queries [(#960)](https://github.com/andromedaprotocol/andromeda-core/pull/960)
- feat: Standardized events in ADOS [(#970)](https://github.com/andromedaprotocol/andromeda-core/pull/970)
- feat: Kernel queries for all chains and all channels [(#978)](https://github.com/andromedaprotocol/andromeda-core/pull/978)

### Changed

- feat: Improved macros and execution flow for AMP [(#741)](https://github.com/andromedaprotocol/andromeda-core/pull/741)
- chore: remove unused contracts & code [(#790)](https://github.com/andromedaprotocol/andromeda-core/pull/790)
- feat: Recipient in rate limiting withdrawals [(#804)](https://github.com/andromedaprotocol/andromeda-core/pull/804)
- chore: refactored tests to be more readable, fixed ibc direct packet bug and test setup [(#808)](https://github.com/andromedaprotocol/andromeda-core/pull/808)
- feat: deploy script validate & build steps [(#736)](https://github.com/andromedaprotocol/andromeda-core/pull/736)
- ref: Simplify the kernel's handling of its Send ExecuteMsg [(#736)](https://github.com/andromedaprotocol/andromeda-core/pull/736)
- fix: kernel's tx id generation [(#851)](https://github.com/andromedaprotocol/andromeda-core/pull/851)
- ref: change cw20 exchange's sale start time and end time to Milliseconds, same for auction [(#869)](https://github.com/andromedaprotocol/andromeda-core/pull/869)
- feat: Create and apply Schedule struct [(#900)](https://github.com/andromedaprotocol/andromeda-core/pull/900)
- feat: Enable Cross Chain Components in App [(#932)](https://github.com/andromedaprotocol/andromeda-core/pull/932)
- refactor: Make recipients list optional in Splitter [(#937)](https://github.com/andromedaprotocol/andromeda-core/pull/937)


### Fixed

- fix: permission migration [(#792)](https://github.com/andromedaprotocol/andromeda-core/pull/792)
- fix: IBC denom casing [(#795)](https://github.com/andromedaprotocol/andromeda-core/pull/795)
- fix: macro error crate references [(#799)](https://github.com/andromedaprotocol/andromeda-core/pull/799)
- fix: Prevent duplicate relay in Kernel [(#802)](https://github.com/andromedaprotocol/andromeda-core/pull/802)
- fix: ibc username not working for ibc send with funds [(#814)](https://github.com/andromedaprotocol/andromeda-core/pull/814)
- fix: kernel's handle_local not resolving AndrAddr [(#846)](https://github.com/andromedaprotocol/andromeda-core/pull/846)
- fix: Timelock Release Funds [(#857)](https://github.com/andromedaprotocol/andromeda-core/pull/857)
- fix: Added missing version bumps [(#858)](https://github.com/andromedaprotocol/andromeda-core/pull/858)
- fix: GetComponents query's schema  [(#873)](https://github.com/andromedaprotocol/andromeda-core/pull/873)
- fix: Save App contract address during instantiation  [(#896)](https://github.com/andromedaprotocol/andromeda-core/pull/896)
- fix: Remove sale after Cancel Sale in Exchange  [(#913)](https://github.com/andromedaprotocol/andromeda-core/pull/913)
- fix: Kernel Send Msg vulnerability  [(#915)](https://github.com/andromedaprotocol/andromeda-core/pull/915)
- fix: Duplicate Redeem in Exchange  [(#919)](https://github.com/andromedaprotocol/andromeda-core/pull/919)
- fix: LP Denom Extraction in Osmosis Socket  [(#951)](https://github.com/andromedaprotocol/andromeda-core/pull/951)


## Release 4

### Added

- Crowdfund, added additional state [(#715)](https://github.com/andromedaprotocol/andromeda-core/pull/715)
- Added optional config for Send in Splitter contracts [(#686)](https://github.com/andromedaprotocol/andromeda-core/pull/686)
- Added CW20 suppport in Splitter contracts [(#703)](https://github.com/andromedaprotocol/andromeda-core/pull/703)
- Matrix ADO [(#539)](https://github.com/andromedaprotocol/andromeda-core/pull/539)
- Added Distance ADO [(#570)](https://github.com/andromedaprotocol/andromeda-core/pull/570)
- Rates: Handle cross-chain recipients [(#671)](https://github.com/andromedaprotocol/andromeda-core/pull/671)
- Permissions: Permissioned Actors in AndromedaQuery [(#717)](https://github.com/andromedaprotocol/andromeda-core/pull/717)
- Added Schema and Form ADOs [(#591)](https://github.com/andromedaprotocol/andromeda-core/pull/591)
- feat: kernel environment variables [#738](https://github.com/andromedaprotocol/andromeda-core/pull/738)
- Flat Rate denom can be a VFS path [(#727)](https://github.com/andromedaprotocol/andromeda-core/pull/727)
- Auction ADO: Added buy_now_price option in Update Auction [(#730)](https://github.com/andromedaprotocol/andromeda-core/pull/730)
- feat: added a query for pending packets to kernel [(#740)](https://github.com/andromedaprotocol/andromeda-core/pull/740)
- Kernel: added `verify_signature` function inside `signature.rs` for  [(#800)](https://github.com/andromedaprotocol/andromeda-core/pull/800)

### Changed

- Rates: Limit rates recipient to only one address [(#669)](https://github.com/andromedaprotocol/andromeda-core/pull/669)
- Address Validation: Cross-chain recipients don't need to be registered in VFS [(#725)](https://github.com/andromedaprotocol/andromeda-core/pull/725)
- Weighted Splitter: Replace Recipient with AndrAddr in RemoveRecipient and GetUserWeight [(#739)](https://github.com/andromedaprotocol/andromeda-core/pull/739)
- feat: IBC packet tracking adjustments [#748](https://github.com/andromedaprotocol/andromeda-core/pull/748)
- ADODB: Version Validation during Publish ensures different, not greater, version [(#752)](https://github.com/andromedaprotocol/andromeda-core/pull/752)
- ref: Rename Set Amount Splitter to Fixed Amount Splitter [(#754)](https://github.com/andromedaprotocol/andromeda-core/pull/754)
- Point ADO: remove Rates module from the contract[(#761)](https://github.com/andromedaprotocol/andromeda-core/pull/761)
- feat: SignedDecimal for Distance [(#774)](https://github.com/andromedaprotocol/andromeda-core/pull/774)
- feat: SignedDecimal for Point [(#779)](https://github.com/andromedaprotocol/andromeda-core/pull/779)
- feat: SignedDecimal for Graph [(#778)](https://github.com/andromedaprotocol/andromeda-core/pull/778)

### Fixed

- feat: alterations to kernel for IBC [(#726)](https://github.com/andromedaprotocol/andromeda-core/pull/726)
- Fixed handle_local amp message when a amp message is passed with custom config [(#729)](https://github.com/andromedaprotocol/andromeda-core/pull/729)
- Fixed wrong return attribute for SubDir Query [(#756)](https://github.com/andromedaprotocol/andromeda-core/pull/756)
- fix: Prevent bypassing splitter lock via config [(#757)](https://github.com/andromedaprotocol/andromeda-core/pull/757)
- Fixed Curve ADO to be able to update curve config after reset [(#762)](https://github.com/andromedaprotocol/andromeda-core/pull/762)
- Fixed Curve ADO's query error caused by Float data type [(#767)](https://github.com/andromedaprotocol/andromeda-core/pull/767)

## Release 3

### Added

- Added IBC Registry ADO [(#566)](https://github.com/andromedaprotocol/andromeda-core/pull/566)
- Added Denom Validation in IBC Registry ADO [(#571)](https://github.com/andromedaprotocol/andromeda-core/pull/571)
- Added Kernel ICS20 Transfer with Optional ExecuteMsg [(#577)](https://github.com/andromedaprotocol/andromeda-core/pull/577)
- Added IBC Denom Wrap/Unwrap [(#579)](https://github.com/andromedaprotocol/andromeda-core/pull/579)
- Added deployment script/CI workflow for OS [(#616)](https://github.com/andromedaprotocol/andromeda-core/pull/616)
- Added deployable interfaces to all ADOs [(#620)](https://github.com/andromedaprotocol/andromeda-core/pull/620)
- Added MultiSig ADO [(#619)](https://github.com/andromedaprotocol/andromeda-core/pull/619)
- Added Validator Staking ADO [(#330)](https://github.com/andromedaprotocol/andromeda-core/pull/330)
- Added Restake and Redelegate to Validator Staking [(#622)](https://github.com/andromedaprotocol/andromeda-core/pull/622)
- Added andromeda-math and andromeda-account packages[(#672)](https://github.com/andromedaprotocol/andromeda-core/pull/672)

### Changed

- Removed staking from vesting contract [(#554)](https://github.com/andromedaprotocol/andromeda-core/pull/554)
- Vesting: Changed to use Milliseconds instead of seconds and removed unnecessary is_multi_batch_enabled flag [(#578)](https://github.com/andromedaprotocol/andromeda-core/pull/578)
- Splits up ADOs: moved Counter, Curve, Date-Time, Graph, Point, Shunting ADOs to math package and Fixed Multisig ADO to accounts package[(#672)](https://github.com/andromedaprotocol/andromeda-core/pull/672)

### Fixed

- Vesting: Added validation to the instantiation process [(#583)](https://github.com/andromedaprotocol/andromeda-core/pull/583)
- Fix precision issue with vestings claim batch method [(#555)](https://github.com/andromedaprotocol/andromeda-core/pull/555)
- (validator-staking) fix: validator staking distribution message for andromeda chain [(#618)](https://github.com/andromedaprotocol/andromeda-core/pull/618)

### Removed

## v1.1

### Added

- Added `Asset` enum [(#415)](https://github.com/andromedaprotocol/andromeda-core/pull/415)
- Added `ADOBaseVersion` query to all ADOs [(#416)](https://github.com/andromedaprotocol/andromeda-core/pull/416)
- Staking: Added ability to remove/replace reward token [(#418)](https://github.com/andromedaprotocol/andromeda-core/pull/418)
- Added Expiry Enum [(#419)](https://github.com/andromedaprotocol/andromeda-core/pull/419)
- Added Conditional Splitter [(#441)](https://github.com/andromedaprotocol/andromeda-core/pull/441)
- Validator Staking: Added the option to set an amount while unstaking [(#458)](https://github.com/andromedaprotocol/andromeda-core/pull/458)
- Set Amount Splitter [(#507)](https://github.com/andromedaprotocol/andromeda-core/pull/507)
- Added String Storage ADO [(#512)](https://github.com/andromedaprotocol/andromeda-core/pull/512)
- Boolean Storage ADO [(#513)](https://github.com/andromedaprotocol/andromeda-core/pull/513)
- Added Counter ADO [(#514)](https://github.com/andromedaprotocol/andromeda-core/pull/514)
- Added Curve ADO [(#515)](https://github.com/andromedaprotocol/andromeda-core/pull/515)
- Added Date Time ADO [(#519)](https://github.com/andromedaprotocol/andromeda-core/pull/519)
- Added Graph ADO [(#526)](https://github.com/andromedaprotocol/andromeda-core/pull/526)
- Added Authorized CW721 Addresses to Marketplace [(#542)](https://github.com/andromedaprotocol/andromeda-core/pull/542)
- Added Denom Validation for Rates [(#568)](https://github.com/andromedaprotocol/andromeda-core/pull/568)
- Added BuyNow option for Auction [(#533)](https://github.com/andromedaprotocol/andromeda-core/pull/533)
- Include ADOBase Version in Schema [(#574)](https://github.com/andromedaprotocol/andromeda-core/pull/574)
- Added multi-hop support for IBC [(#604)](https://github.com/andromedaprotocol/andromeda-core/pull/604)
- Added astroport and osmosis socket [(#845)](https://github.com/andromedaprotocol/andromeda-core/pull/845)

### Changed

- Merkle Root: stage expiration now uses `Milliseconds`[(#417)](https://github.com/andromedaprotocol/andromeda-core/pull/417)
- Module Redesign [(#452)](https://github.com/andromedaprotocol/andromeda-core/pull/452)
- Primitive Improvements [(#476)](https://github.com/andromedaprotocol/andromeda-core/pull/476)
- Crowdfund Restructure [(#478)](https://github.com/andromedaprotocol/andromeda-core/pull/478)
- Conditional Splitter Internal Audit [(#479)](https://github.com/andromedaprotocol/andromeda-core/pull/479)
- Merkle Root: Andromeda Asset is used now as a `asset_info`[(#480)](https://github.com/andromedaprotocol/andromeda-core/pull/480)
- Reference Address List contract for Permission [(#481)](https://github.com/andromedaprotocol/andromeda-core/pull/481)
- Crowdfund Internal Audit [(#485)](https://github.com/andromedaprotocol/andromeda-core/pull/485)
- Auction: Minimum Raise [(#486)](https://github.com/andromedaprotocol/andromeda-core/pull/486)
- Made Some CampaignConfig Fields Optional [(#541)](https://github.com/andromedaprotocol/andromeda-core/pull/541)
- Permissioning: Allow multiple actors to be set and removed at once [(#548)](https://github.com/andromedaprotocol/andromeda-core/pull/548)
- Make Action Names in CW721 Conform to Standard [(#545)](https://github.com/andromedaprotocol/andromeda-core/pull/545)
- Timelock ADO: Replace MillisecondsExpiration with Expiry [(#550)](https://github.com/andromedaprotocol/andromeda-core/pull/550)
- Address List: Support for multiple actors while adding and removing permissions [(#556)](https://github.com/andromedaprotocol/andromeda-core/pull/556)
- ADODB now supports pre-release tagging [(#560)](https://github.com/andromedaprotocol/andromeda-core/pull/560)
- Validator Staking: Updated according to audit [(#565)](https://github.com/andromedaprotocol/andromeda-core/pull/565)
- Conditional Splitter: Change lock_time's type from MillisecondsDuration to Expiry [(#567)](https://github.com/andromedaprotocol/andromeda-core/pull/567)
- Permissions now have an optional start time [(#668)](https://github.com/andromedaprotocol/andromeda-core/pull/668)

### Fixed

- Splitter: avoid zero send messages, owner updates lock any time [(#457)](https://github.com/andromedaprotocol/andromeda-core/pull/457)
- Splitter and Conditional Splitter: fix lock time calculation [(#547)](https://github.com/andromedaprotocol/andromeda-core/pull/547)
- AMPPkt verify origin fix [(#552)](https://github.com/andromedaprotocol/andromeda-core/pull/552)
- Fix permissioning limited use consumptions and blacklist bypass scenario [(#553)](https://github.com/andromedaprotocol/andromeda-core/pull/553)
- Crowdfund: fixed error related to `QueryMsg::Tiers` message handler [(#563)](https://github.com/andromedaprotocol/andromeda-core/pull/563)
- Vesting: Recipient validation for VFS paths [(#641)](https://github.com/andromedaprotocol/andromeda-core/pull/641)

### Removed

- Schemas are no longer tracked [(#430)](https://github.com/andromedaprotocol/andromeda-core/pull/430)
