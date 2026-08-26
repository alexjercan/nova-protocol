# Lane: contracts and portability

Judge what the change breaks outside the code that compiles it: identifiers,
formats, targets, and the documents that describe them.

Read `docs/keeping-docs-in-sync.md` for the documentation map.

## Look for

- A renamed runtime identifier - prototype, scenario, style, asset, section -
  with a consumer left behind. These are strings and the compiler does not check
  them. Grep every rename across `assets/`, `webmods/`, `examples/`, `web/`,
  `docs/`, and `crates/`.
- A hand-edited generated file. `assets/base/**/*.content.ron` comes from the
  Rust builders through `content gen`, and `content_ron_parity` goes red.
- A shipped format that changed shape - scenario, mod, save, bundle. It needs
  `**(breaking)**` and a migration note. A format that never shipped needs
  neither: remove its documentation instead of writing a migration for it.
- Portability, which CI catches slowly and you catch for free:
  - `std::time`, `std::thread`, and blocking IO compile for wasm32 and then
    panic in the browser. `ci/wasm-clippy/clippy.toml` holds the ban list.
  - Code reachable only under `--features debug`, and code left unused without
    it. The default-features job builds under `-D warnings`.
- Documentation the change invalidated: player behavior in `web/src/wiki/`,
  creator contracts in `web/src/create/`, developer mechanisms in `docs/`.
  Re-derive a claim from the code; do not only grep for the name.
- `CHANGELOG.md`: one entry per released user-visible change, at most 200
  characters once wrapped lines are joined, grouped by subsystem, measured
  against the last RELEASE rather than the last commit. A bug introduced and
  fixed inside this cycle gets no entry. A revision of an unreleased change is
  collapsed into its entry, not added beside it.

## Verifying

```bash
nix develop --command cargo run content lint
nix develop --command cargo run content lint --target <mod>
```

Do not run `content gen`: it writes. To claim a generated file is stale, read
the builder and the RON, or report the claim as unverified.
