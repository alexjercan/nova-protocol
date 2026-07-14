# Evaluate bevy_capture for in-engine video/GIF capture (Bevy 0.19 support?)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,example,screenshot,spike

Spike: tasks/20260714-081636/SPIKE.md

Goal: PNG stills are covered by Bevy's built-in screenshot API (see the showcase
example task 20260714-081706). If we later want video/GIF clips for devlogs or the
site (moving thruster plumes, a torpedo intercept, a section coming apart),
evaluate `bevy_capture` - it wraps the screenshot API and encodes frame series to
PNG sequence / MP4 / GIF / FFmpeg pipe. Confirm it supports Bevy 0.19 before
adding; if not, the fallback is feeding the `ScreenshotCaptured` image into the
`image`/`gif` crates by hand. Backlog: only pursue if a concrete video need
appears; stills are enough for the current web placeholders.

