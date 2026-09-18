# What the lessons still have to say

An out-of-context review of the shipped handbook
(`crates/nova_authoring/src/base_content/lessons.rs`, 24 lessons) against the
player wiki (`web/src/wiki/**`, 3075 lines over 22 pages). Two reviewers read
the corpus cold, with no memory of how the lessons were written, and traced
every claim to the code that implements it. Their findings are merged here.

The brief this answers: cover all the wiki's sections and all the important
details, in technical wording, so a lesson says plainly WHAT a thing does and
WHAT it means.

Three things came out of it, in the order they should be acted on:

1. **Fifteen lessons carry a claim that is wrong or that hides the rule which
   decides the outcome.** Eight of those tell the player the game does
   something it does not do. Fix these before adding anything.
2. **Six wiki areas have no lesson at all** - the railgun (231 lines), docking,
   the controller section, gravity, the NOVA OS terminal and its command set,
   and the whole editor gesture set.
3. **Forty candidate lessons** are specced below, each with a body inside the
   45-word cap, verified action ids and a verified wiki anchor. That is more
   than the handbook should ship; the tiering says which subset is the release
   set.

Every `path:line` below was read. The five load-bearing corrections were
re-verified in the main session against the code, not the reviewers' notes.

## Contents

- [1. Corrections](#1-corrections)
- [2. Coverage gaps by category](#2-coverage-gaps-by-category)
- [3. Detail gaps, lesson by lesson](#3-detail-gaps-lesson-by-lesson)
- [4. Proposed lessons](#4-proposed-lessons)
- [5. What blocks the rest](#5-what-blocks-the-rest)

---

## 1. Corrections

### 1.1 The eight that describe behaviour the game does not have

**C1. NOVA OS does not run while you fly.** `novaos_open` body
(`lessons.rs:461`) and field note (`:466`): "NOVA OS runs while you fly... The
ship keeps flying while the screen is open." `novaos_view` (`:480-481`) repeats
it. `PauseStates::NovaOs` is a frozen variant - "the clocks (`Time<Virtual>` +
`Time<Physics>`) pause on entering any frozen variant"
(`crates/nova_gameplay/src/lib.rs:143-146`, variant doc at `:164-166`,
`is_frozen` at `:194-196`). The wiki has it right: "While the computer is open
the game is frozen: the clocks stop, so combat, physics, AI and every
projectile hold mid-frame" (`nova-os.md:53`). This is the handbook's most
misleading line - it invites a player to open the terminal mid-fight expecting
to keep flying. VERIFIED in-session.

**C2. ORBIT does not use your mark.** `flight_orbit` (`lessons.rs:279`): "ORBIT
flies a circle around your mark and holds it." It orbits the ship's DOMINANT
GRAVITY WELL at engage time. The input layer only names the well
(`crates/nova_ship/src/input/player/flight_rig.rs:486-497`), and outside every
SOI the key is a documented no-op: "Parking needs a well; outside every SOI
this is a no-op". `AutopilotAction::Orbit { well }` carries the well, not a
target (`crates/nova_ship/src/flight/state.rs:190`). The lesson contradicts the
page its own `wiki_path` points at (`gravity-wells.md:60`). VERIFIED
in-session.

**C3. ORBIT does not hold the target in view.** `flight_orbit`
(`lessons.rs:280`): "It keeps the target in view while you do something else."
ORBIT holds a ring and a plane; nothing in it aims the hull. Target-facing is
the combat stance's job.

**C4. Raising weapons does not change how the flight computer flies.**
`combat_stance` body (`lessons.rs:315-318`) and field note (`:323`). The
`WeaponsRaised` component's own doc names its consumers - "the radar slot
latch, the weapons safety, manual turret aim ... never the camera enum"
(`crates/nova_ship/src/camera/mode.rs:34-45`). Flight is not among them. What
raising actually does: switches the camera to Turret mode, moving mouse look
off the hull's rotation command and onto turret aim (`mode.rs:79-122`), and
latches the red lock slot (`targeting-radar.md:57-60`). The behaviour the
lesson describes is the ENEMY's fight maneuver - hold a velocity, keep the nose
on the target (`factions.md:29`; `AutopilotAction::MatchVelocity { velocity,
facing }`, `crates/nova_ship/src/flight/autopilot.rs:680-686`). The handbook
has attributed an AI behaviour to the player. VERIFIED in-session.

**C5. Your own ship does not block your own turrets.** `combat_turrets` body
(`lessons.rs:352-354`) and field note (`:359`): "Your own ship blocks part of
that arc, so a turret cannot shoot a target hidden behind your drive." Wrong on
both halves.
- The blind zone is the elevation hinge's fixed floor, 10 degrees below level
  (`TURRET_DEPRESSION_LIMIT = PI/18`,
  `crates/nova_authoring/src/base_content/sections/standard.rs:106`).
  `TurretSectionArc::bears_on` tests elevation against that band and nothing
  else (`crates/nova_ship/src/sections/turret_section/arc.rs:94-101`). No
  geometry of your ship is consulted.
- Your hull is transparent to your own rounds by design: "The firing body is
  transparent because the muzzle sits on its own hull"
  (`crates/nova_gameplay/src/rounds.rs:1172-1188`). A target behind your drive
  is shot straight through the drive.
VERIFIED in-session, both halves.

**C6. Selecting a map contact sets no mark, and ORBIT has none to set.**
`novaos_contacts` (`lessons.rs:498-499`): "Selecting one here sets the same
mark the flight computer uses for GOTO and ORBIT." Selection sets no lock; the
`map_goto` verb inserts `Autopilot::engage(AutopilotAction::Goto { target })`
directly, bypassing the travel lock (`crates/nova_os_ui/src/map/app.rs:71-77`).
And ORBIT takes no mark at all (C2).

**C7. Killing the drives does not stop a ship.** `combat_components` body
(`lessons.rs:334-336`) and field note (`:341`): "Step it onto the drive to stop
the ship moving." This contradicts `flight_momentum` two categories earlier
("Nothing in space does", `:207-209`) and the physics. Drives are thrust; the
hull keeps its velocity. The section that ends maneuvering is the CONTROLLER -
losing the last one leaves "a drifting, tumbling wreck" (`controller.md:21`).

**C8. You cannot rebind every control in Settings.** `advanced_bindings`
(`lessons.rs:546`). Two rows are read-only by design: Esc and the pad's pause
chord, "the way out of every other screen, including this one"
(`settings.md:103-104`). And a section's weapon or thruster trigger is not in
Settings at all - it is per ship, set in the editor or the NOVA OS SHIP app
(`settings.md:106-107`, `nova-os.md:279-285`). Suggested replacement: "You can
rebind the flight, targeting, camera and interface controls in Settings. Esc is
fixed, and a section's own trigger is set per ship."

**C9. Plain mouse-look turns the ship.** `start_camera` (`lessons.rs:171`):
"You can turn the camera without turning the ship." In Normal mode the mouse IS
the helm - the camera rig's rotation output is written straight to the
controller (`crates/nova_ship/src/input/player/intent.rs:21-33`). The hull
holds still only while free look or the combat stance is held
(`crates/nova_ship/src/camera/mode.rs:84-121`). The lesson already names
`free_look` in its chips (`:174`); the body has to name it too.

### 1.2 The seven that hide the rule which decides the outcome

**C10. STOP does not reach zero.** `flight_stop` (`lessons.rs:226`, field note
`:232`): "thrusts until your speed reads zero." A residual the drive is already
pointing at is braked to 2 m/s (`stop_speed_epsilon: 0.2` u/s,
`crates/nova_ship/src/flight/state.rs:462`); one it is not - a sideways drift -
is handed back at up to 7.5 m/s (`settle_deadband: 0.75` u/s, `:478`) with the
hull never turning. The wiki says it in as many words: "'At rest' is a deadband
and not a dead stop" (`flight-autopilot.md:38`).

**C11. GOTO does not stop "there".** `flight_goto` (`lessons.rs:261`, field
note `:268`). It stops one standoff - 500 m by default - off the target's
SURFACE, measured from your own hull face, not centre to centre
(`arrival_standoff: Meters(500.0)`, `state.rs:460`; `flight-autopilot.md:36`).

**C12. Taking the controls back does NOT turn the flight computer off - and
the wiki is wrong the same way.** `flight_orbit` (`lessons.rs:281`, field note
`:286`). Moving the mouse does not disengage: the manual rotation system is
gated `Without<Autopilot>` (`intent.rs:38-47`), so while the computer flies,
the mouse silently becomes camera-only. The complete disengage set is
`autopilot_off` (`flight_rig.rs:519-523`), a main-drive burn (`:352-357`), a
bound thruster key (`crates/nova_ship/src/input/player/weapons.rs:115-119`),
entering RCS (`flight_rig.rs:592-595`), pressing the same verb again, and - for
GOTO only - clearing the travel lock
(`crates/nova_ship/src/input/targeting/gesture.rs:165-171`).
**Fix alongside:** `flight-autopilot.md:34`, `glossary.md:18`,
`getting-started.md:46` and `keybinds.md:3` all say "a rotation" or "any manual
input" disengages.

**C13. A tap does not always clear the mark.** `combat_radar`
(`lessons.rs:298-300`). With weapons raised a tap only ever drops the COMBAT
lock, never the travel lock (`targeting-radar.md:68`). The lesson also names
only `radar_hold` in `actions` while `radar_clear` is a registered action
(`crates/nova_ship/src/input/bindings.rs:66`) and is what the sentence is
about.

**C14. "Well clear of you" is a number, and below it the shot is wasted.**
`combat_torpedoes` (`lessons.rs:372`). Arming needs BOTH the ordnance's own
condition (0.5 s or 50 m) and the launching hull's arm plus the warhead's full
blast radius, snapshotted at launch so shedding sections mid-flight cannot
shrink it (`crates/nova_ship/src/sections/torpedo_section/mod.rs:802-828`).
That is 324 m off a small boat and 494 m off a carrier. Inside it the torpedo
is inert and bounces off as a dud - "that distance is the shortest shot the bay
has" (`torpedo-bay.md:39`).

**C15. There is no frame, and some things ARE there only for looks.**
`build_sections` (`lessons.rs:388-390`): "sections bolted to a frame... nothing
on a ship is there only for looks." No frame exists anywhere in the code or the
wiki; sockets are "the sole source of structural adjacency"
(`crates/nova_editor/src/snap.rs:3-4`) and the hull is the backbone
(`hull.md:19`). And Ship Skin cladding is derived decoration that places no
section (`keybinds.md:488-494`). Invented vocabulary in the first Shipbuilding
lesson, plus a flat claim the editor contradicts.

**C16. `build_mass` gives no number.** "The same drive handles a light ship and
a heavy ship very differently" (`lessons.rs:405`) is the vague copy the brief
forbids, and "The clearest sign is how long STOP takes" is unfalsifiable. The
wiki has the measurement: skiff 21.00 mass pulls 61 m/s2, tug 41.00 pulls 31
m/s2, on the same two 1.0 drives (`thruster.md:24`).

### 1.3 Two more, already known

- `novaos_contacts`'s body still says "The contacts list shows what is known
  about each contact" while the captured art is now the map PLOT. Reword with
  C6.
- `advanced_scenarios`'s `wiki_path` is `#browsing-and-replaying-scenarios`
  (`lessons.rs:520`) while the body teaches the scenario/campaign distinction.
  The anchor resolves; the link and the body answer different questions.

### 1.4 Nothing catches these

`crates/nova_authoring/tests/lesson_wiki_links.rs` catches a renamed page or
heading. It cannot catch a false statement, and none of C1-C16 fails any test
today.

---

## 2. Coverage gaps by category

Verdicts: **must** - a player meets this in normal play and the handbook is
silent; **worth** - real, teachable, second rank; **wiki-only** - correctly
left to the manual.

### Start Here

| Wiki heading | What it carries | |
|---|---|---|
| `hud.md:54` flight readouts, keybind dock | The chip row showing the verbs the ship has RIGHT NOW, each with the player's real keycap, absent when the verb would do nothing, inverted while engaged. Every drill's design leans on it (`drills.rs:17-19`). | **must** |
| `hud.md:23` what is on screen, and when | The HUD is contextual; the HUD key cycles On and Cinematic; Cinematic clears every instrument. `start_hud` NAMES `hud_cinematic` in its chips (`lessons.rs:159`) and never says what it does. | **must** |
| `hud.md:58-60` sphere colours, mode chip | White/blue manual, cyan autopilot, violet RCS, yellow gravity; the `AP GOTO - BURN` verb-and-phase chip; shells sized to the live hull. | **must** (fold into `start_hud`) |
| `glossary.md:5` units | m under a kilometre, km above; m/s; one cell = 10 m. The handbook states the game's units nowhere. | **worth** |
| `hud.md:135` allegiance markers | Green ally, red hostile, grey neutral; your own shows none; a provoked neutral flips on the spot. | **worth** |
| `getting-started.md:36` pause | Esc pauses any scenario: Resume, Retry, Settings, Back to Main Menu, Exit. | **worth** |
| `getting-started.md:7-24` menu doors | New Game, Lessons, the field-note card and its Settings switches. | **worth** (fold into `start_welcome`) |
| `getting-started.md:40,51` first two minutes | The scenario is the teacher. | wiki-only |

### Flight

| Wiki heading | What it carries | |
|---|---|---|
| `gravity-wells.md:23` the pull | `a = mu / r^2`; mu is the body's authored number and never your mass; surface clamp; smoothstep fade over the outer 15% of the SOI (`gravity.rs:210-214`). **The handbook does not contain the word "gravity" anywhere** - `flight_orbit` only LINKS here. | **must** |
| `gravity-wells.md:43` sphere of influence | Reach follows from mass alone - the distance where raw pull decays to 2.5 m/s2; 4x mass buys 2x reach; outside it the well does not exist. | **must** |
| `gravity-wells.md:47` the dominant well | Overlapping SOIs do not blend; you feel only the strongest; the incumbent holds until a challenger beats it by 1.10x. | **must** |
| `flight-autopilot.md:24` manual flight | The optional soft speed cap - Basic Training's 150 m/s governor (`range.rs:85,139`), on TOTAL speed, tapering over the last fifth, never capping a braking burn (`manual.rs:74-77`). A player on every shipped range hits this wall. | **must** |
| `flight-autopilot.md:32` + widget `:41` | The arrival envelope: closing speed capped at what a flip-and-brake can cancel, flip one swing early, brake at 85% authority, 15 m/s floor, last metres on RCS. | **must** |
| `flight-autopilot.md:32` cancel | What actually disengages (C12). No lesson teaches CANCEL as its own idea. | **must** |
| `flight-autopilot.md:28` the hull decides handling | Turn authority is the lower of computer torque and the 8 G structural limit. Shared with Shipbuilding - see `build_turning`. | **worth** |
| `hud.md:109` + `keybinds.md:103` docking sight | DOCK clamps hull to hull; the sight draws inside 80 m. `dock` is a registered flight action (`bindings.rs:55`). | **worth** |
| `thruster.md:33` a hurt drive | A damaged thruster delivers exactly the push a fresh one does; the plume lies. | wiki-only |

### Combat

| Wiki heading | What it carries | |
|---|---|---|
| `railgun.md` - all seven headings, 231 lines | **The whole weapon has no lesson.** No traverse, the hull is the aim; 13 sockets, none on the muzzle face; 1.5 s commit that only the safety aborts; 300 Pierce / 1800 power / 15,000 m/s / 18 km; recoil 45 at the muzzle - a shove on the spine, a shove AND a yaw off it; one shell, a shot every 13.5 s. | **must** |
| `combat-weapons.md:65` cover and line of fire | The first tangible thing eats the round; a hostile that loses the line loses its PICK and reverts to its passive routine. Cover is a full disengage. | **must** |
| `combat-weapons.md:107` damage types | Kinetic budget vs Pierce power. Not a multiplier: both hit a hull and a drive for the same number. | **must** |
| `combat-weapons.md:139` magazines | A rate limit, not a budget. Nothing runs out for good. | **must** |
| `combat-weapons.md:153` your own battery | The flight computer works your idle mounts against inbound torpedoes, with no toggle and no key; you always win the argument. | **must** |
| `combat-weapons.md:149` point defense | Per-mount picks; saturate a facing. | **must** |
| `ships.md:27` taking a ship apart | The 1/20 structural collapse threshold (`DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD = 0.05`, `integrity.rs:29`); NEUTRALIZED keeps the hull and stops defending itself. | **must** |
| `targeting-radar.md:72` line of sight | Cover drops BOTH lock slots; nothing re-locks itself; an engaged GOTO is the exception. | **must** |
| `targeting-radar.md:66` clearing locks | A tap clears in stages, and only the combat lock while raised (C13). | **must** |
| `turret.md:27` barrel discipline | The fire gate is the target's ANGULAR SIZE - 11 deg for a carrier at 1 km, 0.7 deg for a torpedo at 800 m; a slewing mount holds fire. | **must** |
| `turret.md:61` reach and closing speed | Everyone's gun reaches 2 km; the AI closes to ~1 km and holds fire until inside it. | **must** |
| `factions.md:5,22` allegiance | Player/Enemy/Neutral -> Own/Hostile/Neutral; a round copies its shooter's side at launch and keeps it; AI standoff measured hull to hull; a ship with dead engines fights on straight. | **must** |
| `torpedo-bay.md:45` the two run-ins | Serpent 320 m/s weaving vs Lance 350 m/s straight; 390 vs 116 defending rounds. | **must** |
| `combat-weapons.md:23` three reaches | 2 km guns, 18 km railgun, 29-31 km torpedoes; reach is muzzle speed x lifetime. | **worth** |
| `combat-weapons.md:117` closing speed | 1,000 m/s = 1.0x; head-on doubles Kinetic and trebles Pierce depth; tail chase floors at 1/4 and 1/2. | **worth** |
| `targeting-radar.md:80` lock ranges | 30x signature under a 200 km ceiling: 9 / 22 / 60 km. A ship you shoot apart goes quieter. | **worth** |
| `turret.md:116` stowed between fights | Deploy is fast, stow is lazy; the first trigger pull on a cold ambush raises the guns instead of firing. | **worth** |
| `combat-weapons.md:83` shooting rock | Real carving; rounds within ~10 m deepen the same crater. | **worth** |
| `ships.md:39` what damage looks like | Cracks and sparks read history, not capability. | **worth** |

### Shipbuilding

Two structural facts first: **the whole category has `practice: None` and
`proven_by: []`** (`lessons.rs:379-452`) - nothing in the game can prove or
practise any of it. And **the editor registers no actions at all** (no
`ActionBinding::new` anywhere in `crates/nova_editor/src/`), so an editor
lesson can name neither a key (rule 1 of the handbook) nor an action id (rule
2). That is an owner decision, not an authoring problem - see section 5.

| Wiki heading | What it carries | |
|---|---|---|
| `controller.md` - whole page | **No lesson.** Required for any flyable ship; the last one lost is disabled, not destroyed; turn rate is the LOWER of computer torque and the 8 G ceiling at the furthest section's face - the base gunship is structure-bound at 1.42 rad/s2 where its computers could push 8.74; a wreck turns sharper; stacking buys precision and redundancy, not rate. | **must** |
| `docking.md` - whole page | **No lesson.** 10 m face gap / 15 deg / 5 m/s relative, roll ignored; offered against the TRAVEL lock; modal - drive, helm and RCS dead while clamped; either ship releases. **No base hull mounts one, so a ship that can dock is one you built.** | **must** |
| `keybinds.md:351` editor | Placement, socket cycling, roll, and six refusal reasons including a blocked muzzle or exhaust (`crates/nova_editor/src/snap.rs:30-47`). No lesson names a single editor gesture. | **must** |
| `sections.md:29` what every section shares | The 10 m grid (a bay is 2 cells, a railgun 3); mass is the collider box at density 1 and **nothing authors it**; faces decide seating; a part does not degrade. | **must** |
| `thruster.md:19-27` | Thrust is analog, authority = magnitude x count; the 640 m/s2 ceiling; **a thruster bolts on by its forward face only** - you pick the face it grows from. | **must** |
| `turret.md:49` what it can bear on | Unlimited traverse, barrel floor 10 deg below level = 58.7% of the sky, blind cone under the keel; 180 deg/s on both hinges; 2 km reach. | **must** (the current lesson is C5) |
| Generate - `crates/nova_editor/src/generate.rs:1-18` | Seed plus ticked kinds, rolls a whole hull into the ship you are inside and replaces its contents; the result is ordinary nodes, already key-bound. **Also a wiki gap: no page documents it.** | **must** |
| The build readout - `crates/nova_editor/src/readout.rs:1-9` | Mass, summed thrust, summed health, part count, attitude envelope, plus the remedy note. **Also a wiki gap.** | **worth** |
| `hull.md:42` variants | Health ranked by mass inverts the catalog - 17 vs 1040 per unit mass. | **worth** |
| `thruster.md:53` variants | Thrust per mass FALLS with grade: 1.0 -> 0.5 -> 0.33. | **worth** |
| `keybinds.md:465` the inspector | Drag-the-number's-NAME grips, units printed per row, typed values refused out of range. | **worth** |
| `keybinds.md:488` ship skin | Derived, never placed; re-derived live; carries into Play. | **worth** |
| `sections.md:41` the catalog at a glance | Non-unit mounts: PDC 0.5-cell at 1/8 mass, bay 2 cells, railgun 3. | **worth** |

### NOVA OS

| Wiki heading | What it carries | |
|---|---|---|
| `nova-os.md:118` command reference | The whole registered set: `help`, `log`, `objectives`, `clear`, `version`, `exit`, `commands`, `map`, `map view`, `map goto`, `ship`, `ship view`, `ship section`, `ship reload`, `ship repair`. **The handbook names zero commands.** | **must** |
| `nova-os.md:61` the terminal | The `nova>` prompt, Tab completion over live section and contact codes, 200-line history, 500-row scrollback, the ghost suffix, the boot report. Nothing says NOVA OS has a prompt you type at. | **must** |
| `nova-os.md:224` the ship | Section codes (`HULL-1`, `THR-1`, `CTL-1`, `PDC-1`, `TRB-1`), status-coloured blips, the inspector with its ASCII integrity meter, and the repair / reload / rebind verbs. `novaos_view` covers turning the model only. | **must** |
| `nova-os.md:266` rebinding a section | Thrusters, turrets and tubes fire on per-ship triggers Settings does not list; a reserved flight control is refused by name. This is the other half of C8. | **must** |
| `nova-os.md:182` map goto | `G` engages the flight autopilot on the selection, and the burn keeps flying after the computer closes (`map/app.rs:71-84`). The fact that makes the map a navigation console. | **must** (fold into `novaos_contacts` with C6) |
| `commands.md` - whole page | The `:` command shell: a second vocabulary on the same monitor, four permission classes, and cheats that mark the run for good. | **worth** |
| `nova-os.md:167` apps | An app swallows the monitor; Esc backs out one level and leaves the scrollback intact. | **worth** (fold into `novaos_view`) |
| `nova-os.md:287` the monitor | BRIGHT / SCAN / SND / PWR chin controls. | wiki-only |

### Advanced

| Wiki heading | What it carries | |
|---|---|---|
| `settings.md:109` mouse | Three sliders and no bindings: Look 100-300% default 200% (steering, free look, turret aim); RCS 100-500% default 100%; Free Camera 100-300%. Each 100% is its own baseline; the gamepad is untouched. | **must** |
| `settings.md:86-101` rebinding | One group at a time; `PRESS A KEY`; refusal by name ("W is already bound to Main Drive"); sharing a key across screens is deliberate. | **must** (fold into `advanced_bindings` with C8) |
| `getting-started.md:20` mods disabled | A mod whose content will not load is switched off FOR the player and named in one notice on the front door, files left installed so it can be updated or re-enabled. The failure path a modded player actually meets. | **must** (fold into `advanced_mods`) |
| `factions.md:5,22` | See Combat - the relation model and AI flight. Place in whichever category ships first. | **must** |
| `settings.md:33` graphics quality | Low/Medium/High: camera shake, hit flashes, particle bursts, 3D-world resolution reduced and upscaled at Low. | **worth** |
| `settings.md:18` audio | Four sliders; what Interface vs World mean; Master multiplies. | **worth** |
| `scenarios.md:5,20` what a scenario places | The six object kinds plus trigger areas; the events/filters/actions model over typed variables. | **worth** (fold into `advanced_scenarios`) |
| `getting-started.md:91` the sandbox | Editor then free-flight range: belts, hulks, pickets that wake on a combat lock, a planetoid. | **worth** |
| `scenarios.md:28` the shipped scenarios | Basic Training plus four backdrop scenes. | wiki-only |
| `settings.md:59` window | Borderless default; the web build has no row. | wiki-only |

---

## 3. Detail gaps, lesson by lesson

Each entry quotes the shipped body first, then lists what a technical reader
needs that it does not carry. Corrections are cross-referenced, not repeated.

### Start Here

**`start_welcome`** (`lessons.rs:137-139`) - "Each lesson is one screen: a
picture or a short loop, a few lines of text, and the controls it uses. Some
lessons open a practice flight. Every lesson links to a longer page on the
wiki."
Missing: that the chips are LIVE bindings, not fixed keys - the one guarantee
the whole format rests on (`catalog.rs:186-191`), and the reader is never told
the keys shown are theirs. Missing: read vs done - opening marks read, done
needs a won scenario that proves it (`getting-started.md:18`,
`Lesson::proven_by`). Missing: that progress is kept between sessions, and that
field notes are these same claims resurfaced on the menu card.

**`start_hud`** (`:155-158`) - "The HUD only shows what you need right now.
Your speed and a velocity sphere sit around your ship. The sphere points the
way you are actually moving, which is not always where the nose points."
Missing: the colours ARE the state readout - white/blue manual, cyan autopilot,
violet RCS (violet wins), yellow for the local gravity pull, hidden in flat
space (`hud.md:58`). Missing: the mode chip `AP GOTO - BURN`, verb and phase,
up only while engaged (`hud.md:34,60`). Missing: the shells are sized to the
live hull, settling back over ~0.2 s as sections die. Missing: the unit (m/s).
And it names `hud_cinematic` in its chips without ever mentioning it - either
explain it or move the chip to a lesson that does.

**`start_camera`** (`:171-173`) - see C9. Also missing: the three camera modes
and what picks them (`mode.rs:191-215`), and that only Normal steers the hull.

### Flight

**`flight_aim`** (`:186-191`) - "The main drive only pushes in the direction
the nose points..."
Missing: the main drive is not one engine - it is the sum of every thruster
pointing forward, with the flight computer setting each throttle to cancel the
twist through the live centre of mass (`thruster.md:31`,
`crates/nova_ship/src/flight/thrusters.rs:305`). Missing: throttle is analog
and spools - 6.0 up, 10.0 down per second (`state.rs:456-457`), so engines cut
faster than they light. Missing: **no steering control is named at all**
(`actions: &["main_drive"]`) in a lesson titled "turn, then thrust". Missing:
turn rate is clamped 10-240 deg/s, so on a heavy hull the heading you are
leaving lasts seconds.

**`flight_momentum`** (`:205-209`) - "Releasing the drive does not slow you
down. Nothing in space does."
Missing: the two exceptions the player is flying under right now - the 150 m/s
soft cap on every shipped range (`range.rs:85,139`), which contradicts
"nothing" on screen the first time a held burn levels off, and gravity inside a
SOI. Also: `proven_by` lists only the drill while `flight_aim` lists the
tutorial too, though Basic Training Part 1 teaches both.

**`flight_stop`** (`:224-227`) - see C10. Also missing: STOP budgets for
gravity along your velocity and refuses when the pull eats the whole brake
authority (`autopilot.rs:445`); it brakes on RCS without turning below 100 m/s
on an RCS-granted hull; and the key TOGGLES - pressing it again disengages, and
STOP overrides any other engaged maneuver (`flight_rig.rs:385-396`).

**`flight_rcs`** (`:243-245`) - "Hold the RCS modifier and the mouse drives the
small thrusters..."
Missing: the numbers - 100 m/s cap in any direction and a mass-independent 5 g,
with diagonal input sharing one speed and one acceleration budget across all
three axes (`state.rs:488-489`, `manual.rs:39-66`). Missing: the third axis -
the scroll wheel drives ship-local up and down while RCS is held
(`flight_rig.rs:1256-1262`); "sideways" hides fore/aft and vertical. Missing:
RCS is a GRANTED capability the mainline campaign withholds. Missing: entering
RCS disengages the autopilot and freezes helm and camera. Missing: the sphere
turns violet. `field_notes` is empty, and the 100 m/s cap is exactly the
two-line fact a field note wants.

**`flight_goto`** (`:261-263`) - see C11. Also missing: it needs a TRAVEL lock
specifically - a red combat lock is not flown (`flight_rig.rs:437-444`), which
is the entire reason `drill_autopilot` exists. Missing: the target is captured
at the press, so re-designating afterwards does not re-route the leg, but
CLEARING the designation does disengage. Missing: it tracks a drifting target
and has no collision avoidance. Missing: the key toggles.

**`flight_orbit`** (`:279-281`) - see C2, C3, C12. Also missing: orbital speed
is `v = sqrt(mu / r)`, held with micro-burns; the ring is clamped into a stable
band - never closer than 1.5x the surface clearance, never beyond 90% of the
fade start - and ORBIT DISENGAGES rather than fly a well with no stable band
(`autopilot.rs:345-346`); it never self-completes; the HUD draws a world-space
ring and a radius spoke while you hold it.

### Combat

**`combat_radar`** (`:298-300`) - see C13. Also missing: **there are two lock
slots, not one mark** - lowered writes the white travel lock, raised writes the
red combat lock (`targeting-radar.md:59-60`), and the body's "autopilot orders
and weapon locks both work from the contact you mark here" implies one lock
feeding both. Missing: the 18-degree cone. Missing: the dwell - 0.25 s
tap/hold threshold, then 0.6 s point-blank stretching to 1.5 s at 20 km.
Missing: line of sight is required and cover drops both slots, which is the
single most confusing radar behaviour. No field notes.

**`combat_stance`** (`:315-318`) - see C4. What it should say instead is on the
page it already links to: the stance picks the lock slot, weapons are hot while
raised or while a combat lock exists, it is a HELD stance, it moves the mouse
from the helm to turret aim, and it takes the battery back from the flight
computer's point defense.

**`combat_components`** (`:334-336`) - see C7. Also missing: the prerequisite -
you must hold a combat lock focused for ~1.5 s before you can drill in
(`targeting-radar.md:64`). Missing: fine-lock either snaps to the crosshair or
is pinned by cycling, and a manual pin holds only a couple of seconds. Missing:
turrets AND the viewfinder follow the fine-locked section - the reason to use
it at all.

**`combat_turrets`** (`:352-354`) - see C5. Also missing: unlimited traverse, a
10-degree depression floor, 58.7% of the sky, 180 deg/s on both hinges so a
90-degree swing costs 0.5 s and 50 unfired rounds, 2 km reach. Missing: barrel
discipline - the fire gate is the target's angular size, so a battery is looser
on a freighter than on a torpedo. **Structural:** `practice: None` but
`proven_by` lists the gunnery drill - provable by a range the lesson gives the
player no button to reach.

**`combat_torpedoes`** (`:370-372`) - see C14. Also missing: "a moment later"
is 0.6 s, and during that window the torpedo is cargo - no thrust, no guidance,
no fuze, unshootable. Missing: it becomes shootable once the drive lights (10
ordnance hp). Missing: the Serpent/Lance split. `proven_by: []` and `practice:
None` - nothing can prove it, and the gunnery trainer carries no bay.

### Shipbuilding

**`build_sections`** (`:388-390`) - see C15. Also missing: the 10 m grid.
Missing: WHY mass differs - it is the collider box at density 1
(`crates/nova_ship/src/sections/base_section.rs:46-50`). Missing: faces - parts
seat so their working end points out. Missing: **the seven section kinds are
never listed**, though the category exists to teach them. No field notes,
unlike every other category opener.

**`build_mass`** (`:405-407`) - see C16. Also missing: the tapering return -
each added drive is mass as well as push, and no stack passes 640 m/s2.
Missing: a weapon mount is cheap mass (PDC 0.125) while a cargo hull is
expensive (1.0), which is the actual build decision. Its `wiki_path` points at
`sections#what-every-section-shares` when every number it needs is on
`sections/thruster`.

**`build_balance`** (`:425-427`) - accurate against `thruster.md:31`. Missing:
off-axis thrusters are recruited purely for COUNTER-TORQUE when the firing set
cannot balance itself. Missing: the balance runs through the LIVE centre of
mass, which is why it survives losing sections. Missing: **a thruster bolts on
by its forward face only** - the placement rule a builder needs, nowhere in the
handbook. Missing: a damaged drive delivers full push; the guttering plume is
cosmetic. The field note drops the "less thrust" half entirely.

**`build_flight_test`** (`:445-447`) - "Test a new ship before you use it..."
This is advice, not a fact: no number, no unit, no name. Missing: the **Play**
button that does exactly this (`crates/nova_editor/src/ui/mod.rs:378,626`),
that Play is disabled while you are inside a ship, and what the sandbox holds -
target hulks and three dormant pickets, the farthest carrying the spinal lance
(`crates/nova_editor/src/scenario.rs:17-19`). Missing: turning, which is the
half of handling the hull shape decides most.

### NOVA OS

**`novaos_open`** (`:459-461`) - see C1. Also missing: the cursor is freed, so
you point and click. Missing: **Tab does not close it** - inside, Tab is the
completion key; Esc backs out one level, Shift+Esc powers off from anywhere,
Ctrl+C leaves an app but keeps the computer on. Missing: the session persists
across closes - scrollback, history and a running app come back as left.
Missing: opening marks posted objective chips read.

**`novaos_view`** (`:478-481`) - see C1. Also missing: the keys the app
actually publishes - Q/E turn, R/F tilt, right-drag look, wheel zoom, `[`/`]`
select, T reset, all registered as `novaos_*` actions
(`crates/nova_os_ui/src/bindings.rs:54-92`); the lesson names only
`novaos_reframe`. Missing: what the schematic TELLS you - one dim-green block
per section in a bright outline, status on the blips not the block colour, and
only the selected section spelling out its glyph and code. Missing: the
inspector carries HP, ammo and current bindings.

**`novaos_contacts`** (`:497-499`) - see C6 and 1.3. Also missing: the labels
(`SELF`, `ALLY-1`, `HOST-1` with a pulsing blip, `OBJ-1`, `AST-1`). Missing:
the readout shape - "HOSTILE HOST-1 / Raider - range 412 m, bearing 214 mark
+12." Missing: `map view` prints the same contacts as a table.

### Advanced

**`advanced_scenarios`** (`:516-518`) - missing what a scenario places, the
events/filters/actions model, and what the picker looks like (campaigns fold
under a header in play order; a base install lists Basic Training alone). Its
own anchor is about browsing and replaying and the body says nothing about
either.

**`advanced_mods`** (`:532-534`) - missing the failure path a modded player
actually meets (`MODS DISABLED` on the front door, files left installed).
Missing: the online catalog browser. Missing: **lessons themselves are mod
content** (`crates/nova_training/src/catalog.rs:4-8`) - the one self-
referential fact the Advanced category is placed to carry.

**`advanced_bindings`** (`:546-548`) - see C8. Also missing: how rebinding
works - one group at a time, `PRESS A KEY`, refusal by name, `Reset Defaults`.
Missing: sharing a key across screens is deliberate (`G` is GO TO in flight and
the mates overlay in NOVA OS, and only one is listening). Missing: the MOUSE
sliders entirely.

---

## 4. Proposed lessons

Forty candidates. Every body is inside the 45-word cap
(`crates/nova_training/src/catalog.rs:148`), every `actions` entry is a
registered id, and every `wiki_path` anchor resolves under the slug rule
(`crates/nova_authoring/tests/lesson_wiki_links.rs:29-41`). Orders slot into
the existing spacing without renumbering anything.

Tier **A** is the release set: it closes a must-have gap or replaces a wrong
claim. Tier **B** is real and teachable, second rank. A body marked `[draft]`
is the reviewer's wording, offered as a starting point, not as final copy.

### Start Here

| Tier | id / order / title | Body `[draft]` | actions | wiki_path | practice |
|---|---|---|---|---|---|
| A | `start_verbs` 25 *The keybind dock* | "The row of chips along the bottom shows the verbs this ship can use right now - STOP, GOTO, ORBIT, CANCEL, RADAR, COMPONENT, RCS and DOCK. Each prints your own keycap. A verb the ship does not carry is not on the row at all." | none (naming eight verbs would draw a wall of chips) | `wiki/hud#flight-readouts` | `drill_autopilot` |
| A | `start_cinematic` 40 *Two HUD levels* | "The HUD is contextual: a quiet cruise draws the velocity sphere, your speed and the dock's live verbs, and everything else arrives with its moment. The HUD key cycles On and Cinematic, a clean screen for captures. It clears every instrument and chip." | `hud_cinematic` | `wiki/hud#what-is-on-screen-and-when` | none |
| B | `start_units` 15 *Distances and speeds* | "Ranges read in meters under a kilometer and in kilometers above it; speeds read in m/s. The build grid counts in cells: one cell is 10 m on a side. Every readout on the HUD and in NOVA OS uses those units." | none | `wiki/glossary#units` | none |
| B | `start_markers` 35 *Who is who* | "A small filled triangle floats over every ship in view: green for your allies, red for hostiles, grey for neutral bystanders. Your own ship shows none. A neutral provoked into a threat flips its marker red on the spot." | none | `wiki/hud#allegiance-markers` | none |
| B | `start_pause` 45 *Pausing and retrying* | "Esc pauses any scenario and offers Resume, Retry, Settings, Back to Main Menu and Exit. Retry restarts the scenario you are on with the same ship. Settings is the same modal the main menu opens, so you can change bindings without leaving the flight." | none - **Esc is deliberately unbindable** and the pause toggle is a raw keycode (`crates/nova_menu/src/pause.rs:91-96`) | `wiki/getting-started#launch-and-start` | none |

If `start_cinematic` lands, drop `hud_cinematic` from `start_hud`'s actions
(`lessons.rs:159`).

### Flight

| Tier | id / order / title | Body `[draft]` | actions | wiki_path | practice |
|---|---|---|---|---|---|
| A | `flight_gravity` 45 *Gravity wells* | "A planetoid pulls you inward at a = mu / r^2. That is an acceleration, so a laden hauler falls as fast as a stripped fighter. The pull reaches to the sphere of influence, where it decays to 2.5 m/s2, and is zero beyond." | none | `wiki/gravity-wells#sphere-of-influence` | `drill_autopilot` (its range spawns the planetoid) |
| A | `flight_cancel` 35 *Taking the ship back* | "CANCEL drops any engaged maneuver and hands you a ship that is already moving. A main-drive burn, a bound thruster key or entering RCS does the same. Moving the mouse does not: while the computer is flying, the mouse is camera only." | `autopilot_off`, `main_drive`, `rcs_modifier` | `wiki/flight-autopilot#the-autopilot-flies-the-hull` | `drill_stop` |
| A | `flight_arrival` 55 *The arrival envelope* | "GOTO obeys one rule: at any distance it caps closing speed at what a flip-and-brake from there can still cancel. It flips one swing early, brakes at 85% of the drive's authority down to a 15 m/s floor, and settles the last meters on RCS." | `autopilot_goto` | `wiki/flight-autopilot#the-autopilot-flies-the-hull` | `drill_autopilot` |
| B | `flight_handling` 15 *How hard your hull turns* | "Nothing authors handling. A hull turns at the lower of two limits: what its flight computers can twist against its mass, and what its metal survives at 8 G out at the furthest section. A wreck turns sharper than the whole ship did." | none | `wiki/flight-autopilot#the-hull-decides-the-handling` | `drill_momentum` |
| B | `flight_dock` 65 *DOCK* | "DOCK clamps your hull to the ship you hold a travel lock on, port face to port face - both hulls need a docking port. Inside 80 m the docking sight draws a cross on each port and a ticked line between them." | `dock` | `wiki/hud#docking-sight` | **none possible** - no shipped hull carries a port |

`flight_handling` and `build_turning` are the same fact from two sides. Ship
one, not both.

### Combat

| Tier | id / order / title | Body `[draft]` | actions | wiki_path | practice |
|---|---|---|---|---|---|
| A | `combat_railgun` 70 *The hull is the aim* | "A railgun has no traverse: it fires down its own axis, so the face you bolted it to is the line. A tap starts a 1.5 second charge; only the weapons safety stops it. The gun never re-checks the nose." | none - the railgun trigger is a per-section binding with no action id | `wiki/sections/railgun#committing-the-shot` | none |
| A | `combat_cover` 25 *Cover and the firing line* | "A round expends itself on the first solid thing it meets, and a rock on the line blocks the radar too. A hostile that loses the line loses its pick and goes back to its passive routine, so breaking line of sight is a disengage." | none | `wiki/combat-weapons#cover-line-of-fire` | `drill_gunnery` seeds rock belts, but nothing shoots back |
| A | `combat_damage_types` 35 *Kinetic and Pierce* | "A damage type is not a multiplier: both hit a hull and a drive for the same number. Kinetic carries on only through what it destroys. Pierce deals full damage to every section it crosses, paying a separate power budget for each layer's full health." | none | `wiki/combat-weapons#damage-types` | **none** - the trainer mounts only a kinetic PDC |
| A | `combat_magazines` 45 *Reloading magazines* | "Weapons automatically refill their magazines after a short time without firing. Firing again restarts the wait. Long bursts can empty a magazine temporarily, but ammunition never runs out permanently." | none | `wiki/combat-weapons#magazines` | `drill_gunnery` - **fits**, the trainer's gun carries the authored `Limited(500)` magazine |
| A | `combat_point_defense` 60 *Your battery defends itself* | "While you hold no combat lock and your weapons are lowered, the flight computer puts your idle mounts onto inbound torpedoes and fires them; a thin line shows each mount and its pick. Lock or raise and every mount is yours that instant." | `combat_stance` | `wiki/combat-weapons#your-own-battery` | none - nothing on the gunnery range launches torpedoes |
| A | `combat_collapse` 90 *When a ship comes apart* | "You do not have to shoot every section off. A hull carrying less than a twentieth of the structure it was built with collapses. A ship that has lost every weapon, or its last flight computer, is NEUTRALIZED: it keeps its hull and stops answering." | none | `wiki/ships#taking-a-ship-apart` | `drill_gunnery` - its win condition puts collapse on screen |
| A | `combat_allegiance` 15 *Who shoots whom* | "Every ship is Player, Enemy or Neutral, and any pair resolves to Own, Hostile or Neutral. A round copies its shooter's side at launch and keeps it, so your own ordnance never hits you. A small triangle over each ship reads green, red or grey." | none | `wiki/factions#the-relation-model` | none - the gunnery range has no neutrals |
| B | `combat_torpedo_types` 55 *Serpent and Lance* | "Two normal bays ship and only the run-in differs. A Serpent weaves, so one defending PDC spends 390 rounds and only kills it 400 m out. A Lance flies straight and faster, and the same mount kills it 1.14 km short of you." | none | `wiki/sections/torpedo-bay#the-two-run-ins` | none - no trainer carries a bay |
| B | `combat_bore_sight` 80 *The bore sight* | "The flight computer draws a blue line from the muzzle to where the slug would stop, with a ring on every section that shot would destroy. Aiming down a ship's long axis reads differently from catching its shoulder. The line thickens as the charge runs." | none | `wiki/sections/railgun#the-bore-sight` | none |
| B | `combat_lock_ranges` 12 *How far you can lock* | "How far you can lock is what the target IS: thirty times its own radar signature, under a 200 km scanner ceiling. A bare one-cell hull is a contact at about 9 km, a light skiff at 22 km, the largest carrier at 60 km." | `radar_hold` | `wiki/targeting-radar#lock-ranges` | `drill_gunnery` (loose fit) |

Two more worth specifying if the owner wants the railgun family taught before
the ladder that places it: `combat_reaches` (2 km / 18 km / 29-31 km,
`wiki/combat-weapons#three-reaches`) and `combat_barrel_discipline` (the
angular-size fire gate, `wiki/sections/turret#barrel-discipline`).

TAKEN. The owner approved both, and both ship:

| Tier | id / order / title | Body | actions | wiki_path | practice |
|---|---|---|---|---|---|
| - | `combat_reaches` 32 *How far each weapon reaches* | "Reach is never authored. It is muzzle speed times how long the round lives. A PDC round reaches 2 km, a railgun slug 18 km and arrives in about a second, a torpedo 29 to 31 km over a minute and a half." | none | `wiki/combat-weapons#three-reaches` | none |
| - | `combat_barrel_discipline` 42 *Barrel discipline* | "A gun fires only while its barrel is on what it is aimed at, and the tolerance is how big that thing looks from here: a carrier at 1 km is 11 degrees wide, a torpedo under one. Mounts hold while they slew." | none | `wiki/sections/turret#barrel-discipline` | none |

One producer shoots both, `examples/screenshots/lesson_combat_reach.rs`: the
reach still is a broadside of a gunship's full battery with the tracer streams
stopping 700 m short of the hostile hull they are laid on and that hull's
torpedoes already past the point the rounds stop, and the discipline loop steps
the commanded bearing 90 degrees twice a cycle and lets the sheet show the
battery go quiet while the barrels chase it.

### Shipbuilding

| Tier | id / order / title | Body `[draft]` | actions | wiki_path | practice |
|---|---|---|---|---|---|
| A | `build_turning` 35 *What decides your turn rate* | "A ship turns at the lower of two ceilings: computer torque against its mass, and what 8 G allows at the furthest section's face. The base gunship is structure-bound - a 55.2 m arm caps it at 1.42 rad/s2, where its computers could push 8.74." | none | `wiki/sections/controller#what-sets-how-hard-a-ship-turns` | none |
| A | `build_faces` 15 *Sockets, not glue* | "A part mates at a socket, and sockets are the only structural link between sections. Cycle which socket mates and roll the part about that axis before you click. The editor refuses an occupied or ambiguous socket, an overlap, or a blocked muzzle or exhaust." | **blocked** - the editor registers no actions (section 5) | `wiki/keybinds#editor` | none |
| A | `build_weapon_mounts` 60 *Where a weapon can point* | "A turret traverses freely but its barrel stops 10 degrees below level, so each mount owns 58.7 percent of the sky and a blind cone under its keel. A railgun does not traverse: the face you bolt it to is the line it fires down." | none | `wiki/sections/turret#what-it-can-bear-on` | none |
| A | `build_dock_envelope` 75 *Making a dock* | "Three things must be true at once: the two port faces within 10 m, the ports within 15 degrees of opposed, and under 5 m/s of relative motion. Roll is ignored. While the clamp holds, your drive, helm and RCS are dead." | `dock` | `wiki/sections/docking#flying-the-approach` | none - `docking_approach` is an example, not a `role: Lesson` scenario |
| A | `build_docking_port` 70 *Bolting on a docking port* | "No base hull carries a docking port, so a ship that can dock is one you built. The port costs 90 health, the lightest part on a hull, and takes neighbours on every face but the hatch it docks through." | none | `wiki/sections/docking#variants` | none |
| A | `build_generate` 90 *Generate a hull* | "Generate rolls a whole hull into the ship you are inside, from a seed you can type or reroll, using only the section kinds you ticked. It replaces what that ship holds. The result is ordinary section nodes, already bound to their kind's key." | none | **blocked** - no wiki page documents Generate (section 5) | none |
| B | `build_stacking` 50 *A second flight computer* | "A second flight computer adds no turn rate to a hull already held by its structure, which is every base hull but the carrier. A stack buys precision - the heavy barge loses its 6.7 degree overshoot - and redundancy. Only the last computer matters." | none | `wiki/sections/controller#stacking-controllers` | none |
| B | `build_readout` 25 *The build readout* | "The rail prints four sums as you place: mass, summed thrust, summed health and section count, plus the turn rate they give. Mass is the volume of the box, one per 10 m cell, so the numbers move the moment a part lands." | none | **blocked** - the readout has no wiki heading (section 5) | none |
| B | `build_skin` 80 *Ship skin* | "Ship Skin dresses the build in the cladding it would fly with. Nothing places a plate: the skin is derived from the structure and re-derived as you build, including around the part in hand. A refused placement stays bare. The toggle carries through to Play." | none | `wiki/keybinds#the-inspector` | none |

### NOVA OS

| Tier | id / order / title | Body `[draft]` | actions | wiki_path | practice |
|---|---|---|---|---|---|
| A | `novaos_commands` 25 *What you can type* | "`help` lists everything. `log` prints the flight log, `objectives` the open objectives, `clear` wipes back to the boot report. `map` and `ship` hand the screen to an app; `map view` and `ship view` print the same data as a table instead. `exit` returns to flight." | none | `wiki/nova-os#command-reference` | none |
| A | `novaos_terminal` 15 *The prompt* | "The prompt reads `nova>`. Tab completes command names, subcommands and live section or contact codes; repeated presses cycle the matches. Up and Down walk the last 200 lines of history, PgUp and PgDn scroll 500 rows of scrollback. An unknown word turns the input red." | none - terminal editing keys are handled inside the CRT, not as registered actions | `wiki/nova-os#the-terminal` | none |
| A | `novaos_service` 35 *Repair and reload a section* | "Sections carry short codes - HULL-1, THR-1, CTL-1, PDC-1, TRB-1 - stable for the session and shared by the app and the prompt. Select one and repair restores its integrity; reload refills a weapon's magazine. Both are instant, and a hull section refuses reload." | `novaos_next`, `novaos_prev`, `ship_repair`, `ship_reload` | `wiki/nova-os#the-ship` | none - no drill damages the trainer on purpose |
| A | `novaos_rebind_section` 45 *Rebinding a section* | "Your thrusters, turrets and tubes fire on per-ship triggers that Settings does not list. Select the section in the SHIP app and press rebind; the next key or mouse button takes over its keyboard trigger and leaves the pad half alone." | `ship_rebind`, `novaos_next`, `novaos_prev` | `wiki/nova-os#rebinding-a-section` | none |
| B | `novaos_shell` 55 *The command shell* | "A second language runs on the same monitor. It opens with no ship, over the menu or the editor, and reads the run: `status`, `ships`, `bindings`, `settings`. Cheat commands are refused until `cheats enable`, which marks the run for good." | none - `:` is read as a logical character, not a rebindable action | `wiki/commands#the-commands` | none |

### Advanced

| Tier | id / order / title | Body `[draft]` | actions | wiki_path | practice |
|---|---|---|---|---|---|
| A | `advanced_mouse` 35 *Mouse sensitivity* | "The MOUSE group in Settings holds three sliders, not bindings. Look scales ship steering, free look and turret aim (100-300%, default 200%). RCS scales mouse translation (100-500%, default 100%). Free Camera scales the WASD camera. Each 100% is its own baseline; the gamepad is untouched." | none - sliders, not actions | `wiki/settings#mouse` | none |
| B | `advanced_factions` 15 *Sides* | Same ground as `combat_allegiance`. Ship one, in whichever category reads it first. | none | `wiki/factions#the-relation-model` | none |
| B | `advanced_ai_flight` 25 *How an enemy flies* | "An enemy flies the same flight computer you do: closing while it is outside its standoff, circling once inside, nose on you throughout. Its standoff is clear space between the two HULLS, and its speeds come off its own live drive." | none | `wiki/factions#what-allegiance-drives` | none; `proven_by: [TUTORIAL_SCENARIO_ID]` - Part 4's drones are the only shipped hostiles that manoeuvre |
| B | `advanced_graphics` 40 *Graphics presets* | "One Low / Medium / High preset trades richness for framerate. High adds camera shake; Medium drops it and keeps hit flashes; Low drops both, spawns no particle bursts at all, and renders the 3D world at a reduced resolution upscaled to the window." | none | `wiki/settings#graphics-quality` | none |
| B | `advanced_audio` 45 *Audio mix* | "Four sliders: Master, Interface, World and Music. Interface is the cockpit - menu clicks, HUD ticks, the flight computer. World is everything out there, heard at the distance it happens. Master multiplies the other three, so a channel you turned down stays down." | none | `wiki/settings#audio` | none |

---

## 5. What blocks the rest

**Practice is the binding constraint on the whole handbook.** Four
`role: Lesson` scenarios exist (`drills.rs:68-74`), all Flight- or
Combat-shaped, all flying the same trainer on the same range. The trainer
mounts **one kinetic PDC and nothing else**
(`assets/base/ships/base.content.ron:45220`), so no lesson about torpedoes, the
railgun, Pierce rounds, point defense or docking can be practised or proven by
anything that exists. Every Start Here, Shipbuilding, NOVA OS and Advanced
proposal above therefore carries `practice: None`. Making them practisable is
one new drill per weapon family plus a hull that carries the part.

The cheapest additions, if the owner wants them: `drill_novaos` (a damaged
trainer parked at rest, so repair, reload and rebind have something to act on)
and `drill_hud` (a quiet cruise with a well in reach, to show the sphere change
colour four ways).

**Three proposals are blocked on the wiki, not on the handbook.** The anchor
test resolves every `wiki_path`, so a lesson cannot ship before its heading
does:
- `build_generate` - Generate is undocumented. A grep over `web/src/wiki/`
  returns nothing for a feature that rolls a whole hull.
- `build_readout` - the editor's build readout has no heading.
- `flight_dock` / `build_docking_port` - documented, but no shipped hull
  carries a port, so nothing demonstrates it.

**One proposal needs an owner decision.** `build_faces` is the editor's core
gesture set, and the handbook's two authoring rules collide over it: a lesson
never spells a key, and `actions` must be registered ids - but
`crates/nova_editor/` registers no `ActionBinding` at all. Either the editor
gains registered actions, or that lesson ships with an empty chip row and
describes the gestures in prose. Same question, smaller, for the railgun and
per-section weapon triggers: they are built from content `input_mapping` at
spawn (`crates/nova_ship/src/input/bindings.rs:9-10`) and have no action id
either.

**Two recurring shapes worth fixing while editing anything.** `proven_by` and
`practice` disagree in places - `combat_turrets` is proven by a drill it gives
no button to reach - and several lessons that state a hard number carry
`field_notes: &[]`, when the number is exactly the two-line claim a field note
wants.

## Review method

Two reviewers worked with no session context, reading the wiki corpus and then
the handbook. Everything above is `path:line` cited. In the main session five
load-bearing corrections were re-checked directly against the code before this
document was written: the NOVA OS freeze (C1), ORBIT taking the well not the
mark (C2), `WeaponsRaised`'s actual consumers (C4), the turret elevation band
and own-hull projectile transparency (C5), and the collapse threshold. All five
held.
