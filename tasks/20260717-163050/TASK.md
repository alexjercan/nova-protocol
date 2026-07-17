# Outcome transition pacing: timed auto-advance behind the overlay + lint for the linger:false swallow trap

- STATUS: OPEN
- PRIORITY: 37
- TAGS: spike,v0.7.0,scenario,menu,lint

Goal (USER DIRECTIVE 2026-07-17: "add to the pacing by doing linger
false in some cases maybe with a time delay"): a middle gear between the
hard cut and the modal overlay. An authorable delay on the non-lingering
NextScenario switch - queue the chain, let the world keep playing (or
show the outcome banner non-blocking), advance automatically after N
seconds; plus an optional timed auto-advance on the modal overlay; plus
a content_lint WARN for the Outcome + linger:false same-handler swallow
trap (NovaEventWorld::clear's documented footgun). Mind pause semantics
(delays tick on the scenario clock's gate). Spike:
tasks/20260717-155740/SPIKE.md.
