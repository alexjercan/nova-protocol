# Auditor's torpedo bay clips inside its hull - mount it on the ship's side

- STATUS: OPEN
- PRIORITY: 43
- TAGS: v0.7.0,scenario,content,bug

User report (2026-07-17 playtest): "the Auditor's torpedo bay is placed
inside the ship, it should be on its side, it's clipping." The Auditor is
the hostile in webmods/the-ledger/ledger_ch4.content.ron (spawned by two
mutually exclusive ending handlers - fix BOTH spawn sites; the
content_lint dual-spawn warning marks them). Move the tube section to a
side mount so the mesh no longer intersects the hull sections; check the
section grid offsets against how other multi-section ships place side
mounts, and eyeball-verify if a screenshot rig is cheap (render-output-
eyeball lesson) or verify offsets geometrically otherwise. Sibling task
20260717-143806 changes the same ship's gun - coordinate landings.
