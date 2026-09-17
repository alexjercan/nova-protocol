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
- A shipped format with known consumers that changed shape. It needs
  `**(breaking)**` and a migration note. An unshipped format needs neither:
  delete compatibility code and remove its obsolete documentation.
- A required value hidden by a default, optional field, alias, or permissive
  parser. Invalid authoring must fail at lint or load.
- Portability, which CI catches slowly and you catch for free:
  - `std::time`, `std::thread`, and blocking IO compile for wasm32 and then
    panic in the browser. `ci/wasm-clippy/clippy.toml` holds the ban list.
  - Code reachable only under `--features debug`, and code left unused without
    it. The default-features job builds under `-D warnings`.
- Documentation the change invalidated: player behavior in `web/src/wiki/`,
  creator contracts in `web/src/create/`, developer mechanisms in `docs/`.
  Re-derive a claim from the code; do not only grep for the name.
- `CHANGELOG.md`: apply AGENTS.md's release-baseline and entry rules. Collapse
  pre-release revisions; omit bugs introduced and fixed inside the cycle.

## Verifying

```bash
nix develop --command cargo run content lint
nix develop --command cargo run content lint --target <mod>
```

Do not run `content gen`: it writes. To claim a generated file is stale, read
the builder and the RON, or report the claim as unverified.
