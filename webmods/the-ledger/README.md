# The Ledger: Field Trials

Six short flight and combat activities. No cast, dialogue scenes, or carried
story state. Fly through dense asteroid belts, survey instruments, patrol
relays, and fight around working traffic and planetary backdrops.

This is the **2.0.0 worktree preview**, not a portal publication. It replaces
the previous campaign. No old progress is required.

## Play this checkout

From the repository root:

```sh
nix develop --command bash scripts/preview-ledger.sh
```

Pass `2` through `6` to start another activity. The launcher copies this mod
into a temporary profile and removes that profile when the game exits. It
does not change your installed mods, settings, or saves.

In the Scenarios picker, expand **The Ledger: Field Trials**. Every activity
is independently replayable. Victory offers Continue, except at the finale.
Defeat retries the current activity with a fresh ship. Flight is live at spawn;
the short corner title does not take the controls.

## Activities

| Activity | Ship | Task |
| --- | --- | --- |
| **Drift Run** | Racer | Timed slalom through an asteroid belt. |
| **Rock Garden** | CargoA | Clear a staged arena fight. |
| **Blue Survey** | Racer | Scan three sites in any order. |
| **Cold Patrol** | CargoA | Inspect relays and clear hostile patrols. |
| **Freight Lane** | CargoA | Screen two transports over 8 km past hostile blockers. |
| **Siege Line** | CargoB | Assault an outpost with a wingman. |

The objective carries the instructions. Beacons carry short place or gate
labels. Drift Run starts its clock at START, requires a direct pass through
each 50 m gate, sounds each clearance, and freezes the clock at FINISH; only
the next gate is active. Survey sites show a
countdown while scanning. Leaving
a site resets that scan, not sites already completed.

## Fleet and environment

- The Racer, CargoA, CargoB, and cruciform platform retain their existing
  meshes and section layouts. Freighter variants reuse the same hulls.
- CargoA has two PDCs. CargoB adds two torpedo pods. Guns use left mouse;
  torpedoes use **R**. The Racer is unarmed.
- Player ships have a 500 m/s manual-speed governor.
- Modelled structural sections have twice their 1.x health. Player computers
  and engines have extra protection. The player's CargoA nose, which connects
  both guns, also has stronger plating. Raiders remain weaker.
- Mod-owned PDCs keep this fleet independent of base weapon tuning. The
  workboat gun fires 35 rounds/s at 4 damage; the patrol gun fires 25 at 2.
  Both have finite magazines and reloads. Base weapons are unchanged.
- Every map has two seeded planets, more than 50 mixed-material asteroids,
  and two AI traffic routes. Cover and open flight routes are authored
  separately. Asteroids are destructible and have no gravity; planets have
  real gravity wells away from the objective routes.
- Combat remains dangerous. Moving across incoming fire is safer than
  stationary trading. Reinforcements have an arrival grace unless shot.

## Authoring and checks

The RON is authored mod content, not output from `content gen`. GLB parts
live under `gltf/parts/` (Kenney Space Kit, CC0; see `credits/CREDITS.md` at
repository root). Effects, sounds, and skies use `dep://base/`. Picker art
remains generated title-card placeholders.

The six new scenario ids and their order are in `ledger_campaign.content.ron`.
Old chapter ids are removed. The campaign id remains `the_ledger`.

```sh
nix develop --command cargo run --features dev -- content lint --target the-ledger
nix develop --command cargo test -p nova_authoring --test ledger_campaign
python3 scripts/gen-scenario-thumbnails.py --check
```

Contract tests cover membership, retries, immediate play, concise beacon
labels, terrain and traffic, ordered gates, resettable scans, convoy arrivals,
resources, and health/weapon budgets. They do not prove flight, balance, or
rendered appearance.
