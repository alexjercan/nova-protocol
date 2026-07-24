# Goal: Scenarios tab collapsible campaign headers + real campaign->scenario mapping

- DATE: 20260724
- UMBRELLA TASK: 20260724-193016
- LANDING SCOPE: squash-merge each task to local `master` via `sprout land`; no push (user's call at Finish).

## Goal

The Scenarios picker (`nova_menu`) currently shows campaign scenarios as a flat
list with an inline position prefix ("Nova Protocol 1 - Shakedown Run"), shipped
by the campaign-grouping run (umbrella 20260723-093914). That was explicitly the
interim style. This run supersedes it with:

1. Collapsible campaign HEADERS in the picker - one header row per campaign that
   expands/collapses to reveal its ordered scenarios, so a player browses a
   campaign as a unit and can jump straight to any chapter for replay.
2. A real campaign->scenario MAPPING in the content/mod model, so a campaign's
   full ordered membership is first-class and known - including scenarios that
   are `hidden` from the flat picker but should still be reachable/listed under
   their campaign header for replay - instead of being reconstructed from
   per-scenario display metadata.

A player can browse the base "Nova Protocol" storyline as a collapsible group and
launch any of its chapters directly, including hidden mid-story chapters, without
replaying the whole arc.

## Done means

(Refined from task 20260723-095951 DoD; each item names its proof.)

1. A campaign's ordered membership (including replayable hidden chapters) is known
   from a real mapping in the content model, not from display-name parsing.
   (test: mapping resolves a campaign's members in order, hidden included)
2. The generated base content regenerates cleanly with the new mapping and parity
   holds. (cmd: `cargo run -p nova_assets --bin content -- gen` leaves a clean
   tree; cmd: `cargo test -p nova_assets`)
3. The Scenarios tab shows each campaign as a collapsible header grouping its
   ordered scenarios; expanding/collapsing works. (test: nova_menu widget-tree
   test of header + expand/collapse; manual: user browses a campaign, expands
   and collapses it)
4. A player can launch any listed scenario of a campaign directly for replay,
   including a hidden mid-campaign chapter. (test: launch path resolves a hidden
   member; manual: user replays a mid-campaign scenario without the earlier ones)
5. The New Game first-listed-scenario fallback and selection-repair stay
   deterministic under the new grouped/collapsible model. (cmd: `cargo test -p
   nova_menu`)
6. A decision record fixes the mapping representation (first-class Campaign
   content entity vs derived-from-metadata) and the hidden-scenario replay policy.
   (artifact: DECISION.md in the mapping task folder)

Overall: `cargo check` clean, `cargo fmt --check` clean, newly written/touched
tests pass, generated content parity holds, and the picker visibly reads as
collapsible campaign groups.

## Tasks

Updated as tasks land (one line per land).

(to be filled at the plan gate)

## Decisions (load-bearing, architectural)

(to be filled at the plan gate)

## Manual acceptance (batched for the user at Finish)

- (pending) collapsible campaign header browse + expand/collapse in the Scenarios tab
- (pending) replay a hidden mid-campaign chapter directly from its campaign header
