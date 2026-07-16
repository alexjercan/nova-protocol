# Review: News section TOC sidebar + exhaustive post expansion

- TASK: 20260716-114245
- BRANCH: changelog-revamp

## Round 1

- VERDICT: APPROVE (one MINOR fabrication fixed before landing)

Independent out-of-context reviewer cross-checked every technical claim in all 6
expanded posts against CHANGELOG.md and the pre-merge devlog/release pages (git
91653f20), and audited the TOC infra, scroll-spy, CSS, ASCII, and completeness.

Confirmed clean:
- FABRICATION: no BLOCKER/MAJOR. Every named number, file path, API/event/action
  name, crate rev, and mechanism traces to a source. 0.6.0's added specifics
  (rev 4c81117, the cargo bench command, tasks/20260714-083331/modding-perf-report.md, the two
  deferred optimizations, 17-24% / 500-5000) all exact. 0.4.0's (550m, 2km->20km,
  max_torque 100->40, the five AI states, the example ranges) all supported. The
  heavily-expanded 0.1.0-0.3.0 (tiny CHANGELOG stubs) trace to the devlogs; the
  0.3.1 dependency block matches exactly.
- COMPLETENESS: H1 on all 6; video iframe only on 0.1.0/0.2.0; callout--breaking
  only on 0.3.0/0.6.0; Point releases only on 0.2.0/0.3.0/0.4.0/0.5.0; no post
  carries its own footer or CHANGELOG pointer (shell owns them).
- TOC INFRA: TOC link ids and in-body anchor ids come from the same
  markdown-it-anchor id, so they match by construction; empty-headings fallback
  to plain .prose works; heading text HTML-escaped; only real markdown h2/h3
  collected (code fence / video / callout HTML not picked up).
- SCROLL-SPY: no-ops on the index, guards empty/missing headings, no crashes.
- ASCII clean; CSS .news__body correctly overrides .prose in the grid.

Findings:

- [x] R1.1 (MINOR) web/src/news/0.5.0.md:126 - the per-damage-type flavor
  ("AP punches through hardened plating, EMP leans on systems, Explosive trades
  precision for spread") asserted per-type behavior no source backs; sources only
  name the four types + resistance tables. Fixed: reduced to name-only
  ("one of four types - Kinetic, AP, EMP, or Explosive - and each section carries
  its own resistance table"), dropping the invented behaviors.
