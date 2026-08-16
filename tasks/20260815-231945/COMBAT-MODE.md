# Round 3: combat mode UX - manual, auto-aim and point defence together

Scoped market research for the weapon-control decision. Four questions, asked in
`TASK.md` under "Round 3". Rounds 1 and 2 are `RESEARCH.md` and
`PLATING-AND-GREEBLES.md`; `PRIOR-POINT-DEFENCE.md` is the adjacent survey by
another lane. Where this round touches them it says so, and says whether it
confirms or contradicts.

## The headline, before the sources

**No surveyed game solves this with a mode. Every one of them solves it with a
per-weapon setting made at BUILD time.**

Nine games were reached with a primary source. Not one of them gives the player a
runtime "point defence on / off" control. What they give the player is a place,
in the fitting or refit screen, to say which weapons the computer owns. After
that the state persists, needs no key, and cannot be forgotten, because it was
never entered.

That matters here more than in any of those games, because **Nova already asks
the question at build time and already stores the answer**. A turret gets its
input binding in the editor. A turret with no binding is a mount the player
cannot fire. The autonomy rule falls straight out of data that exists:

> A bound mount is yours. An unbound mount is the ship's.

Binding cost: **zero**. Not one new key, modifier or gesture. It is the shape the
brief hoped for -- a mode that is a consequence of something the player already
does -- and it is the shape four shipped games independently arrived at.

Three supporting facts, each from shipped code:

- **Naev's `pilot_weaponAuto` creates a dedicated point-defence weapon set the
  moment a PD outfit is fitted.** The player never asks for it. Naev spends
  **no keybinding at all** on autofire, hold or point defence.
- **Naev's default weapon-set type is literally `/**< Tap to toggle, hold to
  hold. */`.** One binding carries a toggle AND a quasimode. If a mode is wanted
  later, it does not cost a key.
- **Endless Sky ships the owner's exact request as a settings row**:
  `AUTO_FIRE_SETTINGS = {"off", "on", "guns only", "turrets only"}`. "Turrets
  only" is "I fire my guns, the computer runs the battery".

And the correction that changes what is available: **the keyboard is not nearly
full. The MODIFIERS are.** Measured against the tree (section 0), `Ctrl`, `Alt`,
`Shift` and both mouse buttons are spent, but roughly nineteen plain letters, all
ten digits and `MouseButton::Middle` are free in the flight context. A design
needing a plain key is affordable. A design needing a modifier is not.

## How to read this

Round 1 and round 2 rules apply. Two labels, as in round 2:

- **DEMONSTRATED** - the source shows the thing working: shipped code, a patch
  note describing a change that was made, a documented control.
- **CLAIMED** - the source asserts it, or it is theory rather than measurement.

Every source carries its licence. Share-alike and proprietary sources are flagged
loudly. Nothing is committed beside this file. Commercial game wiki text is
quoted in short, attributed fragments for analysis only, which is the precedent
round 2 set.

**METHOD CAVEAT, stated up front because it bounds everything.** The session's
web-search budget (200 calls) was already exhausted when this round began, the
same as round 2. Nothing here came from a search engine. Every source is a direct
fetch of a URL reasoned about in advance, a raw file from a public git host, or a
MediaWiki `api.php` search. Breadth is therefore worse than round 1, and the gaps
in section 9 are gaps in COVERAGE, not evidence of absence.

## 0. The code reading this round rests on

Verified in the worktree, not taken from the brief. Line numbers will drift.

**Binding is per SECTION, at build time, and modifier-free by construction.**

- `crates/nova_scenario/src/objects/spaceship.rs:71`
  `PlayerControllerConfig::input_mapping: HashMap<SectionId, Vec<Binding>>`. Keyed
  by section id. There is no fire-group or weapon-group type anywhere in the tree.
- `crates/nova_scenario/src/objects/binding_input.rs` -
  `enum BindingInput { Keyboard(KeyCode), Mouse(MouseButton), Gamepad(GamepadButton) }`.
  `TryFrom<&Binding>` rejects anything with modifier keys, motion or wheel:
  "Only the simple, modifier-free button forms are authorable."
- `crates/nova_scenario/src/objects/spaceship.rs:492` - a turret only gets
  `SpaceshipTurretInputBinding` if `config.input_mapping.get(&section.id)` returns
  something. **An unbound turret is already a first-class, expressible state.**
  It spawns, it aims, and nothing can fire it.
- `crates/nova_editor/src/placement.rs:107` `default_binds_for` gives every
  placed `SectionKind::Turret` `MouseButton::Left` plus
  `GamepadButton::RightTrigger2`. So today an unbound turret never happens by
  accident. This is the one thing R1 has to change.

**Player turrets always track something. There is no idle state.**

- `crates/nova_ship/src/input/player/intent.rs:84` `update_turret_target_input`:
  `component_tier.or(lock_tier).unwrap_or(ray_tier)`, where `ray_tier` is the
  camera forward times 100.0. Every player turret follows the crosshair when
  nothing is locked.
- The policy is already named in that file: **LOCK-WINS**. "a combat lock holds
  the turrets even while RAISED ... tap CTRL (clearing the lock) is the explicit
  road back to manual."

**The AI point-defence assigner is complete and is gated by one query filter.**

- `crates/nova_ship/src/input/ai/point_defense.rs:72` `insert_turret_defense_target`
  inserts `AITurretDefenseTarget` only where the parent matches
  `Query<(), (With<SpaceshipRootMarker>, With<AISpaceshipMarker>)>`.
  `update_turret_point_defense` carries the same filter.
- Tunables: `AI_PD_URGENCY_FACTOR: f32 = 0.5` (line 46) is the dwell -- a mount
  breaks off only for a threat arriving in under half the time of the one it
  holds. `AI_PD_ARC_MARGIN: f32 = 0.05` (line 53) is shaved off both arc ends
  **when acquiring, never when holding**.
- `let mut claimed: HashSet<Entity> = HashSet::new();` (line 197), and line 240
  `.min_by_key(|threat| claimed.contains(&threat.entity))`.
- Reachability: `crates/nova_ship/src/sections/turret_section/arc.rs`
  `TurretSectionArc::bears_on`, **fail-open** -- an unrecognised joint tree bears
  anywhere.
- Consumption: `crates/nova_ship/src/input/ai/guns.rs:86` `ai_turret_gun_target` -
  per-turret assignment, else the ship-wide `AIPointDefenseTarget`, else the
  primary `AITarget`.

**There is no ship mode and no alert state.** No `ShipMode`, `CombatMode`,
`AlertState` or `CombatStance` type exists. The nearest things:

| Identifier | Kind | Persistence |
| --- | --- | --- |
| `WeaponsRaised(bool)` (`camera/mode.rs:44`) | Component on ship root | **momentary** -- re-derived every frame from held RMB |
| `WeaponsHot(bool)` (`targeting/state.rs:180`) | Component on ship root | derived: raised OR combat lock |
| `SpaceshipCameraControlMode { Normal, FreeLook, Turret }` | Resource | camera only, documented as not a gameplay authority |
| `AIBehaviorState { Idle, Patrol, Orbit, Engage, Evade, Retreat }` | Component | **AI ships only** |

**`WeaponsRaised` is already a quasimode.** Section 2.1 is about why that is the
strongest asset this design has.

**The HUD has no mode indicator.** The only "am I hot" signals are the target
inset border colour, the reticle pulse, and the per-turret lead pip colour
(`crates/nova_hud/src/turret_lead.rs:25/51`, `PIP_COLOR` amber ->
`PIP_HOT_COLOR` red). That file's own comment states the reason: "raised-manual
gunnery has no lock crosshair or inset on screen, so the pip the player is aiming
with must carry the state."

**Two corrections to the brief's own model of its input.**

1. **Right click is not a fire path.** `MouseButton::Right` is `CombatInput`
   (`crates/nova_ship/src/camera/rig.rs:203`), which sets `WeaponsRaised` and puts
   the camera in `Turret` mode. It makes the ship hot so the content-bound fire
   key can fire without a lock. The fire key is authored per section and is
   `Mouse(Left)` on every shipped ship.
2. **The keyboard is not nearly full; the modifiers are.**
   `crates/nova_ship/src/input/player/hints.rs:164` `flight_rig_reserved_sources`
   is the whole reserved set. Spent: `W`, `Space`, `X`, `G`, `O`, `Z`, `Ctrl`,
   `[`, `]`, `Shift`, plus `Alt` (free look) and both mouse buttons from the
   camera rig, plus `Enter`, `Escape`, `Tab`, `` ` ``, `V`, `B`, `F1`.
   **Free in the flight context:** `MouseButton::Middle`, `AltRight`, the letters
   `A C D E F H I J K L M N P Q R S T U Y`, all digits, `F2`-`F10`, and the
   punctuation block.

Two defects found in passing, neither fixed here:

- **`GamepadButton::LeftTrigger2` is bound twice.** `RcsModifierInput`
  (`input/player/flight_rig.rs:241`, and it is in the reserved list as "RCS
  modifier") and `CombatInput` / raise weapons (`camera/rig.rs:203`). Both run
  `consume_input: false`, so a pad player raises weapons every time they touch
  RCS.
- **The reserved list covers the flight rig only, not the camera rig.**
  `MouseButton::Right` (`CombatInput`) and `AltLeft` (`FreeLookInput`) are absent
  from `flight_rig_reserved_sources`, so the content lint `scenario_input_overlaps`
  and the editor's `binding_conflict` would both pass a turret bound to right
  click. `InputSource::Mouse` exists, so this is an omission, not a limitation.

---

## 1. Handing weapons to the computer, and taking them back (question 1)

### 1.1 The general answer: it is not a mode, it is a fitting

Nine games, one pattern. The runtime control is thin or absent; the decision
lives where the ship is built.

| Game | Where the decision is made | Runtime control | Binding spent |
| --- | --- | --- | --- |
| Naev | fit time, automatic (`pilot_weaponAuto`) | weapon-set keys `1`-`0` | **none extra** |
| Starsector | refit screen, per weapon group | `Shift`+`1`-`5` toggle, `X` hold fire | 1 modifier over existing keys, 1 key |
| Endless Sky | Preferences row | none | **none** |
| Cosmoteer | none needed -- PD is automatic | weapon menu states | menu, not a key |
| Space Engineers | turret Control Panel checkboxes | `Control` button; `F` releases | 1 key (release only) |
| From the Depths | **placing a controller block** | none | **none** |
| Nova (AI ships, today) | none -- automatic | none | none |

DEMONSTRATED in all rows except Naev's runtime column, where the source is the
key table in `src/input.c` and the absence is the evidence.

### 1.2 Naev: one key does toggle and hold, and PD gets its own set for free

Naev is GPL-3.0-or-later. **Code UNUSABLE. Ideas free.** This is the single most
useful source in the round.

`src/pilot.h`:

```c
typedef enum WeaponSetType_ {
   WEAPSET_TYPE_DEFAULT = 0, /**< Tap to toggle, hold to hold. */
   WEAPSET_TYPE_HOLD    = 1, /**< Activates weapons (while held down). */
   WEAPSET_TYPE_TOGGLE  = 2, /**< Toggles outfits (if on it deactivates). */
} WeaponSetType;
```

`WEAPSET_TYPE_DEFAULT` is the answer to "toggle or hold". It is **both, on one
binding**, discriminated by press duration. `src/pilot_weapon.c`, in
`pilot_weapSetPress()`: "Tap is toggle, hold is hold."

Per-weapon-set flags in the same header, which are the same two axes Nova needs:

```c
int    inrange; /**< Whether or not to fire only if the target is inrange. */
int    manual;  /**< Whether or not is manually aiming. */
int    volley;  /**< Whether or not the weapon set is firing in volleys. */
```

`manual` is **exactly Nova's manual-versus-auto-aim distinction**, and Naev keeps
it **per weapon set, persistent**, not as a ship-wide mode. It selects whether
`pilot_shootWeapon()` applies the lead solve.

Point defence is a **weapon property, not a mode**. `src/outfit.h`:

```c
#define OUTFIT_PROP_WEAP_POINTDEFENSE ( 1 << 9 ) /**< Weapon can hit ammunitions. */
```

And the fit-time grouping, from `pilot_weaponAuto()` in `src/pilot_weapon.c`,
which is the zero-binding mechanism in full:

> "Weapon set 0 is for all weapons. Weapon set 1 is for forward weapons. Ammo
> using weapons are secondaries. Weapon set 2 is for turret weapons. Ammo using
> weapons are secondaries."

```c
if ( outfit_isSecondary( o ) ) id = 1;                    /* Secondary override. */
else if ( !outfit_isProp( o, OUTFIT_PROP_WEAP_POINTDEFENSE ) && ( ... ) ) id = 0;  /* Primary. */
else if ( outfit_isLauncher( o ) && outfit_isSeeker( o ) ) id = 1;   /* Secondary. */
else if ( outfit_isProp( o, OUTFIT_PROP_WEAP_POINTDEFENSE ) ) id = haspd;
else if ( outfit_isFighterBay( o ) ) id = hasfb;
```

Fitting a PD outfit **creates a PD weapon set**. The player did nothing.

Naev's whole default keymap for this, from `src/input.c`: `KST_WEAPSET1`..`0` on
the digits `1`-`0`, `NMOD_ANY`. Targeting on `T` / `Ctrl+T` / `N` / `Backspace`.
**There is no binding named autofire, hold, or point defence.** DEMONSTRATED.

**Negative result, and it is important.** Naev's player still presses a key to
fire the PD set. Naev does **not** give the player autonomous point defence. The
build-time half of the owner's request is shipped there; the "saves your ass"
half is not.

### 1.3 Starsector: the toggle, its default set at refit, and the filled square

Proprietary. Analysis and links only. Sources are the community wiki
(`starsector.wiki.gg`), whose licence was **NOT verified this session**.

- Runtime: "'1-5' selects weapon group ... 'SHIFT 1-5' sets weapon group to
  autofire. This is shown on the HUD with a filled in square." (`/wiki/Piloting`)
- Safety: "'X' will hold fire; weapon group settings stay the same but nothing
  fires." A safety that does **not** destroy the configuration.
- Build time: "The initial _autofire_ state can also be toggled for each weapon
  group" in the refit screen (`/wiki/Refit_screen`).
- Patch note 0.97a: "set autofire state to group defaults when switching ships
  for the first time" - the default comes from the saved variant. DEMONSTRATED.
- HUD, from `/wiki/Combat_screen`: the flagship panel shows "# & name of weapons
  in the group", "damage type of the weapons", and **"fire mode and autofire
  status"**.

Two findings from Starsector that no other source gave:

**(a) Players voluntarily hand nearly everything to the computer, to buy
attention.** The wiki's own advice: "Many people find it easiest to put every
weapon on autofire except missiles. This way the mouse position only governs
shields, not shields and weapons." CLAIMED, but it is community consensus text in
the game's main piloting guide, and it is direct support for the owner's "it's
too much micro management" instinct. The player does not want the guns. They want
the one decision the guns are competing with.

**(b) Autofire is deliberately WORSE than manual.** Crew quality and the Gunnery
Implants skill both modify "target leading accuracy for autofiring weapons"
(`/wiki/Skill`), and combat readiness below 50% degrades "autofire accuracy"
(`/wiki/Combat_readiness`). Handing a weapon over costs precision. That is the
lever that keeps manual gunnery worth doing, and it is the lever Nova currently
does not have -- `lead_intercept_point` is exact for everyone.

Build-time role tagging exists too. Weapons carry a role (Point Defense, Anti
Armor, Anti Shield, Strike, General, Suppression, Finisher), and "a weapon's
role(s) effect how the AI will use it". The `Integrated Point Defense AI` hullmod
reclassifies "all small non-missile, non-strike weapons" as point-defense. So
**which mounts count as PD is itself a fitting decision**.

### 1.4 Endless Sky: the owner's shape, shipped as a settings row

GPL-3.0-or-later code, CC-BY-SA-4.0 art. **UNUSABLE for copying. Ideas free.**
Round 1 already recorded the licence.

`source/Preferences.cpp`:

```cpp
const vector<string> TURRET_OVERLAYS_SETTINGS = {"off", "always on", "blindspots only"};
const vector<string> AUTO_AIM_SETTINGS        = {"off", "always on", "when firing"};
const vector<string> AUTO_FIRE_SETTINGS       = {"off", "on", "guns only", "turrets only"};
```

Three findings, all DEMONSTRATED:

1. **`"turrets only"` is question 4 answered.** The player fires forward guns by
   hand; the computer runs the turrets. It is a settings row. It costs no key.
2. **`"when firing"` is a mode that is a consequence of an action the player
   already takes.** Auto-aim engages because the trigger is down, not because a
   mode was entered. The tooltip: "Automatically turn your ship towards your
   target to aim fixed weapons."
3. **`"blindspots only"`** is a legibility idea worth stealing outright: draw the
   turret arc overlay only where coverage is missing. Tooltip: "Setting this to
   'blindspots only' shows the overlays only when the blindspots are blocking the
   turrets' fire."

Endless Sky also ships the disagreement rule as a preference. See section 3.

Anti-missile is a numeric outfit attribute ("The power of the anti-missile shots.
Higher values make it easier to destroy stronger projectiles.") with **no player
control surface at all** in the preference set. That it fires fully automatically
is CLAIMED, not DEMONSTRATED: `source/AI.cpp` is too large for the fetch tool and
truncated before `AutoFire`.

### 1.5 Space Engineers and From the Depths: autonomy granted by placing a part

Both proprietary. Space Engineers source is **DO NOT TOUCH** (round 1 recorded the
EULA clause). Analysis only.

**From the Depths** states the mechanism in one line, on the Missile Controller
page: *"Primary action Fire to fire manually. Placing LWC or CIWS Controller
nearby enables AI control."*

That is the purest form of the recommendation in section 6. The player builds a
controller block beside the weapon and the weapon becomes autonomous. No mode, no
key, no menu. And the same battery can serve both roles: "Laser Combiners with a
Close-in Weapon System (CIWS) Controller can be dual-purposed for offensive and
defensive roles" (`/wiki/Lasers`).

**Space Engineers** does it with build-time checkboxes on the turret. Target
types: "meteors, rockets, characters, stations, large ships, small ships,
neutrals, friends, enemies", plus a subsystem priority (Default / Weapons /
Propulsion / Power Systems) and an "AI Aiming Radius". Note that **"rockets" is a
checkbox**, so "this mount does point defence" is a tick in the fitting screen.

Its hand-over is asymmetric and worth copying:

- Take over: open the Control Panel, click **Control**.
- Hand back: press **`F`**.
- The Custom Turret Controller states it plainly: "Keep AI switched off for
  manual control, or switch it on for automatic AI aiming behaviour."

**One key, spent on giving it back, not on taking it.** The take-over is a
deliberate UI act; the release is instant and cheap. Nova already has this shape:
holding right click takes the turrets, releasing gives them back.

Space Engineers' Event Controller shows what automatic-on-trigger costs in
practice. It fires actions on 21 conditions (block integrity %, stored power %,
grid speed, distance to locked target ...), configured through the Control Panel
with one action for "true" and one for "false". But it "monitors only its own
grid", so it cannot see an inbound threat. The trigger a player would actually
want -- something is shooting at me -- is the one the mechanism cannot express.

### 1.6 Cosmoteer: four named states, and PD that has no state at all

Proprietary. Wiki licence NOT verified.

Cosmoteer names four weapon-control states (`/wiki/Guides_Hub/Ship_Control`):

| State | Behaviour |
| --- | --- |
| hold fire | weapons will not activate |
| fire at target | fire only when the target is in arc and lined up |
| fire at will | shoot any enemy part in arc, rotate toward targets |
| autofire | fire continuously as cooldown resets |

Targeting is right click on an enemy part in direct control mode; `Ctrl` + right
click when weapons are in manual control mode. So Cosmoteer spends **one modifier
on the same button** to separate "manual aim" from "designate target". Nova has
no modifier to spare for that trick.

And the important half: the Point Defense System is described as *"An automated
defensive system that shoots down enemy missiles and projectiles"* that *"does not
require crew to function"*. It has **no control surface**. It is not one of the
four states. Its cost is power, paid in the ship layout.

The game whose entire pitch is that you built the ship out of visible parts gives
point defence zero UI. That is the strongest single argument against building a
mode for it.

### 1.7 What makes the current state legible: an inventory

Collected from every source that showed one.

| Mechanism | Source | Where it applies in Nova |
| --- | --- | --- |
| A per-group glyph on the HUD ("a filled in square") | Starsector | the lead pip already exists, per turret |
| Fire mode and autofire status in a fixed panel | Starsector combat screen | no equivalent |
| Overlay drawn only where coverage FAILS | Endless Sky "blindspots only" | `TurretSectionArc` is already computed |
| The mount itself moves and is visible | Cosmoteer, Space Engineers, Nova | Nova's turrets are physical sections, in view |
| Whole-ship alert, ordered and announced | general quarters | nothing yet |

**The Expanse read, grounded on the real thing.** No primary source on the show
was reachable. The real-world equivalent is general quarters: an announcement over
the ship's address system ("General Quarters, General Quarters. All hands, man
your battle stations."), historically a drum roll, with material condition changes
that are physically marked around the ship. Two properties transfer:

1. It is **feedback and a command**, never a targeting mode. Nobody sets the ship
   to red in order to make the guns work.
2. It is **announced**, and its consequences are **marked on the world** -- hatch
   labels, not a status bar.

So the red-ship read belongs on the SHIP, driven by the threat picture, and it
must not be the thing that switches the weapons on. Those are two features that
the reference conflates and Nova should not.

---

## 2. Toggle, hold, or automatic (question 2)

### 2.1 The theory says hold, and it says why

The strongest general answer is not from a game. It is the **quasimode** (Jef
Raskin, *The Humane Interface*), summarised in the Wikipedia article on modes:
modes "kept in place only through some constant action on the part of the user;
such modes are also called spring-loaded modes", whose advantage is that "the
user does not have to remember the current state of the application when invoking
a command: the same action will always produce the same perceived result."

That is the brief's question 1 answered in one sentence. A mode you cannot forget
is a mode you are holding.

Label: **CLAIMED**. This is HCI theory. No game postmortem quantifying mode error
was reachable this session, and the article does not cite kinesthetic feedback
either. Do not present it as measured.

**Nova already has the quasimode.** `WeaponsRaised` is momentary, re-derived every
frame from held right click, and resets on respawn. The design does not need to
adopt the principle. It needs to avoid abandoning it.

### 2.2 What shipped games actually spend, and on which of the three

| Game | Toggle | Hold | Automatic on trigger |
| --- | --- | --- | --- |
| Naev | `WEAPSET_TYPE_TOGGLE`, and tap of `DEFAULT` | `WEAPSET_TYPE_HOLD`, and hold of `DEFAULT` | none |
| Starsector | `Shift`+`1`-`5` per group; `X` hold fire | none | Incursion Mode (an enemy ship system) |
| Endless Sky | preference rows | none | `"when firing"` auto-aim |
| Space Engineers | `Enable AI` checkbox | none | Event Controller, own grid only |
| Cosmoteer | four menu states | none | PD, always |
| From the Depths | none | none | CIWS controller, always |
| Nova today | none | RMB raise | none |

**Nobody ships a hold for weapon handover.** Holds are spent on aiming and on
raising, not on delegating. The reason is obvious once written down: you delegate
BECAUSE your hands are busy, so a design that occupies a hand to delegate defeats
itself.

**Nobody ships a naked toggle either.** Every toggle in the table has a build-time
default behind it. Starsector's is the clearest: the refit screen sets the initial
state, patch 0.97a made "set autofire state to group defaults when switching ships
for the first time" the rule, and the runtime toggle only deviates from it for one
fight.

That reframes the question. It is not toggle versus hold versus automatic. It is:
**default from the build, deviate at runtime if you must.**

### 2.3 The automatic-on-trigger evidence, which is thinner than it looks

Two real instances were found, and both are narrower than the brief's hypothesis.

- **Starsector 0.35a patch note**: "Won't turn off autofire on PD groups when
  flux is high but enemy missiles are nearby." DEMONSTRATED. Read it carefully:
  the trigger does not TURN ON point defence. It stops an economy heuristic from
  turning it off at the worst moment. The trigger is a **guard on an existing
  state**, not a state change.
- **Starsector Incursion Mode**: "The system forces all weapon groups into
  Autofire mode", lash-activated, 15 seconds, and it also forces a Fearless
  personality and makes the ship "ignore orders". It is an enemy ship system.
  DEMONSTRATED that forced-auto exists in a shipped game, and that it is framed as
  **losing control**, not as being saved.

Against pure automatic, one real-world engagement, DEMONSTRATED: in 1991 USS
Jarrett's Phalanx, "operating in automatic target-acquisition mode, fixed on
Missouri's chaff and fired". The system did exactly what it was set to do and the
operators did not expect it. And the Iran Air 655 investigation found that
"ineffective user interface design caused poor integration with the crisis
management human processes it was intended to facilitate".

Both are about ANTI-AIR against ambiguous contacts. **Neither transfers to Nova as
a risk**, and it is worth saying why: Nova's PD threat list is built only from
hostile `TorpedoProjectileMarker` entities (`point_defense.rs`). An autonomous
mount there cannot shoot a ship, a friendly, or a piece of debris. The class of
accident that makes automatic engagement frightening in the real world is
**arithmetically unavailable**. That is the specific fact that makes automatic
safe HERE, and it should be recorded as a constraint the design must not relax.

### 2.4 The doctrinal answer, which dissolves the question

Real weapons-control states, from the multiservice brevity codes:

| State | Definition |
| --- | --- |
| WEAPONS FREE | "At targets not identified as FRIENDLY" |
| WEAPONS TIGHT | "At targets positively identified as HOSTILE." |
| WEAPONS HOLD / SAFE | **"In self-defense or in response to a formal order."** |

**Self-defence is the floor of the most restrictive state there is.** There is no
weapons-control state in which a ship stops defending itself against something
already flying at it.

So the honest framing for the owner is not "an emergency auto mode". It is:

> Point defence is not a mode. It is what the ship does when nobody is telling it
> anything. The modes are all about OFFENCE.

That is why the games in section 1 have no PD control. They are not being lazy.
They are modelling the same fact.

### 2.5 The verdict

**Automatic, unconditionally, for unbound point-defence mounts. No trigger
condition, no toggle, no hold.**

- A trigger condition ("ordnance inbound") is redundant: there is nothing for a PD
  mount to shoot unless ordnance is inbound. The condition IS the target list.
- A hull-fraction trigger ("below 40%") is worse than redundant. It means the
  first salvo is uncontested by design, which is the binary outcome
  `PRIOR-POINT-DEFENCE.md` names as the failure mode, reintroduced through the
  UI instead of through the arithmetic.
- A toggle needs a legibility budget to stop it being forgotten, and buys nothing
  the build-time decision does not already buy.

CONFIDENCE: **high** on "no trigger condition", because the redundancy argument is
structural and does not depend on a source. **Medium-high** on "no toggle",
because it rests on the pattern across seven games rather than on a measurement.

Falsifier for the whole verdict: a playtest where a player wants a specific mount
to STOP firing (ammunition, heat, signature, or a scripted stealth beat) and has
no way to say so. If that comes up, the answer is not a mode -- it is R7's hold
fire, which is a safety, not a delegation control.

---

## 3. Two targets, one battery: who wins (question 3)

### 3.1 The one game that ships the disagreement as a setting

Endless Sky, tooltip text verbatim:

> **Turret tracking**: The strategy that your fleet's turrets use when targeting
> hostiles.
> - 'focused': all turrets will attack the same target.
> - 'opportunistic': turrets will target ships that they are nearest to aiming
>   at, allowing multiple hostiles to be engaged at once.

And the default, from `source/AI.cpp`:

```cpp
bool opportunisticEscorts = !Preferences::Has("Turrets focus fire");
```

**The default is opportunistic.** DEMONSTRATED. The player's designation does NOT
capture the battery unless the player asks for it.

That is the opposite of Nova's LOCK-WINS policy, and both are defensible, because
they answer for different mounts. Endless Sky's turrets are never the player's to
aim. Nova's bound turrets are.

### 3.2 The resolution, and it is the same as R1

The disagreement only exists if one mount can receive two orders. Key autonomy on
the mount and it cannot:

| Mount | Aim source | Who wins |
| --- | --- | --- |
| bound, lock held | `lock_tier` | the lock. LOCK-WINS, unchanged |
| bound, no lock | `ray_tier` (crosshair) | the player, unchanged |
| bound, component lock | `component_tier` | the fine sub-target, unchanged |
| **unbound** | **`AITurretDefenseTarget`** | **the computer, always** |

Nothing in the existing three-tier chain changes. The unbound mount is a fourth
case that the other three never reach, because `insert_turret_defense_target`
would only ever fit mounts with no `SpaceshipTurretInputBinding`.

CONFIDENCE: **high**. It is a partition, not a priority rule, so there is no
precedence bug to get wrong.

**What it deliberately gives up:** the player cannot order a PD mount onto a
specific torpedo. Every game surveyed gives that up too. Cosmoteer, Endless Sky
and From the Depths do not expose PD targeting at all; Space Engineers exposes
only build-time target CLASSES. Nobody found it worth a control.

### 3.3 The overkill rule, already satisfied

`PRIOR-POINT-DEFENCE.md` recommends: "Do it Nebulous's way from the first commit:
a claim or reservation set over incoming projectiles". **Nova already did it**, in
`point_defense.rs:197`, with `claimed: HashSet<Entity>` and
`.min_by_key(|threat| claimed.contains(&threat.entity))`, plus a dwell
(`AI_PD_URGENCY_FACTOR = 0.5`) and an acquire-only arc margin
(`AI_PD_ARC_MARGIN = 0.05`) that Nebulous was not reported to have.

That recommendation is **CLOSED, not open**. It should be marked so on that file's
next revision. The bug that cost Sins II a patch cycle cannot occur here, and it
cannot occur for the player either, because the player would run the same
assigner.

---

## 4. Manual fire while the computer keeps the rest on point defence (question 4)

**Yes. It is shipped, in at least four games, and it reads well enough that none
of them advertises it as a feature.**

| Game | How | Evidence class |
| --- | --- | --- |
| Endless Sky | `AUTO_FIRE_SETTINGS` includes `"turrets only"` | DEMONSTRATED, source |
| Cosmoteer | PD is automated and crewless; every other weapon is player-directed | DEMONSTRATED, wiki |
| From the Depths | one weapon manual; another has an LWC or CIWS controller beside it | DEMONSTRATED, wiki |
| Space Engineers | player takes one turret with `Control`; the rest keep AI targeting | DEMONSTRATED, wiki |
| Naev | separate PD weapon set, still player-pressed | partial |
| Nova, AI ships today | `ai_turret_gun_target` falls back per turret | DEMONSTRATED, code |

The consistent detail is that **the split is per MOUNT, never per ship**. Not one
source splits it as a global "the ship is in PD mode". Space Engineers is the
sharpest instance: the player is inside one turret while every other turret on the
grid keeps engaging.

**And Nova's AI already does the whole thing.** `ai_turret_gun_target` reads the
per-turret `AITurretDefenseTarget` first, then the ship-wide
`AIPointDefenseTarget`, then the primary `AITarget`. An AI corvette fires its main
battery at a ship while individual mounts break off onto torpedoes. The shape the
owner is describing runs in this game today, against the player, and the player
has no version of it. That is the balance hole the brief names, stated precisely.

---

## 5. The binding budget

### 5.1 What other games spend

| Game | Bindings spent on weapon delegation |
| --- | --- |
| Naev | **0** |
| Endless Sky | **0** (preferences) |
| Cosmoteer | 0 keys; menu states, plus `Ctrl` + right click for manual-mode targeting |
| From the Depths | **0** (a block placement) |
| Space Engineers | 1 (`F` to release), plus a Control Panel |
| Starsector | 1 modifier over `1`-`5`, plus `X` for hold fire |
| Nova today | 0 for delegation; RMB and `Ctrl` are already spent on raise and lock |

**Five of seven spend nothing.** The two that spend something spend it on giving
control BACK (`F`) or on a safety (`X`), not on taking it.

### 5.2 Nova's actual budget, and where the pressure really is

The brief says three new modifiers is not usable. Measured (section 0), that is
right for the wrong reason. `Ctrl`, `Alt` and `Shift` are all spent, and both
mouse buttons are spent, so **there is no modifier left at all** -- not one, let
alone three. But `MouseButton::Middle`, `AltRight`, nineteen letters and every
digit are free.

Consequences for the design space:

- Any option needing "the existing action, but modified" is **dead**. That rules
  out Cosmoteer's `Ctrl` + right click and Starsector's `Shift`+`1`-`5` in their
  shipped forms.
- Any option needing one plain, memorable key is **affordable**. `H` for hold
  fire, for instance.
- Any option needing zero bindings is **free**, and three of them exist.

### 5.3 The three zero-binding options, ranked by how much they buy

1. **Unbound mount is autonomous.** Uses data that already exists. Buys the whole
   feature. Costs one editor affordance, not a key.
2. **Tap versus hold on right click** (Naev's `WEAPSET_TYPE_DEFAULT`). Buys a
   latched combat-stations state on a button that already exists. Costs a
   discrimination delay on a button the player uses to point-fire, and Nova
   already spends one of those on `Ctrl` (`RADAR_TAP_SECS = 0.25`). Second, and
   only if the latch is wanted.
3. **Auto-aim as a consequence of firing** (Endless Sky's `"when firing"`). Buys
   nothing here: Nova's turrets already track through `ray_tier` whether or not
   the trigger is down. Listed for completeness and as a pattern to reuse
   elsewhere.

---

## 6. Ranked recommendations

What `20260816-114054` should try, in order.

### R1. Make autonomy a property of the MOUNT, decided in the editor. Build no mode.

An unbound turret defends the ship. A bound turret is the player's, exactly as
today.

Mechanically this is small, because the assigner is finished:

- `insert_turret_defense_target` (`point_defense.rs:72`): replace the
  `With<AISpaceshipMarker>` filter with "the parent is a spaceship root AND this
  turret has no `SpaceshipTurretInputBinding`".
- `update_turret_point_defense`: same change to `q_ship`.
- Consumption: player turrets read `TurretSectionTargetInput` from
  `input/player/intent.rs`, not from `ai/guns.rs`. Add the unbound case to the
  player feed as a fourth tier that the existing three never reach, rather than
  running the AI feed on a player ship. Keep one writer per component.
- `crates/nova_editor/src/placement.rs:107` `default_binds_for` must stop being
  the only path. A placed turret needs a visible, reachable "leave unbound" state,
  or the feature is unreachable in practice.

Evidence: Naev `pilot_weaponAuto`; Starsector's refit-screen initial autofire
state and the 0.97a "group defaults" note; Space Engineers' target-type
checkboxes; From the Depths' "Placing LWC or CIWS Controller nearby enables AI
control"; Endless Sky's `"turrets only"`; Cosmoteer's crewless automatic PD.

CONFIDENCE: **high**. Six independent games, and the data already exists in the
tree.

FALSIFIER, and it is a real risk: **players bind every mount out of habit, so no
mount is ever autonomous and the feature never fires.** Measurable before any
code -- count turrets with no `input_mapping` entry across the shipped scenario
ships in `assets/base/scenarios/*.content.ron`. If the answer is zero, R1 needs
the editor change first and the gameplay change second, in that order.

Second falsifier: a player binds a mount, discovers it no longer defends them, and
reads that as a bug. If playtest shows that, the fix is R2, not a mode.

### R2. Give the mount a visible owner. One colour, on a pip that already exists.

`crates/nova_hud/src/turret_lead.rs` already draws one pip per player turret and
already changes colour on `WeaponsHot`. Add a third colour for computer-held.
That is the entire legibility feature.

Evidence: Starsector shows autofire state as "a filled in square" per group, and
lists "fire mode and autofire status" in the flagship panel. DEMONSTRATED that a
per-group glyph is sufficient in a game with more weapon groups than Nova has
mounts.

CONFIDENCE: **high** on the mechanism, **medium** on the exact treatment.

FALSIFIER: a screenshot test where a viewer cannot say, in under a second, which
mounts are theirs. The project has settled two skin rules by rendering; settle
this one the same way.

Do this in the SAME commit as R1. An autonomous mount with no on-screen owner is
the mode-error failure the brief is trying to avoid, arriving through the back
door.

### R3. Ship the alert read as an OUTPUT, not a control.

The Expanse's red-ship moment is feedback. General quarters is announced and
marked, and it is ordered separately from any weapon being switched on. Nova
already derives `WeaponsHot` and already has `ThreatContacts`; a ship-wide alert
read is a HUD and lighting concern with no gameplay authority, exactly like
`SpaceshipCameraControlMode` is documented to be.

Keep it strictly downstream. The moment the alert state gates a weapon, it becomes
a mode, and section 2 says do not build one.

CONFIDENCE: **medium-high**. The fiction reference could not be sourced; the
real-world analogue could. Falsifier: the owner looks at it and says the ship
should turn red because they pressed something.

### R4. Do not add a trigger condition. Not inbound-ordnance, and especially not hull fraction.

The target list IS the trigger. A hull-fraction gate guarantees the first salvo is
uncontested, which is the binary outcome `PRIOR-POINT-DEFENCE.md` identifies as
the mechanic-deleting failure, reintroduced through the interface.

Evidence: Starsector's only shipped PD trigger (0.35a) is a guard that stops PD
autofire being turned OFF, not one that turns it on. Doctrinally, WEAPONS HOLD
already permits self-defence.

CONFIDENCE: **high**. Falsifier: playtest shows always-on PD trivialises the
torpedo economy. Note that this is a BALANCE falsifier with balance answers
already banked next door -- damage subtraction, an interception cap, or shooting
the PD turrets off -- and none of them is a UI change.

### R5. Reuse the assigner unchanged, then re-tune it in view.

`AI_PD_URGENCY_FACTOR = 0.5` and `AI_PD_ARC_MARGIN = 0.05` were tuned against AI
ships that nobody watches closely. On the player's hull the mount slews in frame,
a metre from the camera, and the same hysteresis may read as twitch.

CONFIDENCE: **medium** that they need changing, **high** that they need looking
at. Falsifier: record thirty seconds of a torpedo salvo against a player hull with
four unbound mounts and watch the barrels. Round 2's lesson applies -- the render
is the only thing that has ever settled a question on this project.

Keep the fail-open rule in `bears_on`. A modded mount with an unrecognised joint
tree defending its ship is the right failure.

### R6. Make delegation cost accuracy.

Starsector degrades autofire lead accuracy with crew quality, combat readiness and
a skill; Endless Sky's `"turrets only"` is a convenience, not an upgrade. Nova's
`lead_intercept_point` is exact for everyone, so an autonomous mount would be as
good a gunner as the player, and manual gunnery becomes pointless.

Cheapest form: a small angular error, or a lower `AIM_CORRECTION_GAIN`, on
computer-held mounts only.

CONFIDENCE: **medium**. One game demonstrates the lever; nothing measures how big
it needs to be. Falsifier: playtest where players still never take manual control,
or where the error makes autonomous PD useless and the torpedo economy flips the
other way. Do this AFTER R1 lands, and only if the playtest asks for it.

### R7. If a control is wanted, make it HOLD FIRE, not auto.

Starsector spends `X` on it ("weapon group settings stay the same but nothing
fires"), Cosmoteer names it as one of four states, and WEAPONS HOLD is a real
doctrinal state. Nova has the constant already, disabled:
`hints.rs:32 HOLD_FIRE_DURING_RADAR: bool = false`.

Cost: one plain key. Section 5.2 shows Nova has nineteen letters free. The
scarcity is modifiers.

CONFIDENCE: **medium**. It is the control other games chose to spend a key on, but
it is a safety, not a delegation control, and Nova may not need it. Falsifier:
count accidental discharges in playtest. If nobody fires by mistake, a safety is
dead weight.

### R8. Only if a latched combat-stations state is still wanted: tap versus hold on right click.

Naev's `WEAPSET_TYPE_DEFAULT` -- "Tap to toggle, hold to hold" -- is the shipped
proof that one binding carries both. Right click already raises weapons on hold.
Tap could latch it.

Ranked last on purpose. It reintroduces exactly the forgettable mode that R1
avoids, it adds a second press-duration discrimination to a game that already has
one on `Ctrl`, and it puts that discrimination on the button used to point-fire,
where latency is felt.

CONFIDENCE: **low** that it is needed, **high** that this is the right mechanism
if it is. Falsifier: measure whether the tap window is perceptible when
point-firing. If it is, this option is dead and the answer is a plain free key.

---

## 7. Licence positions for everything cited in this round

Nova is MIT. Share-alike and proprietary code is UNUSABLE for copying; ideas are
free everywhere.

| Source | Licence | Status |
| --- | --- | --- |
| Naev (`src/pilot.h`, `pilot_weapon.c`, `outfit.h`, `input.c`) | **GPL-3.0-or-later** | **READ ONLY.** Never copy. The weapon-set MODEL is free to reimplement. |
| Endless Sky (`source/Preferences.cpp`, `source/AI.cpp`, `data/_ui/tooltips.txt`) | code **GPL-3.0-or-later**, art **CC-BY-SA-4.0** | **READ ONLY.** Round 1 already recorded this. |
| Starsector | proprietary | Analysis and links only. |
| `starsector.wiki.gg` text | wiki licence **NOT verified this session** | Short attributed quotes for analysis only. |
| `cosmoteer.wiki.gg` text | **NOT verified** | Same. |
| `spaceengineers.wiki.gg` text | **NOT verified** | Same. |
| `fromthedepths.wiki.gg` text | **NOT verified** | Same. |
| Cosmoteer, Space Engineers, From the Depths (the games) | proprietary; Space Engineers source is a custom EULA round 1 flagged **DO NOT TOUCH** | Never vendor. Mechanics are facts. |
| Wikipedia (Mode (user interface), Phalanx CIWS, General quarters, Aegis Combat System, Multiservice tactical brevity code) | **CC-BY-SA 4.0** | Facts free, **text not reusable**. |
| Jef Raskin, *The Humane Interface* | book, copyright | Idea free. Not quoted from the book itself -- only the encyclopaedia summary was reached. |
| Multiservice brevity codes | US DoD publication, facts | Free. |

Nothing was committed beside this file. No screenshot, marketing image or review
text was fetched or stored.

---

## 8. What CONTRADICTS or corrects earlier rounds and the brief

1. **`PRIOR-POINT-DEFENCE.md`'s "fix the overkill bug at the start" is CLOSED,
   not open.** Nova already ships the claim set, and with a dwell and an
   acquire-only arc margin that the Nebulous report did not mention.
   `point_defense.rs:197` and `:240`. That file should be marked.
2. **`PRIOR-POINT-DEFENCE.md` frames point defence as a BALANCE problem. It is
   also a UX non-problem, in every game surveyed.** Six of seven give the player
   no point-defence control whatsoever. The one that does (Naev) still creates the
   weapon set automatically at fit time. Nothing in that file is wrong; the
   omission is that its balance recommendations do not need a control surface to
   land.
3. **The brief's "right click is point-fire at anything" is not what the code
   does.** `MouseButton::Right` is `CombatInput` -> `WeaponsRaised` plus a camera
   mode change (`camera/rig.rs:203`). It makes the ship hot so that the
   content-authored fire key -- `Mouse(Left)` on every shipped ship -- can fire
   without a lock. Right click never fires anything.
4. **The brief's "the keyboard is nearly full" is half right, and the half it
   gets wrong changes the design space.** Every MODIFIER is spent (`Ctrl` lock,
   `Alt` free look, `Shift` RCS) and both mouse buttons are spent. Plain keys are
   not: `MouseButton::Middle`, `AltRight`, nineteen letters and ten digits are
   free. So "one new key" is cheap and "one new modifier" is impossible, which is
   the opposite of how the constraint reads.
5. **The brief asks whether any option avoids a new binding. Three do**, and the
   best of them costs less than zero, because it deletes a decision rather than
   adding one. See section 5.3.
6. **`WeaponsRaised` is already a quasimode, and nobody had written that down.**
   The `camera/mode.rs` doc treats it as a camera concern. It is also the single
   piece of interface theory this design most needs to preserve.
7. **The Expanse reference conflates two features.** "Combat stations" is an
   alert READ. "Every mount comes alive" is a weapon STATE. In the real analogue
   they are separate: general quarters is ordered and announced, and WEAPONS HOLD
   still permits self-defence regardless. R3 keeps them separate deliberately.
8. **Two defects found in passing, neither fixed here.**
   `GamepadButton::LeftTrigger2` is bound to both `RcsModifierInput`
   (`flight_rig.rs:241`) and `CombatInput` (`camera/rig.rs:203`), both with
   `consume_input: false`, so a pad player raises weapons whenever they use RCS.
   And `flight_rig_reserved_sources` (`hints.rs:164`) omits the camera rig, so
   `MouseButton::Right` and `AltLeft` are not reserved and both the content lint
   and the editor conflict check would pass a turret bound to right click.
   `InputSource::Mouse` exists, so this is an omission rather than a limitation.

---

## 9. What could NOT be found out

Stated plainly rather than filled in.

- **Elite Dangerous turret modes.** The brief names them and they are the most
  cited prior art in the field. `elite-dangerous.fandom.com` returns **HTTP 402**
  to automated fetches (the same block round 2 hit), `elite-dangerous.wiki.gg` and
  `elitedangerous.wiki.gg` do not exist, `edrefcard.info` renders user binding
  files rather than listing controls, and a reader-proxy attempt reached a
  Cloudflare challenge. **The three-mode design is widely reported but is NOT
  asserted anywhere in this document**, because no primary source was reached.
  This is the largest single gap.
- **Star Citizen master modes, and their reception.** `starcitizen.tools` returns
  **HTTP 403** domain-wide. This was the best available case study of a forced
  mode switch and player reaction to it, and none of it is here. No recommendation
  rests on it.
- **X4: Foundations turret behaviours.** `egosoft.com` has no reachable manual
  (the PDF path 302s to a 404 page), `roguey.co.uk/x4` documents data not
  behaviours, and the Steam guide search returned no matching listings. X4's
  per-turret-group behaviour list is the closest published analogue to a
  build-time PD assignment and it could not be verified.
- **NEBULOUS: Fleet Command weapon control.** Both `nebulousfleetcommand.wiki.gg`
  and `nebfltcom.wiki.gg` return **HTTP 401** to automated fetches. The only
  Nebulous fact in the record remains the second-hand line in
  `PRIOR-POINT-DEFENCE.md` ("PD turrets will try to target different missiles to
  split their fire"), which that file already flags as unverifiable from here.
- **FTL, Highfleet and Avorion.** `ftl.wiki.gg`, `highfleet.wiki.gg` and
  `avorion.wiki.gg` all return **HTTP 401**; the Fandom mirrors return 402. FTL's
  autofire checkbox and its fully autonomous defence drones would have been a
  useful fourth data point on "automatic with no control", and Avorion's
  auto-targeting-as-a-fitted-module would have been a fifth instance of R1.
- **ANY submarine or naval sim with a weapons officer.** The brief asked for this
  explicitly and **it is unanswered**. `uboat.wiki.gg` returns 401,
  `wiki.wargaming.net` 301s to `wiki.worldofwarships.com` where both spellings of
  the anti-aircraft page return 404, and `wiki.warthunder.com` is not a MediaWiki
  and has no `api.php`. World of Warships' priority-sector mechanic and War
  Thunder's AI gunners are both direct instances of question 4 and neither could
  be sourced.
- **Whether Endless Sky's anti-missile fires without any player input.** The
  preference set and the tooltip strongly imply it, but `source/AI.cpp` truncated
  in the fetch tool before `AutoFire`, so it is labelled CLAIMED. Only
  `opportunisticEscorts = !Preferences::Has("Turrets focus fire")` was read
  directly.
- **Any measurement of how often players forget a toggled mode.** The quasimode
  argument is HCI theory and is labelled CLAIMED throughout. No game postmortem,
  telemetry report or patch note quantifying mode error was reachable. This
  project has twice adopted a rule from weak evidence and disproved it by
  measurement; treat section 2.1 accordingly.
- **Formal Phalanx CIWS mode names and firing authority.** The Wikipedia article
  names none; it records "automatic target-acquisition mode" and the USS Jarrett
  incident only. The `Close-in weapon system` and `Aegis Combat System` articles
  carry no mode taxonomy either. Real-world doctrine in this document is limited
  to the brevity codes, which are solid.
- **What proportion of Nova's shipped turrets could be left unbound.** Not
  counted. R1's first falsifier is a grep over
  `assets/base/scenarios/*.content.ron` and it was left for the implementing task,
  because the answer decides whether the editor change or the gameplay change goes
  first.
- **METHOD CAVEAT, repeated because it is the biggest limit.** The 200-call
  web-search budget was already spent before this round began. Every source above
  is a reasoned direct fetch. Nine games were reached; five more were attempted
  and blocked at the HTTP layer.
