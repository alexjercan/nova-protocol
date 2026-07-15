# Allow changing skybox cubemap via action

- STATUS: CLOSED
- PRIORITY: 22
- TAGS: v0.6.0, modding, polish

Expose cubemap swap as a modding hook. Legacy #130.

Pulled into v0.6.0 (20260714) as small modding-surface polish that complements the
RON format: a new `EventActionConfig` variant to swap the skybox cubemap from a
scenario, so modders can change the sky mid-scenario. Rides on the same asset-path
-> Handle authoring layer (20260714-083326) the format uses for `cubemap`. Small;
sits in the sprint tail.
