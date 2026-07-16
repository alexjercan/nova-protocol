# Review: Decouple portal/publish tests from specific mods

- TASK: 20260716-155839
- BRANCH: refactor/portal-tests-synthetic

## Round 1

- VERDICT: APPROVE

Verified independently:

- The inherited-master-red claim is real: `git show master:...` carries
  both `contains_key("demo")` guards while master's base.bundle.ron has
  zero demo entries - mod_cache_install has been failing on master since
  the demo removal landed. This branch repairs it (guards pin
  shakedown_run); land without delay.
- The generic publish assertion cannot pass vacuously: it asserts set
  EQUALITY between webmods/ directories and published ids plus a
  non-empty delivery guard, so a dropped mod, a phantom entry, or an
  empty source all fail.
- The wire e2e keeps every production stage it had (real generator with
  the real shipped-catalog collision gate, real tiny_http + transport,
  real loaders via mods://, real merge on enable/uninstall); the only
  coverage narrowed is byte-identity of REAL webmods files after
  install, which is content-agnostic and remains proven on fixture
  bytes.
- Scripted swaps were count-asserted per edit; the final gate
  (`grep -ri gauntlet` over crates/src/examples) is empty outside
  webmods/, and `grep -rn 'contains_key("demo")'` is empty repo-wide.
- Suites re-run green: portal_install 9/9, mod_cache_install 7/7,
  nova_portal_gen 12/12, nova_mod_format 9/9, nova_menu 46/46 (full),
  check --all-targets and fmt clean.
- The fixture mod-id charset discovery (portal ids: lowercase/digits/'-'
  only) is recorded in the close notes; the fixture uses
  "fixture-slalom" + scenario "fixture_slalom_run" accordingly.

No findings.
