# Torpedo blast self-harm at close range: own ship and salvo siblings caught in blast

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.4.0,torpedo,balance

Observed in the 06_torpedo_range headless logs while fixing 20260709-131502:
a torpedo detonating on the NEAR gate (30 units) put the firing ship inside its
own blast (blast_radius 30, ~48 damage to three player sections) and dealt ~58
damage to the next torpedo of the salvo that had just left the bay (its sections
have 1.0 hp). Contact-owner filtering is fixed; this is the separate, by-design
blast path, which affects everyone in radius - but the current tuning means
shooting a close target hurts you and can wipe your own follow-up salvo.

This is a balance/design question, not a code bug:

- Should the blast damage the firing ship at all (arming keeps the torpedo from
  detonating early, but the blast at a legitimately close target still reaches
  back)?
- Should salvo siblings be blast-immune, tougher (more than 1.0 hp), or is
  fratricide desirable spacing pressure?
- Or is this purely a numbers problem (blast_radius 30 vs near-gate 30 vs
  fire_rate 1/s)?

## Steps

- [ ] Decide the intended behavior with the user (blast friendly-fire on owner:
      yes/no; salvo fratricide: yes/no) before touching code.
- [ ] Apply the decision: config knobs on TorpedoSectionConfig (e.g. blast
      falloff, min self-damage range) or an owner/sibling exemption in the blast
      overlap path in bevy_common_systems (which is our crate, see AGENTS.md:
      changes there get a task in ~/personal/bevy-common-systems).
- [ ] Cover with a range scenario or physics test per the decision.

## Notes

- Log evidence: first detonation at the near gate applied blast damage 48.01 to
  colliders of body 974v0 (player ship) and 58.01 to both sections of torpedo
  1110v0 (fired 14 ms earlier). Same behavior on master and after the
  owner-filter fix, as expected.
- Related: 20260709-091756 (one hit = one cue dedup) will change how such blasts
  sound/feel; 20260525-133025 (ammo limits) raises the cost of losing a salvo.

## Decision

We decided to keep the blast as-is: it can hurt the firing ship and its salvo
siblings. The rationale is that this encourages players to maintain a safe
distance when firing torpedoes, adding a layer of tactical consideration to
their use. Players will need to be mindful of their positioning and the timing
of their shots to avoid self-inflicted damage, which can lead to more strategic
gameplay.
