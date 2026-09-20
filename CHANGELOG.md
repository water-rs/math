# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/water-rs/math/compare/v0.1.1...v0.2.0) - 2026-09-20

### Added

- *(math)* speak the formula instead of announcing its MathML
- *(text)* share one font collection through the environment
- *(graphics)* [**breaking**] give SceneContent an intrinsic size
- *(math)* add waterui-math, formula rendering on the OpenType MATH table

### Fixed

- *(ci)* nest workflow_run types under the trigger so release.yml parses
- *(math)* publish the formula's MathML on its accessibility node
- *(math)* redraw a bound formula when its source changes
- *(math)* satisfy the CI gates the local run did not reach

### Other

- consume the unpublished dev-dependencies by git revision ([#16](https://github.com/water-rs/math/pull/16))
- publish on the main push, not under workflow_run ([#14](https://github.com/water-rs/math/pull/14))
- adopt the 0.5.0 framework wave (0.2.0) ([#11](https://github.com/water-rs/math/pull/11))
- lock gpu-allocator to windows 0.62.2 so wgpu-hal builds on Windows
- adopt the 0.5.0 framework wave
- let the PR source gate accept release-plz release branches ([#9](https://github.com/water-rs/math/pull/9))
- disable incremental builds and trim debuginfo ([#6](https://github.com/water-rs/math/pull/6))
- run tests with cargo nextest ([#5](https://github.com/water-rs/math/pull/5))
- [**breaking**] rename waterui-str dependency to suiteki
- install dxc on the Windows test leg
- stand waterui-math on its own workspace
- release
- Merge branch 'dev' into agent/scene-invalidator-watch-274/20260906-024222
- export visual-review images under the shared artifact root
- *(math)* add doctests for the public entry points
- *(math)* paint an opaque ground under the gallery renderings
- *(math)* render a formula gallery on both scene engines

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
