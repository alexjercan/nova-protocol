# Ship skin styles: moddable looks with deterministic decoration fixtures

- STATUS: OPEN
- PRIORITY: 76
- TAGS: v0.11.0, ship, render, modding, content

## Goal

The derived skin covers a hull correctly but reads as bare plate. Give it a
VOCABULARY - vents, ribbing, antennae, trim, blisters - scattered from the
neighbourhood the derivation already computes, and make the whole look
selectable as an authored, moddable STYLE.

Blocked on `20260815-190741` (the derived skin) landing on master. Do not start
before that.

## Decisions already made by the owner

- **Decorative fixtures are DESTRUCTIBLE**, exactly like skin plates: health,
  mass, collider, `SectionFixture`, damage isolated from the section behind.
  Consistent with cladding, and shooting an antenna off should work.
- **Style is authored PER SHIP**, referenced by id.
- **Styles are CONTENT, not constants.** A mod can define one, and a scenario or
  campaign can inject it - enemy ships wearing a raider style, civilians wearing
  a clean one. Author them through the Rust content builders like every other
  content type; never hand-edit the generated RON.

## Structure: build the spine once, then fan out

Four agents each inventing a fixture system produces four incompatible
architectures and nothing that can be grafted. So:

### Phase A - the mechanism (one agent, no art decisions)

Expose the neighbourhood facts `boundary_heights` ALREADY works out, and scatter
against them. At minimum: is this plate flat, is it a rim, a ridge, a peak, how
enclosed is its cell, which way does it face, how long is the contiguous flat
run it sits in.

That vocabulary is what Townscaper's prop scatter reads, and it costs nothing
here because the derivation has already computed it.

Also in Phase A: the style hook. A style is DATA - a set of materials per
surface role, plus a fixture set with scatter rules and densities - resolved by
id from a ship's config.

**Determinism is not optional.** Scatter must be a pure function of structure,
hashed off cell position rather than RNG state, exactly like the skin. Otherwise
decorations flicker while a hull is dragged in the editor and a reloaded ship
comes back wearing different antennae.

### Phase B - four candidate looks (four agents, in parallel)

Same vocabulary, different content: which fixture goes where, what materials,
what density. Because all four speak the same language, taking the trim from one
and the vents from another is a content merge, not a rewrite.

Directions chosen to DIVERGE, and to map onto the ship cast that already exists:

1. **Industrial** - exposed panels, ribbing, radiators, hazard striping.
2. **Armoured** - flat plate, hard edges, few protrusions, sensor blisters.
3. **Civilian** - smooth, minimal greebles, accent bands. The racer.
4. **Salvage** - mismatched plates, welded patches, antennae, deliberate
   asymmetry. Raiders.

### Phase C - the owner chooses, then graft

## The comparison must be honest

Fix the subject BEFORE Phase B starts: same ship, same seed, same camera, same
lighting, as a harness constant. Every candidate renders that.

This is not ceremony. Every A/B render judged in the first half of the skin work
was invalid because `freeze_bodies` did not exist and the subjects rotated
between runs at a rate that depended on machine load. Comparability has to be
built in, not assumed.

## Research is already banked - do not re-run it

`tasks/20260815-190741/NOTES.md` holds the decoration-continuity findings:
corner-tile systems (Lagae and Dutre 2006) with Barrett's mid-edge fix,
canonicalising decoration by EQUALITY PATTERN rather than value (625 -> 15),
Townscaper REMOVING decoration from tiles in favour of neighbourhood-read prop
scatter, Hardspace: Shipbreaker making panel lines BE module boundaries, and why
corner matching degrades straight features (panel lines want trim or decals).

This task is execution and judgement, not discovery.

## Watch list

- Decorative fixtures add colliders on top of the ~400 a clad ship already
  carries. Measure; the skin's own measurement showed plate colliders sat inside
  the run-to-run spread, but decorations are additional.
- `damage_tint` must not capture decoration meshes either. Fixtures are already
  exempt via the ancestor walk - confirm it still holds for a new fixture kind.
- Keep fixtures close to behaviourless. The rule from the skin work stands: if
  shooting it off should cost the ship a CAPABILITY, it is a section, not a
  fixture.

## Definition of done

- A ship's style is authored by id, resolves from content, and a mod can add one
- Decorations scatter deterministically from structure; the same ship always
  wears the same ones
- Four candidate looks rendered against a fixed subject and pose
- The owner has chosen, and the chosen look ships as base content
- Decorations are destructible fixtures, isolated from section health
- Rendered evidence attached; docs and CHANGELOG ship with it
