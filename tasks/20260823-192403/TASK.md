# Get master green and ship v0.11.0

- STATUS: IN_PROGRESS
- PRIORITY: 100
- TAGS: v0.11.0,ci,release

## Goal

Get `master` green again, then release v0.11.0. The release checklist opens with
"confirm master is green", so the CI repair is the gate on everything else.

## Why CI broke

Two independent causes, both landing right after the v0.10.0 tag (2026-08-13,
the last green run):

1. **Toolchain drift.** `rust-toolchain.toml` said `channel = "nightly"`, so CI
   installed whatever nightly was current that morning. Nightly 1.100.0 changed
   method resolution, and `bevy_render` calls `.run_if(..)` unqualified where
   both `IntoScheduleConfigs::run_if` and the preludeed
   `ObserverSystemExt::run_if` are now applicable candidates - E0034, 8 errors,
   inside the dependency. It killed all three build jobs, so no job ever reached
   our own code and the lints below stayed hidden behind it.

   Not fixable from bevy: `ObserverSystemExt` is in the prelude of bevy_ecs
   0.19.0 *and* 0.19.1, and bevy_render 0.19.1 has the same unqualified call
   sites. The toolchain is the thing to pin.

2. **Real lints of ours**, which only became visible once the dependency built:
   `doc_lazy_continuation` in `nova_gameplay`, dead code in `nova_ship`, and two
   `--features debug` leaks in examples that the default-features job exists to
   catch.

## Direction

- Pin the nightly explicitly, in `rust-toolchain.toml` and `flake.nix` together,
  so the devshell and CI are the same compiler. `nightly.latest` in the flake
  floated with the rust-overlay input: `nix flake update` could move local off
  the CI toolchain without touching a tracked file.
- Fix the lints properly rather than suppressing them. The doc lints are prose
  that wrapped onto a `+ ` / `- ` line start and so read as Markdown list
  markers; rewrap instead of `#[expect]`.
- Revisit the pin once nightly or bevy resolves the ambiguity. The pin is the
  fix for today, not a permanent freeze.

## Done when

- All four CI jobs pass on `master`.
- `cargo fmt --check`, the debug clippy pass, the default-features check, and the
  wasm clippy pass are green locally on the pinned toolchain.
- v0.11.0 is tagged and released per `RELEASE.md`, with the changelog, News, and
  docs current.
