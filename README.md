![Nova Protocol](assets/banner.png)

# Nova Protocol

A 3D space shooter built with [Bevy](https://bevyengine.org). Build ships out of
modular sections, then fly them through asteroid fields and gravity wells with a
diegetic autopilot that flies on real thrusters.

> **[Play in your browser](https://alexjercan.github.io/nova-protocol/)** - or
> read the [tutorial](https://alexjercan.github.io/nova-protocol/tutorial/) and
> [wiki](https://alexjercan.github.io/nova-protocol/wiki/) first.

## What you do

- **Build a ship** out of modular sections - hull, controller, thruster, turret,
  torpedo bay - each with its own mass and health. Lose a section, lose that
  capability.
- **Fly it for real.** The `GOTO`, `ORBIT` and `STOP` autopilot verbs drive the
  ship's actual controller and thrusters: you watch the hull swing and the plume
  light up. Any manual input takes back control instantly.
- **Work the gravity.** Large asteroids carry inverse-square gravity wells;
  `ORBIT` parks you in a stable circular orbit around the dominant one.
- **Fight.** An angular aim-assist cone locks the nearest hostile; turrets
  compute intercept lead and torpedo bays fire guided, blast-damage warheads,
  all sold with camera shake, hit rings and positional audio.

## Controls (quick reference)

| Action | Key |
| --- | --- |
| Main thruster burn | `W` / `Space` (or RT) |
| GOTO the current lock | `G` |
| ORBIT the gravity well | `O` |
| STOP (retrograde burn to rest) | `X` |
| Cancel autopilot | `Z` |
| Fire turrets | Left mouse |
| Cycle HUD (All / Minimal / None) | `` ` `` |
| Pause | `Esc` |

Full controls are on the
[tutorial page](https://alexjercan.github.io/nova-protocol/tutorial/).

## Build and run

The game is a Rust/Bevy workspace on the **nightly** toolchain. On NixOS,
`nix develop` provides the dev shell with the system libs (udev, alsa, vulkan,
X11/wayland, trunk).

```sh
cargo run                       # run the game (boots into the main menu)
cargo run --features dev        # dev build (inspector, wireframe, debug tooling)
cargo run --example scenario # run one of the examples in examples/
trunk serve                     # web build, served on :8080
cargo build --release           # release profile
```

See [`AGENTS.md`](AGENTS.md) and [`docs/`](docs/) for the architecture, the
scenario system, and the section model.

## The landing site (`web/`)

The marketing/content site (landing page, blog, tutorial, wiki) lives in
[`web/`](web/) as a self-contained TypeScript + Webpack + Tailwind project,
separate from the Rust workspace. It fronts the game: the deploy publishes the
site at the root and the WASM game under `/play/`.

```sh
cd web
npm install
npm run serve   # dev server on :8090 (site only - see the note below)
npm run build   # static bundle in web/dist/
npm run ci      # format check + lint + build
```

**The game is a separate build.** `npm run serve` on its own serves only the
content site, so the **Play** button (and `/play/`) falls back to the landing
page. There are two ways to get a working Play locally:

*Live dev (hot reload on both):* run the game and the site side by side. The dev
server proxies `/play/` to `trunk serve`, so edits to either reload:

```sh
# terminal 1 - the game on :8080
nix develop -c trunk serve
# terminal 2 - the site on :8090, /play proxied to :8080
cd web && npm run serve
# open http://localhost:8090/ and click Play
```

(Override the proxy target with `GAME_DEV_URL` if trunk runs elsewhere.)

*One-shot preview (closest to the deploy):* build both and serve the combined
static output. Run inside the dev shell so both `trunk` and `node` are on PATH:

```sh
nix develop -c scripts/preview-web.sh            # debug game build
nix develop -c scripts/preview-web.sh --release  # optimized game build
# open http://localhost:8090/ and click Play
```

The `deploy-github-page` workflow
([`.github/workflows/deploy-page.yaml`](.github/workflows/deploy-page.yaml))
does the same assembly in CI: the webpack site at `/nova-protocol/` and the
Trunk game at `/nova-protocol/play/`.

## Project layout

Cargo workspace; the root crate is thin wiring and the real code lives in
`crates/`:

| Crate | Responsibility |
| --- | --- |
| `nova_core` | `AppBuilder` - assembles all plugins |
| `nova_editor` | The ship editor scene |
| `nova_gameplay` | Ship sections, integrity, input (player + AI), HUD, camera |
| `nova_scenario` | Scenario/modding engine (actions, events, filters, world, loader) |
| `nova_menu` | Main menu + pause menu |
| `nova_events` | Game event kinds and entity id/type components |
| `nova_assets` | Asset loading; registers sections + scenarios |
| `nova_debug` | Debug-only plugin (inspector, wireframe), behind the `debug` feature |
| `nova_info` | Exposes `APP_VERSION` |
| `web` | The landing/content site (TypeScript + Webpack + Tailwind) |

## License

See [`LICENSE`](LICENSE).
