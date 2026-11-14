# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/water-rs/math/compare/v0.2.0...v0.3.0) - 2026-11-14

### Added

- [**breaking**] draw formulas through the waterui Canvas
- port to waterui render-target-neutral recording contract ([#25](https://github.com/water-rs/math/pull/25))
- [**breaking**] port to the Cherenkov scene contract
- support math-alphabet commands through layout and speech

### Fixed

- read math faces through FontCollection::faces
- *(scene)* implement SceneContent::rebuild_for_engine for the 1406 contract
- *(ci)* pin the toolchain without disarming the MSRV gate ([#19](https://github.com/water-rs/math/pull/19))

### Other

- *(deps)* point migrated Batch 2 git dependencies to main ([#47](https://github.com/water-rs/math/pull/47))
- skip cargo-semver-checks on waterui-math
- install libfontconfig1-dev in release-pr job ([#42](https://github.com/water-rs/math/pull/42))
- release from main and retire dev ([#40](https://github.com/water-rs/math/pull/40))
- restore the Hydrolysis test fonts from the Actions cache ([#39](https://github.com/water-rs/math/pull/39))
- *(deps)* point migrated Batch 1 git dependencies to main ([#36](https://github.com/water-rs/math/pull/36)) ([#37](https://github.com/water-rs/math/pull/37))
- format the canvas migration
- raise rust-version to 1.99, the framework's floor
- consume the framework from its dev branch
- lock the framework at its rebased nami 0.13 head
- [**breaking**] move to nami 0.13 and the migrated framework
- pass the release pull request's gate without CI
- [**breaking**] move the patch table to nami 0.12
- require nami 0.12
- *(deps)* pin waterui to dev at 46552dd30 ([#26](https://github.com/water-rs/math/pull/26))
- install Hydrolysis test fonts before tests
- *(deps)* repin water-rs/waterui to 1e359bca
- *(deps)* repin the engine and framework onto in-tree Cherenkov
- Merge branch 'dev' into feat/cherenkov-cutover
- *(deps)* pin hydrolysis, hydrolysis-m3 and video-gpu at the repinned heads
- pin cherenkov to 52236561
- install SPIRV-Tools for cherenkov-gpu's shader build on non-Apple legs
- install STIX Two Math from the upstream release
- repin the vello fork to the glifo canonical-location fix
- consume the 0.5.0 framework wave from crates.io ([#18](https://github.com/water-rs/math/pull/18))

## [0.1.0](https://github.com/water-rs/waterui/releases/tag/waterui-math-v0.1.0) - 2026-09-11

### Added

- *(math)* speak the formula instead of announcing its MathML
- *(text)* share one font collection through the environment
- *(graphics)* [**breaking**] give SceneContent an intrinsic size
- *(math)* add waterui-math, formula rendering on the OpenType MATH table

### Fixed

- *(math)* publish the formula's MathML on its accessibility node
- *(math)* redraw a bound formula when its source changes
- *(math)* satisfy the CI gates the local run did not reach

### Other

- Merge branch 'dev' into agent/scene-invalidator-watch-274/20260906-024222
- export visual-review images under the shared artifact root
- *(math)* add doctests for the public entry points
- *(math)* paint an opaque ground under the gallery renderings
- *(math)* render a formula gallery on both scene engines
