# Node Editor Prior Art

Round 4 research for v0.12.0 planning (godot-style node editor, lowered to flat ScenarioConfig RON). Written 2026-08-24.

Scope: Godot scenes, Unity prefabs, Bevy ecosystem state, shipped vehicle-in-world editors, and flattening prior art. Licences are listed per project. GPL and share-alike projects are marked UNUSABLE for code reuse (Nova is MIT); their design is still fair to learn from.

## 1. Godot scene and node model

Licence: Godot Engine is MIT (Expat). Code patterns are reusable.

- A saved scene is a blueprint. "Once you have saved a scene, it works as a blueprint: you can reproduce it in other scenes as many times as you'd like."
  https://docs.godotengine.org/en/stable/getting_started/step_by_step/instancing.html
- Propagation is live and automatic. Edit the source scene, save, and "all instances of the Ball in the project will see their values update." Same page.
- Overrides pin values. "Changing a property on an instance always overrides values from the corresponding packed scene." The inspector shows a revert icon per overridden property. Same page.
- Owner controls persistence. A node saves into a scene file only if its owner chain reaches that scene's root. Nodes inside an instanced child scene are owned by the child scene, so the parent scene stores only the instance node plus overrides. Setting owner to the edited root and clearing the scene file path is how "Make Local" inlines an instance (one-way, lossy of the link).
  https://deepwiki.com/godotengine/godot-docs/5.1-nodes-and-scene-tree
- Editable Children is per-placed-instance tweaking; scene inheritance is the reusable-template path. Editable Children exposes an instance's internal nodes in the parent scene without a new scene file. Known cost: tree clutter, and internal-node renames in the source scene can orphan overrides.
  https://uhiyama-lab.com/en/notes/godot/editable-children-vs-scene-inheritance/
- tscn serialization: `[node ...]` headings with `name`, `type`, `parent` (absolute path minus root name), then only non-default properties. Node-heading keywords include `instance`, `instance_placeholder`, `owner`, `index`, `groups`. An instanced sub-scene is one node entry with `instance=ExtResource("id")`; overrides on its internal nodes are extra `[node name="Inner" parent="InstancePath" index="0"]` entries carrying only the changed properties; editable-children status is stored as a path list (the PackedScene bundle keeps `editable_instances`). The exact override-entry shape above is standard tscn output but the docs page does not show a snippet; treat the exact syntax as unverified against the spec page.
  https://docs.godotengine.org/en/stable/engine_details/file_formats/tscn.html
  https://github.com/godotengine/godot-docs/blob/master/engine_details/file_formats/tscn.rst
- Known bug worth avoiding: the runtime PackedScene pack path does not compute the override diff correctly and writes every property, silently turning "inherit" into "override" for all children. Editor save and runtime save disagree.
  https://github.com/godotengine/godot/issues/67884
- Enter/exit edit context: opening an instance for editing opens the SOURCE scene as its own edit root (its root becomes the edited root; owner and save target switch to that file). Exiting returns to the parent scene, where the instance is a single collapsed node again. Editable Children is the only way to touch internals without switching edit root.

What survives flat lowering: instance node + property-override diff (a prototype reference plus per-instance field overrides maps 1:1 to a flat prefab record). What needs nesting: Editable Children (structural overrides inside an instance), scene inheritance, and instance_placeholder deferred loading. If Nova's flat format has no per-child overrides inside a prefab, then Editable Children in the editor must either be forbidden or force a "detach to inline entities" (Godot's Make Local) at lowering time.

## 2. Unity prefab system

Licence: Unity is proprietary. Design only, no code.

- Prefab Mode is the enter/exit edit context: open the asset in isolation or "in context" grayed against the scene; edits save to the asset and propagate to all instances that have not overridden the value.
  https://docs.unity3d.com/6000.4/Documentation/Manual/NestedPrefabs.html
- Override granularity (what mattered in practice): four kinds - property overrides, added components, removed components, added GameObjects (later also removed GameObjects). Serialized in the scene YAML as a `PrefabInstance` with an `m_Modification` block: `m_Modifications` entries of (target fileID, propertyPath, value), plus added/removed lists. Property-path granularity is the workhorse; structural overrides are the complexity source.
  https://docs.unity3d.com/6000.4/Documentation/Manual/PrefabInstanceOverrides.html
  https://docs.unity3d.com/6000.6/Documentation/Manual/yaml-prefab-serialization.html
- Nested prefab failure modes: "Apply All" collapses overrides up the chain; applying at the wrong layer overwrites an inner override with an outer one and loses the instance value. Overrides target children by fileID; a renamed child, missing base, or stale fileID silently discards the overrides stacked on it.
  https://bugnet.io/blog/fix-unity-prefab-nested-override-lost-on-apply
  https://discussions.unity.com/t/could-someone-clarify-how-prefab-overrides-work-with-nesting/725034
- Flattening: "Unpack" converts an instance to a plain GameObject "which no longer has any links to the original prefab asset" while retaining override values - Unity's in-editor lossy collapse, mirror of Godot's Make Local.
  https://docs.unity3d.com/Manual/UnpackingPrefabInstances.html
  https://docs.unity3d.com/ScriptReference/PrefabUtility.UnpackPrefabInstance.html
- Build/runtime: the prefab link is an editor concept (PrefabUtility lives in UnityEditor only); built players carry no override machinery, and runtime Instantiate produces an independent clone. I did not find an official page describing the build-time collapse step explicitly; treat the exact mechanism as unverified, the editor-only nature of the link as solid.

Lesson: both big engines chose live propagation with per-instance override pinning, and both provide exactly one escape hatch (Unpack / Make Local) that is deliberately lossy. Neither tries to round-trip a flattened copy back into an instance.

## 3. Bevy ecosystem prior art (state as of 2026-08)

- Bevy 0.19 shipped BSN (Next Generation Scenes): the `bsn!` macro returns a `Scene`; BSN expressions are PATCHES that layer, so `my_button()` extends `button()` by overriding fields - inheritance-as-patch, natively. `Template`/`FromTemplate` handle asset refs (string path -> Handle). Shipped: code-driven workflow. Not shipped: `.bsn` asset files, editor. Bevy is MIT OR Apache-2.0; patterns and code reusable.
  https://bevy.org/news/bevy-0-19/
  https://github.com/bevyengine/bevy/pull/23413
- bevy_editor_prototypes was ARCHIVED 2026-04-16; editor work moved into the main Bevy repo (BSN asset format, UI, tools) and to the community project Jackdaw. MIT OR Apache-2.0.
  https://github.com/bevyengine/bevy_editor_prototypes
- Jackdaw (jbuehler23/jackdaw): "A Bevy 0.19 scene editor with hierarchy, inspector, and 3D viewport". MIT OR Apache-2.0. Most relevant living prior art: its upstream issue documents the exact infrastructure a graph-editor-over-ECS needs - BSN write-back (World -> text with default-diffing), a persistent AST kept alongside the live ECS so edits round-trip, `SceneDocument` + `ScenePatch` edit operations, and Handle-to-path resolution. This is the same shape as Nova's plan: editor keeps a rich document, saving lowers to a serial format.
  https://github.com/jbuehler23/jackdaw
  https://github.com/bevyengine/bevy/issues/23637
- space_editor (rewin123/space_editor): "bevy prefab editor", MIT. Stalled around Bevy 0.14 - stale relative to 0.19, but its prefab-spawn-and-sync systems are copyable in design.
  https://github.com/rewin123/space_editor
- bevy_editor_pls: MIT OR Apache-2.0, last push 2024-12, effectively unmaintained. Inspector-style, no prefab instancing. Skip.
  https://github.com/jakobhellermann/bevy_editor_pls
- Blenvy (kaosat-dev/Blenvy, ex blender_bevy_components_workflow): Blender-as-editor; blueprints are Blender collections exported as gltf, instanced in a level gltf; components ride as gltf extras. Whole repo (code, contents, assets) is dual MIT OR Apache-2.0 per README. Status Alpha 1. Design lesson: "blueprint = named collection, level = instances of collections" is exactly prefab-by-reference, and it lowers through a flat interchange format (gltf) successfully because instances carry only a name reference plus transform.
  https://github.com/kaosat-dev/Blenvy

Maturity verdict: copy patterns from BSN (patch semantics, default-diffing write-back) and Jackdaw (document/patch editor architecture). Nothing here is worth depending on for v0.12.0; BSN asset files do not exist yet and Jackdaw's format is its own (.jsn).

## 4. Shipped "vehicle inside a world" editors

All games in this section are proprietary unless noted. Design only.

- Cosmoteer: ship editor ("Build mode") is a distinct mode entered per ship; Blueprint sub-mode allows temporarily-illegal intermediate states (only the final result must validate) - a good enter/exit precedent: relaxed rules inside the edit context, validation at the exit boundary. Designs persist as `.ship.png` files (blueprint embedded in the image); the library stamps copies into the world; stamped ships are independent copies, and library edits do not touch placed ships.
  https://cosmoteer.wiki.gg/wiki/Ship_Editor
  https://steamcommunity.com/sharedfiles/filedetails/?id=2878136833
- KSP: the VAB edits one craft; the world (save file) holds vessels as flat PART lists. A .craft file is a flat sequence of PART blocks where each part id is `name_uid` and structure is encoded by reference: `link`/`attN`/`srfN` fields point at other parts by id; the tree is strict (no cycles without mods) and the root part anchors it. Symmetry is also by reference (`sym`). Detail caveat: the official wiki was unreachable (bot wall); field names are as community-documented in parsers such as KML.
  https://wiki.kerbalspaceprogram.com/wiki/Root_part
  https://github.com/my-th-os/KML (KML is GPL-3.0: UNUSABLE for code reuse, design reference only)
  https://forum.kerbalspaceprogram.com/topic/51800-any-doco-on-the-craft-file-format/
  Lesson: a FLAT record list plus by-id references encodes a vehicle tree fine. Stamping the same craft twice into a save regenerates uids - copy semantics, no link back to the .craft.
- StarMade: blueprint catalog; spawning fills a blueprint with blocks (or admin-loads it) and produces an independent entity. Editing the catalog entry never retro-edits spawned ships. A tracker bug ("Saving to blueprint is incomplete") shows the classic desync failure in the other direction: the stamped copy diverges and re-capturing it to a blueprint loses attached state.
  https://starmade.fandom.com/wiki/Ship_Catalog
  https://phab.starma.de/T2882
- Avorion: "Saved Designs" blueprints are applied onto a founded ship; copies with the same style/volume/seed are exact copies, otherwise independent. Fighter blueprints are a template-stamping factory loop. Same copy semantics.
  https://avorion.fandom.com/wiki/FAQ
  https://steamcommunity.com/app/445220/discussions/0/2264691750483597328/
- Wesnoth (GPL-2.0-or-later: UNUSABLE for code reuse, design reference only): the editor's scenario mode writes plain WML text; it covers unit placement and simple scenario data, and everything complex stays hand-written WML in the same file. Lesson for Nova: an editor that emits the SAME human-editable format users already hand-write keeps the two workflows compatible; the editor only needs to cover the common subset.
  https://wiki.wesnoth.org/BuildingScenarios
  https://wiki.wesnoth.org/EditorWML
- FTL (proprietary): ships are `<shipBlueprint>` entries in blueprints.xml referencing a separate layout file by name - prototype-by-reference in a fully flat, append-patchable text format; mods extend via `.append` files that add or replace entries by name. Prior art for "prototype id + flat overrides" and for name-keyed patching.
  https://www.toothycat.net/~hologram/FTL/ShipEditingTutorial.html
  https://subsetgames.com/forum/viewtopic.php?t=32568

Pattern across all shipped games: "edit the vehicle" is a separate mode/file with its own validation; "place the vehicle" stamps an independent copy keyed by a design id; NOBODY live-propagates design edits into already-placed instances at the world level. Live propagation exists only in engine editors (Godot/Unity), where the world file stores a reference, not a copy.

## 5. Lowering and flattening prior art

- Source engine func_instance is the canonical flatten-with-namespacing design. VBSP collapses all instances into the map at compile time; entity names inside each instance are rewritten by the instance's "Fix Up Name" - default scheme prefix: `FixupName-targetname` (postfix and none also selectable). Multiple placements of the same instance file get distinct fixup names, so cross-references inside each instance stay internally consistent after the collapse.
  https://developer.valvesoftware.com/wiki/Func_instance
  https://developer.valvesoftware.com/wiki/Working_with_instances
  Failure modes documented there: manual "Collapse instances" in the editor is not identical to the compile-time collapse and "may result in issues"; auto-generated names (`AutoInstance-#`) appear when the user did not pick a fixup name, producing unstable ids across re-collapses.
- Override application order: Unity applies base prefab, then each nesting layer's overrides outward, instance overrides last; the recorded failure is collapsing that stack (Apply All) or breaking a middle layer (stale fileID), which drops outer overrides silently (section 2 sources). Godot layers packed-scene defaults then instance overrides; its runtime pack bug (#67884) shows the cost of computing the diff wrongly: everything becomes an override and future source edits stop propagating - the exact lossy-round-trip trap for any lowering code.
- Propagate-edits-to-instances: Godot and Unity both answer YES, always, live - because their world files store references. All the shipped game blueprint systems answer NO - because their world stores copies. The dividing line is what the world file stores, not editor UX.
- Known failure modes to design against: (a) id collisions when one design is stamped twice into a flat namespace (solved by deterministic prefix fixup); (b) lossy round-trip - once lowered with overrides materialized, the flat file cannot distinguish "inherited" from "pinned", so re-importing a flat file as an instance is impossible (Godot #67884, Unity Unpack); (c) desync - if the world stores copies, a source edit after stamping silently forks the design (StarMade T2882); (d) unstable generated ids across re-saves breaking external references and diffs (AutoInstance-#).

## Recommendations for Nova

Ranked, most important first.

1. Editor document is the graph; the RON is compiler output. Copy Jackdaw's shape: a persistent editor document plus patch-based edit ops, with save = lower (World/graph -> flat ScenarioConfig with default-diffing). Do not try to reconstruct the graph from lowered RON; store the graph in an editor-side project file and treat the RON as regenerable output (like assets/base content gen already works).
2. Propagate-edits answer: propagate at re-lower time, not in the world file. Inside the editor, a placed ship node stays a live REFERENCE to its ship scene (Godot semantics): editing the ship scene updates every placed instance in the editor, and the next save re-lowers all of them fresh. The lowered RON itself has copy semantics and no link beyond the prototype id - which matches every shipped game in section 4 and the existing ScenarioConfig prototype-reference decision. Never propagate by patching previously lowered files.
3. Override model: property-level diffs only, applied last, stored on the instance node. Keep Godot/Unity's "instance override pins the field" with per-field revert. Do NOT implement structural overrides (add/remove children inside a placed instance, Godot Editable Children, Unity added/removed objects) in v0.12.0 - it is the documented complexity and data-loss source in both engines and it is exactly the part that cannot lower to a flat prefab record. If a user needs it, offer "Make Local" (detach to inline entities), explicitly one-way, like Godot/Unity.
4. Id namespacing: deterministic prefix fixup, Source-engine style. Every placed instance gets a required, user-visible, unique instance name; lowered ids inside it become `<instance_name>/<internal_id>` (or `.` separator; pick one and validate the separator out of user ids). Never auto-generate `AutoInstance-#`-style names: derive the default from the ship name plus a stable counter stored in the editor document, so re-lowering is byte-stable and diffs stay small.
5. Compute override diffs against the source scene at lowering, and test the diff. Godot #67884 is the cautionary tale: a wrong diff writes every field, breaks propagation, and is invisible until the source changes. Add a round-trip test: lower, edit source scene, re-lower, assert only overridden fields survived on instances.
6. Reference by stable ids, not tree paths or indices. Unity's fileID-based override targeting silently drops overrides on rename; KSP-style flat `name_uid` references survive reordering. Nova's flat format should key everything by id, and the editor should treat rename as an id-preserving operation.
7. Track BSN, do not adopt yet. The patch-based `Scene` model in Bevy 0.19 is philosophically identical to this plan; when `.bsn` asset files and write-back land upstream (Jackdaw work, bevy#23637), the editor-internal graph could migrate onto it. Keep Nova's graph model small and BSN-shaped (patches over prototypes) to keep that door open. Licence-safe: Bevy and Jackdaw are MIT OR Apache-2.0.
8. Enter/exit semantics: switch the edit root, relax validation inside, validate at the boundary. Copy Godot (edit context = source scene as root) plus Cosmoteer (blueprint mode allows illegal intermediate states; exiting validates). Lowering only ever runs on a scene that passed exit validation.
