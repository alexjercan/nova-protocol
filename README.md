![Nova Protocol](assets/base/banner.png)

# Nova Protocol

A 3D space shooter built with [Bevy](https://bevyengine.org). Build modular
ships, fly them with real thrusters, and fight through asteroid fields and
gravity wells.

Project and gameplay documentation lives on the published site:

- [Project site](https://alexjercan.github.io/nova-protocol/)
- [Play in your browser](https://alexjercan.github.io/nova-protocol/play/)
- [Tutorial](https://alexjercan.github.io/nova-protocol/tutorial/)
- [Player and modding wiki](https://alexjercan.github.io/nova-protocol/wiki/)
- [Developer guide](https://alexjercan.github.io/nova-protocol/wiki/dev/development/)
- [Project tour](https://alexjercan.github.io/nova-protocol/wiki/dev/project-tour/)

## Quick start

The Nix development shell provides the pinned Rust toolchain, Bevy system
dependencies, Node.js, and Trunk.

```sh
git clone https://github.com/alexjercan/nova-protocol.git
cd nova-protocol
```

Without Nix, install the toolchain from `rust-toolchain.toml`, the required
Bevy system dependencies, and Trunk.

### Native

```sh
nix develop --command cargo run
nix develop --command cargo run --features dev  # debug tools
```

### Web

Run the complete local site, including the WASM game and mod portal:

```sh
nix develop --command scripts/serve-web.sh
```

The script prints the local URL and watches all three parts. To run only the
WASM game, use `nix develop --command trunk serve` and open
`http://localhost:8080/`.

See the [developer guide](https://alexjercan.github.io/nova-protocol/wiki/dev/development/)
for builds, tests, tools, examples, and platform setup.

## Repository guide

| Path | Purpose |
| --- | --- |
| [`AGENTS.md`](AGENTS.md) | Repository rules, architecture entry points, workflow, and required checks. |
| [`CONVENTIONS.md`](CONVENTIONS.md) | Rust module, API, Bevy scheduling, comment, and test conventions. |
| [`RELEASE.md`](RELEASE.md) | Short checklist for versioning, changelog updates, tagging, and publishing. |
| [`CHANGELOG.md`](CHANGELOG.md) | Unreleased and shipped user-visible changes. |
| [`Cargo.toml`](Cargo.toml) | Workspace members, examples, features, and the shared package version. |
| [`crates/`](crates/) | Game and tooling crates. `nova_core::AppBuilder` is the main assembly point. |
| [`src/`](src/) | Thin root executable and library entry points. |
| [`assets/`](assets/) | Shipped game content and assets. |
| [`examples/`](examples/) | Runnable player paths, system scenarios, UI flows, and probes. |
| [`web/`](web/) | Landing site, News, tutorial, and wiki sources. |
| [`scripts/`](scripts/) | Local web, content, screenshot, and packaging helpers. |

## License

See [`LICENSE`](LICENSE).
