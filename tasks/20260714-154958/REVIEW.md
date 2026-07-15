# Review: Assets + licenses shipping hygiene (20260714-154958)

Branch: `chore/assets-licenses-hygiene`

## Verdict: APPROVE

A docs/file-relocation chore; no code or tests. Reviewed for correctness and
completeness against the task's two asks + the folded-in docs refresh.

## What was done vs the task

1. **Don't ship source assets** - `assets/blender/*.blend` (3 files, 2.7M) moved
   to top-level `art/blender/` via `git mv`. Both shipped builds bundle the
   whole `assets/` tree (web: Trunk `copy-dir assets`; native: `release.yaml`
   `cp -r assets/` / `tar ... assets`), so the relocation removes the payload
   from BOTH with zero build-config change. `assets/` verified `.blend`-free;
   `git grep assets/blender` finds no lingering tracked reference; no `build.rs`
   or Rust code loads `.blend`. Added `art/README.md` documenting the split.

2. **Consolidate credits/licenses** - credits were ALREADY single-source
   (`credits/` canonical, copied to `dist/credits/` which is gitignored, so no
   hand-maintained artifact; no top-level `CREDITS.md` remains). Expanded
   `credits/CREDITS.md` into an explicit audit: per the owner, all game assets
   are original except the already-credited Bevy icon. Documented the source of
   truth and shipping path. Rust crate-license aggregation (cargo-about) called
   out as a follow-up (see below).

3. **Docs refresh (folded in)** - `assets/sounds/README.md` required-files table
   rewritten from the stale 5 originals to all 16 cues, split into positional
   combat and non-positional UI groups, pointing at `NOVA_SFX_FILES` /
   `every_nova_sfx_key_has_a_file` as the source of truth. `docs/architecture.md`
   updated for the moved sources + the runtime-only `assets/` rule.

## Notes / follow-ups

- A full `trunk build` was not run (cold wasm build is expensive). The shipping
  fix is verified by construction: `copy-dir` copies `assets/` as-is, and
  `assets/` now contains no `.blend`. Low risk for a pure relocation.
- Local-only leftover: the MAIN checkout still has untracked (gitignored)
  `assets/blender/*.blend1` autosave backups on disk; removed post-landing so
  a local Trunk build is clean. CI checkouts are already clean.
- New follow-up filed: aggregate Rust dependency licenses (Bevy/avian MIT +
  Apache-2.0) into the shipped credits (cargo-about) - a real binary-distribution
  obligation deferred from the asset-only audit.
