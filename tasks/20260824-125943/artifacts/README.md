# Concept mockups (disposable, not in game)

Two self-contained HTML pages for review of the progression research in
`../PROGRESSION-RESEARCH.md`. They use no external files. Colors copy the
`base/phosphor` theme palette. All counts, items and yields are illustrative.
Nothing here is an approved interface, schema, ID or policy.

Open a page in a browser. All controls are buttons, so Tab and Enter work.

## station-service.html

| Interaction | Decision it shows | What to look for |
| --- | --- | --- |
| State: docked, undocked, port lost/auto-undocked, record not loaded | Service ownership and error policy | Every state other than "docked, port OK" refuses and shows no default services. A broken dock port releases the joint and fires `OnUndocked`; this is **not** a docked-fault state. An interrupted transaction needs a rollback/refusal decision. |
| Repair policy A or B | Free ship-app Repair against a dock service | A: station repair is the same as ship-app repair. B: ship-app repair is gone, station repair spends stock, and it refuses at 0 stock. |
| Rearm (disabled) | Ammo rule | Idle batch refill leaves no rearm service to sell. |
| Preview fit | Live refit | Refused: no live section-swap runtime. |
| Take a hit on S2 | Review aid | Lets a reviewer repeat repairs until stock runs out. |

## field-salvage.html

| Interaction | Decision it shows | What to look for |
| --- | --- | --- |
| Chart node select | `P* A+ S*` scope | c(1,0) derelict-only and c(-1,0) with all rocks skipped both fail A+ after placement. Empty nodes stay empty under cluster scope. |
| Chart reveal alternative | Readability before travel | Type on the chart, or "unknown field" until scanner range. |
| Fly to range, select contact, claim | Salvage as a choice | A `metal` rock gives plate stock, the derelict gives a part, a `rock` rock and the planetoid give nothing. Yields are invented. |
| Claim again | Repeat-claim refusal | Refused, hold unchanged. |
| Retire and revisit, restart | Claim lifetime | None: retire allows a repeat claim (farm). Session: retire refuses, restart clears. Save: restart keeps the hold. |

## Proof made

The initial pages were checked with headless Chromium at 1280 and 420 px.
After the broken-port correction, the disposable browser harness in `/tmp`
passed all 28 click/state assertions (including auto-release and focus).
A 420 px screenshot was inspected, but headless Chromium may enforce a wider
CSS viewport than the screenshot width; actual mobile overflow and screen
reader behavior remain unverified. No game run backs any value on these pages.
