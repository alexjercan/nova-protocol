# Review: Persist EnabledMods across restarts (native file + wasm localStorage)

- TASK: 20260714-174131
- BRANCH: modding/mod-persist

## Round 1

- VERDICT: APPROVE

Reviewed the persistence diff (nova_assets `mod_prefs` module; `load_enabled_mods`/
`save_enabled_mods` systems; the `seed_enabled_mods` union change; target-gated `dirs`/
`web-sys` deps). Self pass plus an independent out-of-context adversarial pass focused on
the wasm backend (unverifiable locally). Independently verified the highest-risk part:

- The wasm `web-sys 0.3` API usage against the crate source: `window() -> Option<Window>`
  (feature `Window`), `Window::local_storage() -> Result<Option<Storage>, JsValue>`
  (feature `Storage`), `Storage::get_item/set_item` - the `?`/`.ok()?` chains type-check
  exactly (`storage()` yields `Option<Storage>`; `load` yields `String`), and the enabled
  `Window`+`Storage` features are sufficient. Not behind `web_sys_unstable_apis`.
- Native chain proven IN-GAME with a temp `XDG_CONFIG_HOME`: run 1 (no file) wrote
  `["base"]`; run 2 honored a pre-written `["base","demo"]` and preserved demo (if load
  were ignored, seed would have reset it to base-only and save overwritten it). Both runs
  clean, no panic.

The out-of-context reviewer confirmed (no blockers/majors): the seed union keeps base on
while preserving the restored set; `load -> seed -> register_bundles` are `.chain()`ed so
ordering holds; `save_enabled_mods` takes `Res` (immutable) so no infinite write loop, and
its `not(in_state(Loading))` + `resource_exists::<GameAssets>` gates prevent a startup
clobber; `load` only sets on `Some` so no-saved-file startup is unchanged; native IO has
no panic path; the 174120 merge tests are unaffected (they never call seed).

- [x] R1.1 (MINOR) mod_prefs.rs - the comment claimed "CI/Trunk builds" the wasm path,
  but the wasm build (`deploy-page.yaml`) is `workflow_dispatch` only; automated PR/master
  CI (`ci.yaml`) does NOT compile it. Fixed the comment to state static review (against the
  web-sys API) is the real guard for the wasm path, since neither the local runner nor
  automated CI builds it.
- [x] R1.2 (NIT) demo_scenario.rs - the `from_none` seed case used the same input
  (`["demo"]`) as `from_demo`, so it was redundant. Removed it; folded its intent (base
  forced on from a base-less set) into the `from_demo` case's comment/assertions.

Verification: nova_assets 23 unit (incl. 3 mod_prefs) + demo_scenario 7 pass;
`cargo test --workspace --no-run` green; fmt clean; menu example clean.

No BLOCKER/MAJOR. Ships.
