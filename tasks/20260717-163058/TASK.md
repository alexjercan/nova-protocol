# Beat-sheet pass: apply the storytelling rhythm across campaign and ledger; write the convention into the dev wiki

- STATUS: OPEN
- PRIORITY: 36
- TAGS: spike,v0.7.0,scenario,content,docs

Goal: make the rhythm actual. Apply the storytelling convention across
shakedown_run, both broadside parts and the five ledger files using the
scenario clock + the three engine tasks' mechanics (comms queue, arrival
telegraphs, transition delays): announce -> breathe -> arrive -> fight ->
confirm -> breathe -> next; one story line per beat; every fight gets a
lead-in; checkpoint lines fire before their outcome beat or ride its
auto-advance. Write the beat-sheet convention into the dev wiki
(guide-author-scenario.md). Acceptance is checkable: no handler fires
more than one StoryMessage; every balance-audit spawn group trails a
warning beat. Depends on 163033/163042/163050 landing first. Spike:
tasks/20260717-155740/SPIKE.md.
