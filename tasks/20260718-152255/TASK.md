# Spike: can nova_meta_gen become a Python build-time hook, or must it stay Rust (Bevy .meta coupling)? Decide and port-or-document

- STATUS: CLOSED
- PRIORITY: 34
- TAGS: v0.8.0, tooling, spike, web

## Story

As the project owner consolidating tooling, I want a grounded decision on
whether the `.meta` sidecar generator can follow the portal generator to
Python, so that the tooling map has one answer instead of an open question -
and so we do not trade a working Rust tool for a Python script that silently
drifts from Bevy.

The user floated converting the `.meta` sidecar generator to a Python
build-time script (a Trunk post_build hook) like the portal port. But
`nova_meta_gen` is NOT engine-free the way the portal is: it boots a headless
(GPU-free) Bevy app and uses Bevy's `AssetServer` to produce the exact default
`.meta` content for each asset type, so `AssetMetaCheck::Always` works on the
web. A Python rewrite would have to hardcode Bevy's default meta format and
would silently drift when Bevy changes it. So this is a spike: decide
port-vs-keep, don't blindly port.

## Steps

- [x] Characterize what the current tool actually emits: dump the `.meta`
      content it writes per asset extension (png, glb, ogg, wav, ...) and
      confirm how much is Bevy-version-specific vs static boilerplate. Check
      it against at least the last two Bevy versions' defaults (0.18 vs 0.19)
      to see whether the format actually moved.
- [x] Judge drift risk: if the meta is stable, documented boilerplate, a
      Python script that writes it is safe and cheaper; if it is derived from
      Bevy internal defaults, keeping the Rust tool (which asks Bevy) is the
      correct call. Weigh that v0.7.0 made `.meta` correctness load-bearing
      for EVERY mod cubemap (the WebGL2 crash class), so silent drift now
      breaks mods, not just the base build.
- [x] PORT branch RESOLVED: not taken. The decision was KEEP, so there is no
      `scripts/gen-meta.py`, no Trunk rewire, and no parity check - see SPIKE.md.
- [x] If KEEP: record the decision and why in this task + the dev wiki
      (development.md tools list gets a "stays Rust because it asks Bevy"
      line), and update the tooling inventory (20260718-152304) so the plan
      reflects reality.
- [x] Write the SPIKE.md in this task folder either way: evidence dump,
      decision, consequences.

## Definition of Done

- SPIKE.md exists with the per-extension meta dump and the drift assessment.
- Exactly one of: `scripts/gen-meta.py` wired into Trunk with a parity check,
  OR a recorded keep-Rust decision reflected in the dev wiki and the tooling
  inventory.
- Either way, the README/wiki tools sections name the current tool and its
  invocation accurately.

## Notes

- Tool: `crates/nova_meta_gen/{src/lib.rs,src/main.rs}`; invoked as a Trunk
  post_build hook (`TRUNK_STAGING_DIR/assets`).
- Background: `docs/design/wasm-asset-meta-always.md` - source material for
  the dev-wiki write-up (see 20260718-152214's sequencing note before the
  ephemeral-docs wipe folds it away).

## Decision (2026-07-20): KEEP RUST

Full evidence + rationale in SPIKE.md. Summary: `nova_meta_gen` asks Bevy for
each loader's DEFAULT `.meta` (loader type paths + `Settings::default()`), which
are Bevy-version-specific; a Python hardcode would silently drift on a Bevy bump
and break web mod cubemaps (`.meta` correctness is load-bearing since v0.7.0).
The portal generator ported to Python (152247) because it is engine-free; meta
gen is a thin shim over Bevy's own serialization and stays Rust. Recorded in the
README tools row and the tooling-inventory umbrella (20260718-152304). Note: the
dev wiki's tools REFERENCE is the README table (development.md has no meta_gen
row), so the "stays Rust" line went there. No `scripts/gen-meta.py`; no seeded
tasks.

## Reopened (2026-07-20): the LOCATION question, not the language question

The first round (SPIKE.md above) answered "port to Python?" -> NO, it must stay
Rust because it asks Bevy for each loader's exact default meta. That finding
STANDS and is an input here, not up for re-litigation.

The user's actual objection is different and correct: `nova_meta_gen`'s output
is needed for the WEB build ONLY (native's real-filesystem 404 lets Bevy fall
back to the same defaults; only the web SPA-fallback-200-HTML trap needs the
sidecars pre-written). So it should not live as a `crates/` member of the native
game - it belongs to "the thing that builds the game for web" (web/ + Trunk),
not the game workspace.

New spike question: how to relocate the (still-Rust, still-asks-Bevy) meta
generator OUT of the game's crate list into web-build-owned tooling, WITHOUT
reintroducing Bevy-version drift (the tool's Bevy must match the game's pinned
Bevy or the "default" metas it writes will not match what the game expects).
Decide the relocation shape, then /plan it. See the reopened SPIKE.md (round 2).
