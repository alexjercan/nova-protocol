# Investigate asteroid and planet vulnerability ownership

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, scenario, destruction, world, research

## User facts

- The `invulnerable: bool` field on `AsteroidConfig` and `PlanetConfig` is obsolete because the body type should own vulnerability.
- Asteroids should always be carvable and destructible.
- Planets should remain invulnerable until planet destruction is separately designed and proven.

## Decisions

- Remove `invulnerable` from both `AsteroidConfig` and `PlanetConfig`. Do not replace it with another durability, carving, static-body, or invulnerability option.
- Always install the existing asteroid carve, damage, collision-event, and destruction behavior, including for streamed asteroids.
- Never install asteroid damage behavior for planets. Planet invulnerability is an invariant of `PlanetConfig`, not authored data.
- Migrate immutable asteroid-shaped planetoids and gravity anchors to `PlanetConfig` where they are semantically planets. Let ordinary rocks and test cover become destructible rather than preserving a special immutable asteroid path.
- Keep the legacy asteroid in `planet_types` as a visual comparison, but make it follow normal destructible asteroid behavior.
- Remove redundant runtime invulnerability marker components and the `invulnerable` field from probe body snapshots. Body kind is authoritative; do not add a compatibility alias for the unshipped schema.
- Streamed asteroid destruction remains session-local until world mutation persistence is designed. A retired and revisited sector regenerates its pristine deterministic contents, as it already does for destroyed derelicts.
- If making all active streamed asteroids destructible is too expensive, optimize damage or collision activation without restoring author-controlled vulnerability.
- Planet destruction remains out of scope and unsupported. It requires a separate design for damage, render and collider failure, gravity-well teardown, orbit effects, IDs, objectives, and persistence.

## Agent findings

- `AsteroidConfig::invulnerable` changes construction: destructible rocks receive `DamageMarks` and `CollisionEventsEnabled`, while invulnerable rocks omit the carve pipeline (`crates/nova_scenario/src/objects/asteroid.rs`).
- `PlanetConfig::invulnerable` is not a choice. `PlanetConfig::validate`, scenario lint, and planet spawning reject `false` because planets have no damage or destruction implementation (`crates/nova_scenario/src/objects/planet_type.rs`, `objects/planet.rs`, and `lint/scenario.rs`).
- Full asteroid destruction already fires `OnDestroyedEvent` and despawns the asteroid root. A gravity well attached to that root is removed with it, and orbit tracking retains the well ID needed to report an end edge.
- `nova_world` currently hardcodes streamed asteroid configs as invulnerable. Sector retirement discards all local mutations and deterministic regeneration restores pristine bodies on return; this limitation already applies to destroyed streamed derelicts.
- The extra streamed-world cost is unmeasured. Every active asteroid can gain empty damage state and collision-event reporting, while carve remeshing is paid only after damage. The maximum active-window case needs a matched performance proof.
- Existing invulnerable asteroid uses split into old planetoid or gravity-anchor representations, visual comparisons, and test or screenshot cover. The latter cases do not establish a gameplay need for immutable asteroids.
- `AsteroidInvulnerable` and `PlanetInvulnerable` are read by probe snapshots but do not own the damage behavior. With type-owned policy, the body kind fully determines the answer.

## Delivery

1. Preserve the current planet `false` lint and load refusal as before evidence.
2. Change the owning scenario configs first by removing both fields and their validation branches.
3. Make asteroid spawning always install the existing carve and destruction components; keep planet spawning permanently outside that pipeline.
4. Remove invulnerability marker components, reflection registration, snapshot fields, docs, tests, and stale failure text.
5. Use compiler and content errors to migrate every Rust builder, generated or shipped RON source, editor template, example, probe fixture, and world materializer.
6. Convert semantic planetoids and immutable gravity anchors to explicit `PlanetConfig` objects. Do not preserve asteroid aliases or adapters.
7. Regenerate owned content and run focused lint, lifecycle, world-streaming, and performance proofs.

What dies: both config fields, both runtime marker components, planet false-value checks, immutable asteroid construction, and probe snapshot vulnerability reporting.

What may break: authored RON schema, Rust builders, editor templates, examples that use asteroid-shaped planetoids, captures that rely on immutable cover, probe golden data, and streamed-world performance.

What must fail loudly: stale content that still authors either field, invalid migrated planet configurations, malformed generated bodies, and any lifecycle proof that leaves a gravity or orbit reference after asteroid destruction.

## Verification

- Preserve a before artifact showing that `PlanetConfig { invulnerable: false }` is currently rejected by lint and loading.
- Prove an authored asteroid always receives carve and collision-event state and can be fully destroyed.
- Prove destroying a massive asteroid removes its body, gravity well, children, authored ID resolution, and active orbit relationship while emitting the expected destruction and orbit-end behavior.
- Prove planets receive no asteroid damage state and remain non-destructible without an authored flag.
- Prove a streamed asteroid can be carved and destroyed, then document and observe pristine regeneration after sector retirement and return.
- Compare matched maximum-window streamed-world runs before and after the migration. Inspect frame and physics evidence against the named pre-change run; do not use an absolute timing assertion.
- Run focused content lint and confirm no source or generated content retains either field.

## Done when

- Both config fields, marker components, validation branches, snapshot fields, aliases, and obsolete tests are deleted.
- Every content source, Rust caller, editor path, example, world generator, probe consumer, and invalidated document uses the type-owned policy.
- Semantic planetoids and immutable gravity anchors use `PlanetConfig`; ordinary asteroids are destructible.
- Focused destruction, gravity, orbit, streaming, lint, and matched performance proofs pass with inspected evidence.
- Remaining mutation persistence limits are stated without claiming persistent asteroid destruction.
