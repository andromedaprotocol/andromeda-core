# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Added Validator Staking ADO [(#330)](https://github.com/andromedaprotocol/andromeda-core/pull/330)
- Added `Asset` enum [(#415)](https://github.com/andromedaprotocol/andromeda-core/pull/415)
- Added `ADOBaseVersion` query to all ADOs [(#416)](https://github.com/andromedaprotocol/andromeda-core/pull/416)
- Staking: Added ability to remove/replace reward token [(#418)](https://github.com/andromedaprotocol/andromeda-core/pull/418)
- Added Expiry Enum [(#419)](https://github.com/andromedaprotocol/andromeda-core/pull/419)
- Added Conditional Splitter [(#441)](https://github.com/andromedaprotocol/andromeda-core/pull/441)
- Validator Staking: Added the option to set an amount while unstaking [(#458)](https://github.com/andromedaprotocol/andromeda-core/pull/458)
- Added Curve ADO [(#515)](https://github.com/andromedaprotocol/andromeda-core/pull/515)
- Set Amount Splitter [(#507)](https://github.com/andromedaprotocol/andromeda-core/pull/507)
- Added String Storage ADO [(#512)](https://github.com/andromedaprotocol/andromeda-core/pull/512)
- Boolean Storage ADO [(#513)](https://github.com/andromedaprotocol/andromeda-core/pull/513)
- Added Counter ADO [(#514)](https://github.com/andromedaprotocol/andromeda-core/pull/514)

### Changed

- Merkle Root: stage expiration now uses `Milliseconds`[(#417)](https://github.com/andromedaprotocol/andromeda-core/pull/417)
- Module Redesign [(#452)](https://github.com/andromedaprotocol/andromeda-core/pull/452)
- Primitive Improvements [(#476)](https://github.com/andromedaprotocol/andromeda-core/pull/476)
- Crowdfund Restructure  [(#478)](https://github.com/andromedaprotocol/andromeda-core/pull/478)
- Conditional Splitter Internal Audit [(#479)](https://github.com/andromedaprotocol/andromeda-core/pull/479)
- Merkle Root: Andromeda Asset is used now as a `asset_info`[(#480)](https://github.com/andromedaprotocol/andromeda-core/pull/480)
- Reference Address List contract for Permission  [(#481)](https://github.com/andromedaprotocol/andromeda-core/pull/481)
- Crowdfund Internal Audit [(#485)](https://github.com/andromedaprotocol/andromeda-core/pull/485)
- Auction: Minimum Raise [(#486)](https://github.com/andromedaprotocol/andromeda-core/pull/486)
- Version Bump [(#488)](https://github.com/andromedaprotocol/andromeda-core/pull/488)
- Made Some CampaignConfig Fields Optional [(#541)](https://github.com/andromedaprotocol/andromeda-core/pull/541)
- Make Action Names in CW721 Conform to Standard  [(#545)](https://github.com/andromedaprotocol/andromeda-core/pull/545)

### Fixed

- Splitter: avoid zero send messages, owner updates lock any time [(#457)](https://github.com/andromedaprotocol/andromeda-core/pull/457)

### Removed

- Schemas are no longer tracked [(#430)](https://github.com/andromedaprotocol/andromeda-core/pull/430)
