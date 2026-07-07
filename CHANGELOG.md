# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

-

## [0.3.1] - 2026-07-07

### Added

- @alexjercan add a post-processing camera component

### Changed

- @alexjercan upgrade to Bevy 0.19 (avian3d 0.7, bevy_rand 0.15, bevy_enhanced_input 0.26, rand 0.10) and migrate the source to the new API
- @alexjercan externalize `bevy_common_systems` as a git dependency and remove the vendored copy

### Docs

- @alexjercan add `AGENTS.md` and a `docs/` folder (architecture, scenario system, sections, development, Bevy 0.19 migration notes)

## [0.3.0] - 2025-11-29

### Added

- @alexjercan implement OnEnter and OnExit events
- @alexjercan implement torpedo bay section and blast damage
- @alexjercan implement new health system - add health for each spaceship section
- @alexjercan added new event trigger for entering a zone

### Changed

- @alexjercan improve directional shader to make the forward direction more visible
- @alexjercan improve the thruster shaders to allow more complex shapes and animations

## [0.2.1] - 2025-11-15

### Chores

- @alexjercan improve modding documentation and examples
- @alexjercan refactor event system for better performance and scalability

## [0.2.0] - 2025-11-08

### Added

- @alexjercan implement game events and a queue system that handles them
- @alexjercan scenario and modding capabilities added
- @alexjercan asteroids with procedural mesh and dynamic destruction for objects added

## [0.1.0] - 2025-10-21

### Added

- @alexjercan basic spaceship sections added
- @alexjercan editor and simulation scenes added

[unreleased]: https://github.com/alexjercan/nova-protocol/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/alexjercan/nova-protocol/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/alexjercan/nova-protocol/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/alexjercan/nova-protocol/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/alexjercan/nova-protocol/releases/tag/v0.1.0
