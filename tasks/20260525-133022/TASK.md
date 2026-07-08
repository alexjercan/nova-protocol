# HUD indicator when torpedo is fired

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.4.0,torpedo

Show target lock and torpedo state. Legacy #146.

Pulled into v0.4.0 (roadmap spike 20260708-161726): finishes the torpedo UX that
0.4.0 already polished. There is an existing TODO in
`crates/nova_gameplay/src/hud/torpedo_target.rs` to size the reticle to the target
and add range/lead info - fold that in here.
