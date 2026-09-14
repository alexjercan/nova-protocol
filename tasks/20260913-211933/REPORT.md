# Duplication audit: report

Step 1 (identify and record) is complete. No code was changed.
Full detail with every `path:line` and excerpt is in `FINDINGS.md`.

## Claim

The workspace holds four BLOCKER-grade duplicate families whose copies already
disagree in the tree, and about thirty more where a plausible change must be
applied in more than one place. The concentration is in input rebinding, in the
NOVA OS and editor UI shells, and in the lint-versus-editor content vocabulary.

Two families the reviewers rated highly did not survive scrutiny and were
corrected downward after the owner challenged them. One of those corrections
withdraws a defect claim this orchestrator had previously reported as verified.

## Evidence

Ten reviewer-passes over `crates/*/src/**` (~316k lines), in five pairs. Each
pair read the same code twice by deliberately different routes - bottom-up from
code shape, and top-down from a behavior list written before searching - so
that agreement is corroboration rather than a shared blind spot. Examples,
benches, fixtures and test modules were excluded as finding subjects per the
owner's scope.

| Wave | Pair | Chunk |
| --- | --- | --- |
| 1 | A | UI and presentation, ~118k lines |
| 1 | B | simulation, ~110k lines |
| 2 | C | world, content and authoring, ~110k lines |
| 2 | D | cross-crate seams, plus the tooling crates |
| 3 | E | the surfaces waves 1-2 recorded as unread |

Every load-bearing citation was re-read during adjudication. Counts: 4 BLOCKER,
24 MAJOR, 24 MINOR, 16 families confirmed NOT duplicated, 11 claims rejected.

### The four BLOCKERs

1. **`rebind-policy-four-surfaces`** - four surfaces write a key binding under
   four different conflict policies, and the shared sink enforces none.
   Reported independently by three reviewers across two chunks. Two verified
   live consequences: a console `bind` is accepted, persisted, then silently
   reverted at next launch; and `nova_os_ui/src/ship/rebind.rs:82` replaces the
   whole binding vector, dropping a section's gamepad trigger, while
   `nova_editor/src/keybind.rs:922-924` asserts the opposite for its own path.
2. **`scroll-driver-fork`** - `nova_os_ui` re-implements the `nova_ui` scroll
   stack it imports. `nova_ui/src/screen/mod.rs:4-9` predicted this failure in
   writing. The wheel step is 20.0 against the shared 60.0.
3. **`terminal-command-error-vocabulary`** - the `ResolvedCommand` error arms
   written three times; the same typo in the same CRT gets two different
   answers depending on which shell is open.
4. **`scenario-name-vocabulary-two-tables`** - the lint's hand-written match
   and the editor's `Names` reflect attributes are two tables for one question,
   already out of step. A dangling id in `SetInfiniteAmmo` passes lint, is
   dropped by the editor, and warns at runtime.

### The two seeded questions

Both were answered with code, and both answers differ from the premise.

- **ESC versus click.** The two paths have not diverged; the keyboard path was
  never built for `SettingsPanel`, `ModsPanel` or `ScenariosPanel`. All four
  production `KeyCode::Escape` readers in the seven UI crates do something
  else. **Corrected after owner challenge:** the duplication half is four
  ~13-line `Visibility` handlers, below this audit's threshold, and the missing
  Escape is a UX consistency gap, not duplication. No player is trapped; every
  panel has a Back button. Recorded as MINOR plus a separate note.
- **PDC and weapon classes.** A double-barrel turret is not a second
  implementation: `turret_section/setup.rs:106-174` collects every muzzle into
  one `TurretSectionMuzzles` looped against one magazine. **Corrected after
  owner challenge:** the cross-class finding was overstated. What the three
  classes share is two statements calling two already-shared helpers - the
  line-or-two case that is out of scope. The recommendation is **not** to
  consolidate them.

## Change

None proposed as work yet. `FINDINGS.md` names a proposed home and signature
per family; the owner selects before anything is written.

The cheapest high-value fixes, for reference: BLOCKER 1 is one
`nova_input::registry::commit_rebind` that all four surfaces call; BLOCKER 2 is
a deletion, since `nova_os_ui` already imports the module it re-implemented;
BLOCKER 4 is in-crate, folding the lint arms onto `walk_names`.

## Blast radius

Input rebinding touches `nova_input`, `nova_menu`, `nova_os_ui`, `nova_editor`
and `nova_console`, and changes persisted settings behavior - it needs a
launch-cycle check, not only a unit test. The scroll fix changes felt scroll
speed in the NOVA OS drawers, which is player-visible. The scenario-name fix
changes lint output, so content currently passing may start failing, which is
the intended direction. Nothing in this audit touches content formats, so no
`**(breaking)**` changelog entry is implied by the findings themselves.

## Verification

This step produced no code change, so there is nothing to test. The findings
were verified by reading, not by execution: every cited line was re-read, and
reachability was traced for each BLOCKER rather than asserted.

Stated limits:
- No test was run and no example was played. Claims about runtime consequence
  are source-traced, and labelled as such in `FINDINGS.md`.
- The `nova-os-palette-second-copy` divergence has an open question: whether
  the four differing tokens are deliberate CRT art direction or drift. Both
  files cite the same PoC HTML, which argues drift, but the PoC was not opened.
- Whether the editor's stock empty `id_prefix` is reachable in a shipped
  scenario was not established; finding 30 is filed on the code path alone.

### Process notes worth keeping

- The orchestrator's mechanical duplicate-block index does not see inline
  `#[cfg(test)]` modules, only `/tests/` directories. Its highest-priority
  lead (`closing_speed` in four crates) was consequently wrong, and four more
  of its flags were test fixtures. Three reviewers caught this independently.
  Any reuse of that script must gate on the enclosing `#[cfg(test)]` span.
- Pairing earned its cost twice over: it corroborated eight families that one
  reviewer alone would have left as single-source claims, and it produced two
  refutations of the orchestrator's own leads. It also surfaced one genuine
  pair contradiction, on the theme palette, which adjudication resolved as a
  scope gap rather than an evidence conflict.
- Absence of a line is not evidence of drift. The withdrawn railgun claim was
  verified on the absence of an easing seed without checking for a
  justification, which existed in a different file. Wave 3 was briefed on this
  explicitly.
