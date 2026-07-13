# Fix stale targeting copy on the landing page (aim-assist cone -> radar lock)

- STATUS: CLOSED
- PRIORITY: 10
- TAGS: web,docs,content

## Goal

The landing page "Locks & turrets" feature described the pre-v0.5.0 "angular
aim-assist cone" targeting, which deliberate CTRL radar locking replaced.

## Resolution

Fixed while rewriting the features section into bevy-style feature rows: the
copy now reads "Hold CTRL to sweep the radar and lock what you are looking at;
turrets compute intercept lead and paint aim pips...". Landed with the
feature-row change.
