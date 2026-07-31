# Review: KISS pass on the NOVA OS drawer HUD surfaces

- TASK: 20260731-170322
- BRANCH: refactor/kiss-nova-os-hud

Worktree: `/home/alex/.cache/sprouts/nova-protocol/refactor/kiss-nova-os-hud`

## Round 1

- REVIEWER: primary agent (in-session)
- VERDICT: REQUEST_CHANGES

Reviewed at `d0932acc`.

**On the reviewer.** The skill asks for an
out-of-context reviewer on round 1. This session runs under a standing user
directive not to invoke the Agent tool unless the user asks, which overrides
the skill default, so the review was done in-session. The diff is NOT trivial
(14.5k lines relocated), so this is a recorded exception, not the trivial-diff
carve-out. Mitigation: every load-bearing claim below was re-derived
mechanically from the tree rather than taken from the implementation
narrative.

### Independent re-derivation of the central claim

The task's whole risk is that slicing by line range silently drops or
duplicates code. Verified four ways against `master`, not by reading the
close-out:

1. Multiset of every `fn|struct|enum|const|type|trait <name>` token, before vs
   after, per file group: identical for all three (`comm`/`diff` clean). A
   dropped or duplicated item would show.
2. `NovaOsPlugin`, `NovaOsShipPlugin` and `NovaOsMapPlugin` `build` bodies,
   whitespace-stripped: byte-identical to `master`. System sets, ordering,
   run conditions and observers are unchanged.
3. Whitespace- and comment-stripped byte counts moved only upward
   (+6565 / +1722 / +1307), consistent with `pub(crate)` prefixes and the
   per-child import blocks. No shrinkage anywhere.
4. Test names listed via `--list` and diffed against `master`: identical set
   of 102. Spot-checked the three assertion fragments that a naive line diff
   flagged (`80/100 HP`, `[critical]`, `THRUSTER`) - all intact, merely
   rejoined by rustfmt after the test module lost 4 spaces of indentation.

Conclusion: the relocation is content-preserving. DoD 6 (owner skim) is the
only behavioral check left, and it is a user check, not a blocker.

### Checks rerun

| Check | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | green, 0 warnings |
| `cargo fmt --check` | clean |
| `cargo test -p nova_gameplay --lib hud::nova_os` | 102 passed, 0 failed |
| HUID grep over the scope | no hits |
| largest file | 1334 lines (`nova_os_ship/tests.rs`) |

Clippy was not run: standing user directive to skip local clippy, CI covers it.

## Findings

### MAJOR - stale `super::` doc paths after the move

`crates/nova_gameplay/src/hud/nova_os/style.rs:192` and
`crates/nova_gameplay/src/hud/nova_os/sound.rs:12`

Moving these items one module deeper made their relative doc paths point at
the wrong module. `DRAWER_EXEMPT_Z` documents itself as read by widgets
tagging themselves `[`super::HudNovaOsExempt`]`; `super` used to be `hud`, it
is now `hud::nova_os`, so the link is wrong and unresolved. `NovaOsBedSfx`
has the mirror problem with `[`super::super::audio`]`, which used to resolve
and now points one level short.

These are not pre-existing: both resolved on `master`. The move silently
re-pointed them, which is exactly the class of error a moves-only pass must
not leave behind.

Change: re-anchor to the new depth (`super::super::HudNovaOsExempt`,
`crate::audio`), and grep the moved files for any other `super::`-relative doc
path or code path whose depth changed.

### MINOR - the move added 30 rustdoc warnings

`cargo doc -p nova_gameplay --no-deps --document-private-items` emits 35
warnings under `hud/nova_os*` on this branch against 5 on `master`. Two
causes:

- **16** are the new module-layout tables in the three `mod.rs` files
  (`nova_os/mod.rs:37-46`, `nova_os_map/mod.rs:25-27`,
  `nova_os_ship/mod.rs:35-37`), which use intra-doc links `[`style`]` to
  modules that are deliberately private: "public documentation for `nova_os`
  links to private item `style`".
- **14** are doc links that used to resolve within one file and now cross a
  private module boundary - `[`sync_nova_os_app_ui`]` (x3),
  `[`animate_nova_os_crt`]` (x2), `[`position_nova_os_block_caret`]`,
  `[`nova_os_crt_screen_to_image_uv`]`, `[`drive_nova_os_power_led`]`,
  `[`MapContacts`]`, `[`MapOrbit`]`, `[`ShipSectionCommand`]` and friends.

A pass whose stated goal is a quieter surface should not hand back thirty new
warnings.

Change: in the module-layout tables use plain code spans, not links, since the
modules are private by design. For the cross-module links, either qualify with
the sibling path (the referring docs sit on `pub(crate)` items, so a qualified
path resolves without tripping the private-item lint) or demote to code spans.

### Observations (no change requested)

- Uniform `pub(crate)` on every moved item is broader than strictly needed;
  some are used by exactly one sibling. Acceptable for a moves-only pass - the
  per-item-minimal alternative is a lot of churn - and the mod.rs re-export
  lists keep the real surface legible.
- `use super::{a::*, b::*, ...}` glob imports in each child obscure which
  sibling a name comes from. Blunt, but it is what keeps this diff a pure move;
  worth revisiting if these modules are edited substantively later.
- `spawn.rs` (718) arguably holds two concerns: RTT/monitor setup and the
  header/main/footer region builders. Cohesive enough as "what the shell
  spawns", and well under the 1500 line bar.
- NOTES.md's kept/cut judgements were checked, including the claim that
  `examples/ui/nova_os_terminal_poc.html` is still checked in (it is), which
  justifies keeping the `PoC .case` references.

### Verdict for this round

REQUEST_CHANGES - one open MAJOR (stale `super::` doc paths).

Pending user checks:

- DoD 6, `manual:` - owner skims the diff and agrees no behavior changed. Does
  not block APPROVE; the four mechanical checks above are the supporting
  evidence.

## Responses to the round-1 findings

### MAJOR - stale `super::` doc paths - FIXED

`style.rs:192` now reads `[`super::super::HudNovaOsExempt`]` and `sound.rs:12`
now reads `[`crate::audio`]`. Audited the rest of the moved tree for the same
class: `grep -rn '\[`super::' nova_os nova_os_ship nova_os_map` leaves only
`nova_os/mod.rs`'s two links, and `mod.rs` sits at the depth the old
`nova_os.rs` did, so both are correct. No code-level relative path changed
depth (the compiler would have caught it; `cargo check` is green).

### MINOR - 30 new rustdoc warnings - FIXED

The module-layout tables use plain code spans now, since the modules they name
are private by design. The fourteen links the move broke across a private
module boundary are either qualified with the sibling path
(`[`super::shell::sync_nova_os_app_ui`]`, `[`super::crt::animate_nova_os_crt`]`
and so on - these sit on `pub(crate)` items, so a qualified path resolves
without tripping the private-item lint) or demoted to code spans where the item
is not reachable.

`cargo doc -p nova_gameplay --no-deps --document-private-items` now emits **3**
warnings under `hud/nova_os*`, against **5** on `master`. All three are
pre-existing (`MapOrbit`, `assign_map_contact_codes`, `assign_section_codes`);
two of master's five disappeared because the items they name became
`pub(crate)`, so their docs are no longer public documentation. Net: the branch
is quieter than the base.

## Round 2

- REVIEWER: primary agent (in-session)
- VERDICT: APPROVE

Reviewed at `0504cfb4`. Re-ran every check after the fixes:

| Check | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | green, 0 warnings |
| `cargo fmt --check` | clean |
| `cargo test -p nova_gameplay --lib hud::nova_os` | 102 passed, 0 failed |
| HUID grep over the scope | no hits |
| `cargo doc` warnings under the scope | 3, vs 5 on master |
| largest file | 1334 lines |

The round-1 fixes touch doc comments only - no code line changed, so the
content-preservation evidence from round 1 still holds. No fix regressions
found, and no new findings.

### Verdict for this round

APPROVE - both findings fixed and re-verified.

Pending user checks:

- DoD 6, `manual:` - owner skims the diff and agrees no behavior changed. Does
  not block APPROVE. Supporting evidence: item multisets identical to master,
  all three plugin `build` bodies byte-identical, and the same 102 tests
  passing by name.
