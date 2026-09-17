---
name: nova-task
description: Track Nova work as a decision-backed delivery specification.
---

# Nova Task

Use this project wrapper around Tatr only when the user requests tracked work.
Keep one request and its follow-ups in one task.

## Create the specification

Use `tasks/<YYYYMMDD-HHMMSS>/TASK.md` with `OPEN` or `CLOSED` status. Give each
task one scheduling tag: the current release, or `backlog` at priority 0.

Record compact sections as needed:

- `User facts`: requested behavior, constraints, and explicit rejections.
- `Decisions`: approved interfaces, names, defaults, and error rules.
- `Agent findings`: code evidence and risks; never present these as user facts.
- `Delivery`: intended result, deletion list, blast radius, and work sequence.
- `Verification`: named behavior, failure, or invariant and the chosen proof.
- `Done when`: observable completion conditions.

Do not copy the conversation. Quote the user only when exact wording controls
the result. Do not turn an unapproved agent suggestion into a requirement.

## Use Tatr

```bash
tatr new "Title" -p 100 -t <release> -b details.md
printf 'Body from stdin.\n' | tatr new "Title" --body -
tatr ls --sort priority
tatr ls --filter ':status eq OPEN'
tatr edit <id> --status CLOSED
```

Use `-r ROOT` for another checkout. Valid statuses are `OPEN` and `CLOSED`.
Edit existing task bodies directly.

## Deliver against the task

- Keep research, approved decisions, proof, reviews, and retrospectives with it.
- Compare the final code and evidence to `Done when` before closing it.
- Record skipped checks and limits. Do not treat planned work as completed work.
- Do not cite completed tasks in production code or durable documentation.
