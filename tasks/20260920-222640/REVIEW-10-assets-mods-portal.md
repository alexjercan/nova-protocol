# Review 10: Assets, mods, and portal

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Specialist: content tree and format follow-up
- Verdict: no findings

## Adjudication

No BLOCKER, MAJOR, or MINOR finding survived.

The pair checked and cleared bundle dependency ordering, base ownership, overlays and duplicate IDs, generic asset-reference rewriting, bare-reference lint, portal schema and size gates, URL/path containment, staged installs, cache corruption recovery, safe mode, reload scheduling, native/web persistence intent, and wasm cfg boundaries.

The content specialist confirmed that real webmods exercise `self://` and `dep://` across section, ship, and scenario fields; the generic serde walk covers every content variant; the portal generator independently enforces matching rules; shipped/downloaded ID shadowing is refused at all relevant boundaries; and intentional native/web key differences are pinned by tests.

## Coverage

Fully read every Rust source and test file in `nova_assets`, `nova_modding`, and `nova_mod_format`, including all 13 `nova_assets` integration tests and crate manifests. The specialist also read the portal generator, bundle/catalog manifests, and representative real webmod reference shapes.

Not checked: no Cargo command, content lint execution, content generation, game, probe, browser runtime, wasm build, workspace test, or Clippy run. Wasm IndexedDB and browser URL behavior received static source review only. The unmeasured linear overlay merge was not raised because merge is infrequent and no real cost was established.
