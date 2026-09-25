# Type asteroid kind identifiers and standardize content identity terms

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, world, content, refactor

## User facts

- Raw `String` asteroid kinds and mixed `kind`, `type`, and `design` terminology make generated-world APIs harder to reason about.
- The world-generation interface work must not silently turn an open content identity into a closed enum.

## Decisions

- Use one ownership-based convention: `Type` is a closed Rust enum, `KindId` is an open registry identifier, and `DesignId` is an authored catalog identifier.
- Preserve asteroid kinds as an open content surface. Do not replace them with a closed enum.
- Unknown IDs must fail at lint and again at runtime. Do not add a fallback kind.

## Agent findings

- `PlanetType` is a closed enum and rejects unknown serialized variants.
- Asteroid kinds are raw strings checked against `ASTEROID_KINDS`; the owning module says this table may become loaded data for mods.
- `AsteroidKind(pub String)` gives the runtime component a typed wrapper, but authoring, generation, lint, and lookup still pass raw strings.

## Delivery

- Introduce a transparent `AsteroidKindId` newtype in the lowest crate shared by authoring, scenario loading, and world generation.
- Migrate `AsteroidConfig`, `AsteroidKind`, asteroid lookup and lint, editor/picker paths, scenario builders, generated-sector records, fixtures, and tests.
- Keep RON readable and explicit. Treat any format impact as a shipped-format migration decision.
- Align docs and field names with the `Type` / `KindId` / `DesignId` convention.
- Delete raw-string asteroid-kind interfaces and duplicate validation paths made obsolete by the typed ID.

## Verification

- Lint refuses an unknown asteroid kind ID.
- Runtime loading refuses an unknown kind even when lint was skipped.
- Every shipped asteroid kind resolves to its expected look.
- Authored scenarios and procedural sectors preserve their existing asteroid identity.
- Serialization proof records whether the RON representation changed.

## Done when

- No owning asteroid API accepts or stores an untyped kind `String`.
- The terminology convention is documented at the owning APIs.
- All callers compile without compatibility aliases.
- Focused scenario, authoring, editor, and world-generation proofs pass.
