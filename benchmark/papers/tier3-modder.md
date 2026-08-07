# Tier 3 - build a mod

Your working directory `/work` holds four wiki pages under `wiki/` and two
complete worked mods under `webmods/`. **There is no source code and no
repository** - mods are data, and the wiki plus the worked examples are exactly
what a real modder starts from.

Write your output to `/out`. Nothing else you write is kept.

## Style

Be terse. Fragments over sentences, bullets over prose. No preamble, no
restating the question, no summary of what you are about to do. Name the thing
and stop.

Length is not evidence. An answer that names the right path in one line scores
exactly what a page naming the same path scores. Padding a thin answer does not
move it: the grader scores what you located, never how much you wrote.

## Task

You are writing a mod for Nova Protocol, a 3D space shooter. Mods are RON data
files - no Rust, no engine changes.

Everything you need is in `/work`: four wiki pages under `wiki/`, and two
complete worked mods under `webmods/`.

Build a mod called **Salvage Run** in `/out/salvage-run/`:

1. A **bundle manifest** naming the mod, its author, a version, a description,
   and `base` as its only dependency.
2. One **content file** holding two things:
   - a **section prototype**: a variant of a base hull section with different
     stats and a different id, so it is a new section rather than an override
     of a base one;
   - one **scenario** that uses it.
3. The scenario must, at minimum:
   - spawn a player ship built from your new section plus base sections;
   - light the scene (a scenario that authors no light renders black);
   - spawn at least two non-player objects;
   - declare a **sensor area** and react to a ship entering it;
   - track at least one **scenario variable**, and gate at least one handler on
     its value with a **filter**;
   - show at least one **HUD objective** and complete it;
   - declare a win **outcome** and a lose outcome.

Keep it small. Roughly 150-250 lines of RON is the target - `gauntlet` is 1,122
lines and is far more than you need. Correctness beats scope.

Write a short `/out/salvage-run/README.md` saying what the mod does.

## GAPS.md

Also write `/out/GAPS.md`: every point where the wiki did not tell you
something you needed, what you had to guess, and how you resolved it. Be
specific - name the page and what was missing.

This file is as valuable as the mod. The mod is checked by a linter; `GAPS.md`
is the only record of what the documentation cost you.

## Output

Everything goes under `/out`:

- `/out/salvage-run/` - the mod itself
- `/out/GAPS.md` - the wiki gaps, as described above
- `/out/meta.json` - `{"tool_calls": <your count>, "confidence": "high|medium|low"}`
