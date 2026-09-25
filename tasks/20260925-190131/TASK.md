# Scope situated faction encounters and station jobs

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,world,design

## User facts
- The owner is interested in factions beyond the three current gameplay categories, more enemy and neutral ship designs, random events and quests from stations, while keeping a realistic near-future setting. These are research proposals, not approved feature promises.

## Agent findings
- `Allegiance` is currently Player/Enemy/Neutral and `relation()` is a closed pure function (`crates/nova_gameplay/src/relations.rs:26-61`); AI acquires sensor contacts, authored scenarios dispatch events, and streamed bodies are not authored-addressable. No world-scoped encounter director, faction standing or station job board exists. Lore distinguishes state military, corporate security, independent lawful contractors and piracy without treating them as unified four factions (`web/src/lore/economy.md:54-76`, `law-and-power.md:36-60`).

## Proposed design work, not implementation approval
- Compare an authored deterministic sector encounter, a Bevy-owned seeded visit event and a hybrid named encounter template. Decide stable identities, event schedule, mod-authored data/lint, and no-duplicate-reward/retire policy before implementation.
- Compare retaining coarse Allegiance plus situated organization metadata with replacing the relation interface by an extensible faction graph; state impact on AI, HUD, missions and saves. Do not assume a standing number or reputation-gated weapons.
- Choose one player-readable consequence (escort request, disputed salvage, neutral distress or security patrol), with one compelling alternative to combat. New enemy designs must remain recognizable catalog designs; coordinate with existing derelict-role task `20260923-110307` rather than duplicating it.

## Verification to design
- Named repeatable encounter from a world seed and event identity; authored scenario behavior unchanged; live AI targeting, neutral refusal and outcome feedback asserted; no repeat rewards after leave/return in whichever persistence tier is approved. Playtest whether approach and flight remain central rather than only menu text.

## Done when
- Owner has compared one encounter loop and faction identity boundary with consequences; chosen content, UI and lifetime contracts are ready for separate implementation gate. No scripted `WorldScript` or faction save type is assumed to exist.
