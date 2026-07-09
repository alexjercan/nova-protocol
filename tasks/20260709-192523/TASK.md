# Component-lock HUD: section markers, selection highlight, focus meter

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.4.0,hud,spike


Spike: docs/spikes/20260709-192358-component-lock-vats-lite.md

Consumer of the screen-indicator widget (no new substrate): small
entity-anchored markers on the locked ship's live sections in a distinct
color, visible only while focused; the fine-locked section gets a highlighted
variant; a focus meter fills while focusing (thin bar in the readout-column
style first - a radial ring needs image/shader tech the UI pass lacks).
Acquire/lock SFX cues ride the existing audio events where wired (the cue
half of superseded 20260708-165703). Depends on: 20260709-192522 (state to
render).
