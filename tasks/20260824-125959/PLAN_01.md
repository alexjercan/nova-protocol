# Plan 01: First Shift playtest revision

## Goal

Revise the built-in First Shift scenario after the first full cinematic
playtest. Make its tutorial readable and deliberate, make navigation targets
reliable, keep close work precise, and stage Meridian's destruction without
letting live player input disturb the cinematic.

This plan belongs to task `20260824-125959`. The final dialogue and exact story
events remain owner-authored. Temporary lines are playtest scaffolding.

## Related tasks

Keep these investigations separate from the scenario pass:

- `20260904-084733`: review and simplify autopilot arrival standoff. Start with
  code review and evidence. Reconcile the global, per-ship, per-order,
  target-radius, mover-radius, and gravity-parking paths before changing the
  public model.
- `20260904-084734`: enforce total-vector speed caps for diagonal manual and RCS
  flight while preserving braking from an overspeed state.

Do not duplicate either task inside this plan. Consume their final contracts
when they land.

## Decisions

### Scenario identities

- The carrier is `Meridian`.
- The player ship is `Cutter`, with runtime ID `cutter`.
- Cutter is crewed. The player is its captain; the copilot and other crewmates
  can speak.

### Content responsibility

- Cinematic safety is an authoring responsibility. A scenario that disables
  control must first place the player safely and prove that placement.
- The engine may support moving cinematics. It must not globally require a
  stationary ship or forbid gravity wells.
- First Shift starts control-locked cinematics only after Cutter is settled in
  clear space and outside a gravity well.
- GOTO remains a direct flight plan without obstacle avoidance. First Shift
  must author and test every prescribed route as a clear corridor.

### Dialogue policy

- Do not automatically make every `StoryMessage` cinematic.
- Important conversations belong mainly at the opening and ending.
- Important conversations use an explicitly authored safe conversation hold.
- Mid-scenario dialogue is short and sparse.
- A short line needed during quiet travel is a normal flight bark.
- Orbit dialogue remains normal comms because Cutter is moving in a gravity
  well.
- Mainline content normally keeps only one or two comms cards active. Do not
  enforce that as a creator-facing engine limit.

### Tutorial precision

- Do not enlarge beacon intersections to compensate for poor presentation.
- Spawn and frame routes clearly before asking the player to fly them.
- Manual RCS marks may complete on precise physical intersection.
- Salvage collection remains close and physical. Tune toward visible contact,
  not a large collection sphere.
- Cutter keeps a 150 m/s manual-flight limiter for the whole chapter. Do not
  remove it silently. GOTO plans its own speed independently.

## Implementation plan

### 1. Reliable player maneuver completion

Add physical completion events for the player's real autopilot:

- GOTO completion reports the reached target and player ship.
- STOP completion reports the player ship.
- Report only successful terminal conditions.
- Manual cancellation, target loss, and capability loss do not report success.
- A GOTO that transitions into a viable gravity-well orbit uses the orbit
  lifecycle instead of reporting a completed ordinary GOTO.

Use these events in First Shift:

- Make STOP a real objective after the launch mark.
- Do not introduce RCS until STOP has physically brought Cutter to rest.
- Do not complete a GOTO navigation beat on the first `OnEnter` overlap.
- Keep a GOTO target alive while autopilot still uses it.
- Remove its objective marker and despawn it only after GOTO settles.
- Apply the same rule to transit marks, the return work site, and Meridian's
  outer hold.
- Keep manual RCS marks on precise `OnEnter` completion.

Update creator documentation, player tutorial documentation, structural tests,
and generated content.

### 2. Explicit cinematic player-control suspension

Add reusable scenario actions:

- `SuspendPlayerControl`.
- `ResumePlayerControl`.

Suspension must:

- Disable manual flight input.
- Disable mouse-driven ship steering and camera-look gameplay input.
- Disable combat stance and player weapon input.
- Clear held burn, RCS, rotation, stance, and weapon intent so input cannot
  remain latched.
- Preserve pause, menu, and any explicit cinematic-skip input.
- Leave physics, timers, scripted ships, weapons, and the cinematic running.
- Restore control reliably on resume and scenario teardown.

Keep `SetCamera`, `SetCameraAnchor`, and `ReleaseCamera` camera-only. Content
combines camera and control authority explicitly.

Apply suspension to every interval in First Shift where an anchored cinematic
pose owns the view. Resume input whenever the camera is handed back during the
long approach and after the final shot.

Add focused tests for input contexts, held-intent cleanup, pause/menu access,
repeat suspend/resume, and teardown restoration.

### 3. Destruction camera revision

Remove the close front view that mostly shows Cutter and hides the event.

Recompose the attack so:

- Meridian is viewed from Cutter's perspective.
- Cutter remains visible in the foreground of Cutter-relative shots.
- A rear-quarter or side-quarter view replaces the frontal close-up.
- Cutter occupies only a controlled edge of the frame.
- Meridian, the warship, and the ordnance lane remain readable.
- The decisive destruction shot is not anchored to Meridian, because Meridian
  is about to disappear.
- The chase camera returns before the distress aftermath.

Keep the existing physical attack chain. Warship movement, alignment, weapon
fire, and departure continue to sequence from real completion events rather
than guessed movement delays.

Review the result in a rendered run. Coordinate tests alone do not prove a
useful composition.

### 4. RCS briefing and route

Expand the open-space RCS lesson to four or five precise beacons.

Sequence:

1. Cutter reaches the launch work mark.
2. Cutter completes a real STOP.
3. Suspend player control.
4. Spawn the complete RCS route together.
5. Use a wide Cutter-relative pose that clearly shows the route.
6. Deliver one or two short instruction cards.
7. Release the camera and restore control.
8. Activate the first beacon as the current objective.
9. Advance through the remaining marks one at a time.
10. Despawn each completed mark.

Presentation rules:

- All route beacons may exist together so the player understands the path.
- Only the current step is the active objective.
- Later marks remain visually identifiable without competing as active
  objectives.
- Preserve tight physical intersections.
- Teach short RCS taps, translation without turning, and the violet velocity
  indication.
- Do not describe 5 G, 100 m/s RCS as moving "a metre at a time."

### 5. Manual speed limiter and diagonal-cap integration

Keep Cutter's manual speed limit at 150 m/s for the whole scenario.

- Do not clear or raise it after the launch lesson.
- Make its presence legible in the flight HUD if the existing HUD does not
  explain it.
- Do not apply the manual cap to GOTO planning.

Task `20260904-084734` owns the underlying diagonal bug:

- RCS must share one total velocity-vector budget across all axes relative to
  `RcsReference`.
- Manual flight must not exceed its displayed total speed limit by turning and
  adding another velocity component.
- Braking input remains available at and above the cap.

Integrate and re-test First Shift after that task lands.

### 6. Precise salvage contact

Review the current 80 m salvage pickup volume against Cutter and crate geometry.

- Reduce it toward visible physical contact.
- Account for the cutter collider so collection remains reliable.
- Ensure collection does not require a destructive impact.
- Preserve one crate or work objective at a time.
- Verify each crate is clear of worst-case asteroid geometry.

Choose the final radius from a rendered contact test, not from an arbitrary
number.

### 7. Direct-route safety

Audit every route that First Shift asks GOTO to fly:

- Transit 1.
- Transit 2.
- The planetoid approach where applicable.
- Return to the asteroid work site.
- Return to Meridian's outer hold.

For each straight segment:

- Check both planetoids.
- Check every shared plate rock using its worst-case geometric factor.
- Include Cutter's hull radius and an authored safety margin.
- Move scenario-specific marks when a route crosses clutter.
- Do not alter the fixed shared stage geometry used by Second Shift.

Add structural segment-clearance tests. These tests prove authored scenario
correctness; they do not promise generic runtime obstacle avoidance.

### 8. Full orbit and visual-cover story beat

Change the planetoid detour:

1. Mandatory work navigation takes Cutter behind the small planetoid relative
   to Meridian.
2. A crewmate observes that Meridian cannot directly see them.
3. The crew talks the captain into "doing a donut."
4. Cutter starts ORBIT.
5. Cutter completes a real orbit lap rather than a 13-second timer.
6. Cutter returns into Meridian's view at a useful departure angle.
7. Meridian reacts with a humorous "What are you doing, Cutter?" beat.
8. Meridian orders Cutter back to the paid asteroid work because departure and
   the bonus depend on completing it.

Prefer a reusable orbit-lap event based on accumulated angular travel around
the active well. Do not guess a full lap with a timer if the actual orbit can
report progress.

Current player locks and radio do not implement asteroid occlusion. For this
pass, describe direct visual cover only. Do not teach lock or radio degradation
as shipped behavior until those systems exist.

### 9. Comms and objective readability

Make comms screen-relative and substantially larger:

- Width near half the viewport rather than a fixed 320 px.
- Body text around 20 px, adjusted only after 720p and 1080p inspection.
- A distinct speaker header instead of one same-style text run.
- Larger icon, padding, row spacing, and line spacing.
- Preserve readable contrast and subtitle-like placement during cinematics.

Improve related guidance:

- Objective notification text around 16-18 px.
- World objective labels at least 16 px.
- Larger objective diamonds and off-screen chevrons.
- Keep marker labels readable over bright planetoids and asteroid clutter.

Do not enforce a two-card visible limit for all creators. Reconsider the
existing visible and pending queue caps if they silently discard authored
comms. First Shift itself should pace output so only one or two cards normally
need attention.

Test at minimum at 1280x720 and 1920x1080. Check ultrawide behavior before
accepting an unconstrained 50-percent width.

### 10. Dialogue placement pass

After mechanics and presentation stabilize, classify every First Shift line:

- `Flight bark`: one short line during low-workload travel; no cinematic.
- `Conversation hold`: an important exchange at a safe settled location;
  explicit camera pose and control suspension.
- `Set piece`: the destruction sequence; cinematic camera and control
  suspension while the physical world continues.

Move or shorten anything that requires reading during close asteroid work,
manual braking, radar acquisition, or target identification.

Keep important conversations concentrated at the scenario opening and ending.
Use mid-scenario conversation holds sparingly. Orbit is the explicit exception:
its dialogue remains normal comms while autopilot holds the gravity maneuver.

### 11. Possible scripted Player helm authority

Record as follow-up architecture, not a requirement for the current First Shift
shot:

- `MoveShipTo`, `StopShip`, and related shared helm orders may accept a Player
  ship only while player control is suspended.
- Issuing one while player input is active remains an authoring and runtime
  error.
- The scripted order owns the helm without competing with player input.
- Scripted authority must complete or be cleared before `ResumePlayerControl`.
- Forced weapon actions remain separately governed; suspension alone must not
  silently authorize scripted player weapon fire.

First Shift should use the player's real GOTO to settle at the attack hold
unless the reviewed cinematic proves that scripted repositioning is necessary.

## Verification gates

### Focused automated checks

- Player STOP reports exactly one successful completion.
- Player GOTO reports exactly one successful completion with its target.
- Cancellation, target loss, and capability loss report no success.
- A GOTO beacon remains alive until physical settle and disappears afterward.
- First Shift cannot grant RCS before STOP completes.
- Cutter retains its 150 m/s manual cap throughout the scenario.
- Every prescribed GOTO segment clears conservative stage geometry.
- Every temporary mark is eventually removed.
- The orbit detour completes a real lap before the return-to-work beat.
- The RCS teaching pose has paired control suspension and restore.
- The terminal attack keeps camera and player control suspended from reveal
  through aftermath; scenario teardown restores both.
- Generated base content passes content lint.

### Rendered review

Play First Shift from the beginning and verify:

- Opening comms are readable before flight begins.
- STOP must actually settle before RCS teaching starts.
- The four- or five-mark RCS route is obvious from its establishing shot.
- Tight beacon intersections feel deliberate rather than hard to identify.
- Crate pickup visually reads as contact.
- Blindly following every prescribed GOTO does not hit a rock.
- The full orbit begins and ends at useful angles.
- The visual-cover joke reads without promising nonexistent radio mechanics.
- Mouse movement cannot steer Cutter during a cinematic.
- The silent destruction sequence clearly shows the railgun strike on Meridian,
  torpedo impacts from Cutter, the warship starting away without another cut,
  and Cutter facing the aftermath.
- The removed front close-up does not return in another pose.
- Comms, objective notifications, and world markers are readable at 720p and
  1080p.

## Documentation and generated content

Update affected surfaces with each behavior change:

- `CHANGELOG.md` under the correct subsystem.
- `web/src/wiki/getting-started.md` for the First Shift tutorial.
- `web/src/wiki/flight-autopilot.md` for player flight behavior.
- `web/src/wiki/hud.md` for presentation changes.
- `web/src/create/events.md` and `web/src/create/actions.md` for new scenario
  vocabulary.
- `docs/scenario-system.md` for engine architecture changes.
- Generated base content through `cargo run content gen`; never hand-edit
  generated `.content.ron` files.

Run only affected checks during each step. Before the complete plan is accepted,
run focused crate tests, content generation and lint, web CI, `mdbook build`,
`git diff --check`, and the rendered First Shift review.

## Completion

This plan is complete when First Shift:

- teaches STOP and GOTO through real physical completion;
- never despawns an active GOTO target;
- presents a clear four- or five-mark RCS exercise;
- keeps close work precise under a persistent 150 m/s manual limiter;
- flies only authored clear GOTO corridors;
- completes a real playful orbit before Meridian sends Cutter back to work;
- uses large, readable comms and markers;
- suppresses gameplay input during explicit cinematics;
- shows Meridian's destruction from a useful Cutter-visible composition; and
- passes focused automated, generated-content, documentation, and rendered
  playtest gates.
