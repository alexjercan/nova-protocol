# Salvage crate pickup polish: per-crate audio cue + Shakedown crate spacing

- STATUS: CLOSED
- PRIORITY: 18
- TAGS: v0.6.0, polish, audio, scenario

Goal: make salvage pickups feel distinct. Today a crate pickup
(`crates/nova_scenario/src/objects/salvage.rs:~61`) has visual feedback (highlight,
tumble, glow) but fires SILENTLY - the only sound is the shared objective chime when
the whole beat completes. Add a light per-crate pickup cue (a "ding"/"beep",
quieter than and separate from the objective chime).

Pairs with a Shakedown spacing tweak: the 3 tutorial crates sit ~29-37u apart with
an 8u pickup radius (`crates/nova_assets/src/scenario/shakedown.rs:~50-54,111`), so
a fast pass can sweep several at once and they read as one pickup. Space them so
each pickup registers distinctly, reinforced by the new cue. This is the specific
polish the user called out.
